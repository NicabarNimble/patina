//! Child registry — loads, iterates, and provides access to children.
//!
//! Immutable after setup: children are registered before the daemon
//! starts accepting connections. Individual children use per-child
//! RwLock for concurrent handle() vs exclusive tick().

use anyhow::Result;
use std::sync::{Arc, RwLock};

use patina::mother::{ChildHealth, ChildRequest, ChildResponse, MotherChild, MotherHost, Toy};

/// Registry of Mother's children.
pub struct ChildRegistry {
    children: Vec<Arc<RwLock<Box<dyn MotherChild>>>>,
}

impl ChildRegistry {
    pub fn new() -> Self {
        Self { children: vec![] }
    }

    /// Register a child. Call before load_all().
    /// Returns error if a child with the same name is already registered.
    pub fn register(&mut self, child: Box<dyn MotherChild>) -> Result<()> {
        let name = child.name().to_string();
        if self.children.iter().any(|c| c.read().unwrap().name() == name) {
            anyhow::bail!("duplicate child name: {}", name);
        }
        self.children.push(Arc::new(RwLock::new(child)));
        Ok(())
    }

    /// Load all children — calls on_load() for each in order.
    /// Fails fast if any child fails to load.
    pub fn load_all(&self, host: &dyn MotherHost) -> Result<()> {
        for entry in &self.children {
            let mut child = entry.write().unwrap();
            let name = child.name().to_string();
            host.log(&name, "loading");
            child.on_load(host)?;
            host.log(&name, "loaded");
        }
        Ok(())
    }

    /// Tick all children — heartbeat iteration.
    /// Returns toys requested by children.
    pub fn tick_all(&self) -> Vec<Toy> {
        let mut toys = vec![];
        for entry in &self.children {
            if let Ok(mut child) = entry.write() {
                toys.extend(child.tick());
            }
        }
        toys
    }

    /// Health check all children.
    pub fn health_all(&self) -> Vec<(String, ChildHealth)> {
        self.children
            .iter()
            .filter_map(|entry| {
                let child = entry.read().ok()?;
                Some((child.name().to_string(), child.health()))
            })
            .collect()
    }

    /// Route a request to a child by name.
    pub fn handle(&self, child_name: &str, request: &ChildRequest) -> Result<ChildResponse> {
        let entry = self
            .children
            .iter()
            .find(|c| c.read().unwrap().name() == child_name)
            .ok_or_else(|| anyhow::anyhow!("unknown child: {}", child_name))?;

        let child = entry.read().unwrap();
        child.handle(request)
    }

    /// Number of registered children.
    pub fn len(&self) -> usize {
        self.children.len()
    }
}
