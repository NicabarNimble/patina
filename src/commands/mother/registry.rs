//! Child registry — loads, iterates, and provides access to children.
//!
//! Immutable after setup: children are registered before the daemon
//! starts accepting connections. Individual children use per-child
//! RwLock for concurrent handle() vs exclusive tick().

use anyhow::Result;
use std::sync::{Arc, RwLock};

use patina::mother::{ChildHealth, MotherChild, MotherHost, Toy};

/// Registry of Mother's children.
pub struct ChildRegistry {
    children: Vec<Arc<RwLock<Box<dyn MotherChild>>>>,
}

#[allow(dead_code)] // Methods used incrementally as children are added (AC 3+)
impl ChildRegistry {
    pub fn new() -> Self {
        Self { children: vec![] }
    }

    /// Register a child. Call before load_all().
    pub fn register(&mut self, child: Box<dyn MotherChild>) {
        self.children.push(Arc::new(RwLock::new(child)));
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

    /// Unload all children — calls on_unload() for each.
    pub fn unload_all(&self) {
        for entry in &self.children {
            if let Ok(mut child) = entry.write() {
                child.on_unload();
            }
        }
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

    /// Number of registered children.
    pub fn len(&self) -> usize {
        self.children.len()
    }
}
