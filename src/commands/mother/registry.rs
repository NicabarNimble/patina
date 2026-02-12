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
        if self
            .children
            .iter()
            .any(|c| c.read().unwrap_or_else(|e| e.into_inner()).name() == name)
        {
            anyhow::bail!("duplicate child name: {}", name);
        }
        self.children.push(Arc::new(RwLock::new(child)));
        Ok(())
    }

    /// Load all children — calls on_load() for each in order.
    /// Fails fast if any child fails to load.
    pub fn load_all(&self, host: &dyn MotherHost) -> Result<()> {
        for entry in &self.children {
            let mut child = entry.write().unwrap_or_else(|e| e.into_inner());
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
            .find(|c| c.read().unwrap_or_else(|e| e.into_inner()).name() == child_name)
            .ok_or_else(|| anyhow::anyhow!("unknown child: {}", child_name))?;

        let child = entry.read().unwrap_or_else(|e| e.into_inner());
        child.handle(request)
    }

    /// Number of registered children.
    pub fn len(&self) -> usize {
        self.children.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal MotherChild for testing registry logic.
    struct StubChild {
        child_name: String,
    }

    impl StubChild {
        fn new(name: &str) -> Box<dyn MotherChild> {
            Box::new(Self {
                child_name: name.to_string(),
            })
        }
    }

    impl MotherChild for StubChild {
        fn name(&self) -> &str {
            &self.child_name
        }
        fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
            Ok(())
        }
        fn health(&self) -> ChildHealth {
            ChildHealth::Healthy
        }
        fn handle(&self, _request: &ChildRequest) -> Result<ChildResponse> {
            Ok(ChildResponse {
                payload: serde_json::json!({"stub": true}),
            })
        }
    }

    #[test]
    fn register_unique_names() {
        let mut registry = ChildRegistry::new();
        assert!(registry.register(StubChild::new("alpha")).is_ok());
        assert!(registry.register(StubChild::new("beta")).is_ok());
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn register_duplicate_name_rejected() {
        let mut registry = ChildRegistry::new();
        assert!(registry.register(StubChild::new("alpha")).is_ok());
        let err = registry.register(StubChild::new("alpha")).unwrap_err();
        assert!(
            err.to_string().contains("duplicate child name: alpha"),
            "got: {}",
            err
        );
        assert_eq!(registry.len(), 1);
    }
}
