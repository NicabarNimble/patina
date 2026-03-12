//! Child registry — loads, iterates, and provides access to children.
//!
//! Immutable after setup: children are registered before the daemon
//! starts accepting connections. Individual children use per-child
//! RwLock for concurrent handle() vs exclusive tick().

use anyhow::Result;
use std::sync::{Arc, RwLock};

use patina::mother::{
    ChildHealth, ChildRequest, ChildResponse, KnowledgeChild, KnowledgeRuntimeStore, MotherChild,
    MotherHost, RunStatus, Toy,
};

enum RegisteredChild {
    Legacy(Arc<RwLock<Box<dyn MotherChild>>>),
    Knowledge(Arc<RwLock<Box<dyn KnowledgeChild>>>),
}

/// Registry of Mother's children.
pub struct ChildRegistry {
    children: Vec<RegisteredChild>,
}

impl ChildRegistry {
    pub fn new() -> Self {
        Self { children: vec![] }
    }

    /// Register a child. Call before load_all().
    /// Returns error if a child with the same name is already registered.
    pub fn register(&mut self, child: Box<dyn MotherChild>) -> Result<()> {
        self.register_legacy(child)
    }

    pub fn register_legacy(&mut self, child: Box<dyn MotherChild>) -> Result<()> {
        let name = child.name().to_string();
        if self.children.iter().any(|c| match c {
            RegisteredChild::Legacy(child) => {
                child.read().unwrap_or_else(|e| e.into_inner()).name() == name
            }
            RegisteredChild::Knowledge(child) => {
                child.read().unwrap_or_else(|e| e.into_inner()).name() == name
            }
        }) {
            anyhow::bail!("duplicate child name: {}", name);
        }
        self.children
            .push(RegisteredChild::Legacy(Arc::new(RwLock::new(child))));
        Ok(())
    }

    pub fn register_knowledge(&mut self, child: Box<dyn KnowledgeChild>) -> Result<()> {
        let name = child.name().to_string();
        if self.children.iter().any(|c| match c {
            RegisteredChild::Legacy(child) => {
                child.read().unwrap_or_else(|e| e.into_inner()).name() == name
            }
            RegisteredChild::Knowledge(child) => {
                child.read().unwrap_or_else(|e| e.into_inner()).name() == name
            }
        }) {
            anyhow::bail!("duplicate child name: {}", name);
        }
        self.children
            .push(RegisteredChild::Knowledge(Arc::new(RwLock::new(child))));
        Ok(())
    }

    /// Load all children — calls on_load() for each in order.
    /// Fails fast if any child fails to load.
    pub fn load_all(&self, host: &dyn MotherHost) -> Result<()> {
        for entry in &self.children {
            match entry {
                RegisteredChild::Legacy(entry) => {
                    let mut child = entry.write().unwrap_or_else(|e| e.into_inner());
                    let name = child.name().to_string();
                    host.log(&name, "loading");
                    child.on_load(host)?;
                    host.log(&name, "loaded");
                }
                RegisteredChild::Knowledge(entry) => {
                    let mut child = entry.write().unwrap_or_else(|e| e.into_inner());
                    let name = child.name().to_string();
                    host.log(&name, "loading");
                    child.on_load(host)?;
                    host.log(&name, "loaded");
                }
            }
        }
        Ok(())
    }

    pub fn tick_legacy_all(&self) -> Vec<Toy> {
        let mut toys = vec![];
        for entry in &self.children {
            if let RegisteredChild::Legacy(entry) = entry {
                if let Ok(mut child) = entry.write() {
                    toys.extend(child.tick());
                }
            }
        }
        toys
    }

    pub fn run_knowledge_cycles(
        &self,
        runtime: &KnowledgeRuntimeStore,
        lease_owner: &str,
    ) -> Result<()> {
        for entry in &self.children {
            let RegisteredChild::Knowledge(entry) = entry else {
                continue;
            };
            let mut child = entry.write().unwrap_or_else(|e| e.into_inner());
            let plugin_name = child.name().to_string();
            let run_id = runtime.record_run_start(&plugin_name)?;
            let mut metrics = serde_json::Map::new();
            let result = (|| -> Result<()> {
                let drained = child.drain(64)?;
                metrics.insert(
                    "drained_events".into(),
                    serde_json::Value::from(drained.len() as u64),
                );

                let tick_intents = child.tick();
                metrics.insert(
                    "tick_intents".into(),
                    serde_json::Value::from(tick_intents.len() as u64),
                );
                for intent in tick_intents {
                    runtime.enqueue_task(&plugin_name, &intent)?;
                }

                let mut executed = 0_u64;
                while let Some(task) = runtime.lease_next_task(&plugin_name, lease_owner)? {
                    runtime.mark_task_running(&task.id)?;
                    let request = ChildRequest {
                        action: task.kind.as_str().to_string(),
                        payload: serde_json::from_str(&task.payload_json)
                            .unwrap_or(serde_json::Value::Null),
                    };
                    match child.handle(&request) {
                        Ok(_) => runtime.mark_task_succeeded(&task.id)?,
                        Err(error) => {
                            runtime.mark_task_failed(&task.id, task.attempts, &error.to_string())?
                        }
                    }
                    executed += 1;
                }
                metrics.insert("executed_tasks".into(), serde_json::Value::from(executed));
                Ok(())
            })();

            match result {
                Ok(()) => runtime.finish_run(
                    run_id,
                    RunStatus::Succeeded,
                    Some(&serde_json::to_string(&metrics)?),
                    None,
                )?,
                Err(error) => runtime.finish_run(
                    run_id,
                    RunStatus::Failed,
                    Some(&serde_json::to_string(&metrics)?),
                    Some(&error.to_string()),
                )?,
            }
        }
        Ok(())
    }

    /// Health check all children.
    pub fn health_all(&self) -> Vec<(String, ChildHealth)> {
        self.children
            .iter()
            .filter_map(|entry| match entry {
                RegisteredChild::Legacy(child) => {
                    let child = child.read().ok()?;
                    Some((child.name().to_string(), child.health()))
                }
                RegisteredChild::Knowledge(child) => {
                    let child = child.read().ok()?;
                    Some((child.name().to_string(), child.health()))
                }
            })
            .collect()
    }

    /// Route a request to a child by name.
    pub fn handle(&self, child_name: &str, request: &ChildRequest) -> Result<ChildResponse> {
        let entry = self
            .children
            .iter()
            .find(|c| match c {
                RegisteredChild::Legacy(child) => {
                    child.read().unwrap_or_else(|e| e.into_inner()).name() == child_name
                }
                RegisteredChild::Knowledge(child) => {
                    child.read().unwrap_or_else(|e| e.into_inner()).name() == child_name
                }
            })
            .ok_or_else(|| anyhow::anyhow!("unknown child: {}", child_name))?;

        match entry {
            RegisteredChild::Legacy(child) => {
                let child = child.read().unwrap_or_else(|e| e.into_inner());
                child.handle(request)
            }
            RegisteredChild::Knowledge(child) => {
                let child = child.read().unwrap_or_else(|e| e.into_inner());
                child.handle(request)
            }
        }
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
