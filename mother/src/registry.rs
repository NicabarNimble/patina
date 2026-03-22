//! Child registry — loads, iterates, and provides access to children.
//!
//! Immutable after setup: children are registered before the daemon
//! starts accepting connections. Individual children use per-child
//! RwLock for concurrent handle() vs exclusive tick().

use anyhow::Result;
use std::sync::{Arc, RwLock};

use crate::{
    ChildHealth, ChildRequest, ChildResponse, KnowledgeChild, KnowledgeRuntimeStore, MotherChild,
    MotherHost, RunStatus, Toy,
};

/// Registry of Mother's children.
pub struct ChildRegistry {
    knowledge_children: Vec<Arc<RwLock<Box<dyn KnowledgeChild>>>>,
    legacy_children: Vec<Arc<RwLock<Box<dyn MotherChild>>>>,
}

impl ChildRegistry {
    pub fn new() -> Self {
        Self {
            knowledge_children: vec![],
            legacy_children: vec![],
        }
    }

    /// Register a child. Call before load_all().
    /// Returns error if a child with the same name is already registered.
    pub fn register(&mut self, child: Box<dyn MotherChild>) -> Result<()> {
        self.register_legacy(child)
    }

    pub fn register_legacy(&mut self, child: Box<dyn MotherChild>) -> Result<()> {
        let name = child.name().to_string();
        if self.child_name_exists(&name) {
            anyhow::bail!("duplicate child name: {}", name);
        }
        self.legacy_children.push(Arc::new(RwLock::new(child)));
        Ok(())
    }

    pub fn register_knowledge(&mut self, child: Box<dyn KnowledgeChild>) -> Result<()> {
        let name = child.name().to_string();
        if self.child_name_exists(&name) {
            anyhow::bail!("duplicate child name: {}", name);
        }
        self.knowledge_children.push(Arc::new(RwLock::new(child)));
        Ok(())
    }

    fn child_name_exists(&self, name: &str) -> bool {
        self.knowledge_children
            .iter()
            .any(|child| child.read().unwrap_or_else(|e| e.into_inner()).name() == name)
            || self
                .legacy_children
                .iter()
                .any(|child| child.read().unwrap_or_else(|e| e.into_inner()).name() == name)
    }

    /// Load all children — calls on_load() for each in order.
    /// Fails fast if any child fails to load.
    pub fn load_all(&self, host: &dyn MotherHost) -> Result<()> {
        for entry in &self.legacy_children {
            let mut child = entry.write().unwrap_or_else(|e| e.into_inner());
            let name = child.name().to_string();
            host.log(&name, "loading legacy migration child");
            child.on_load(host)?;
            host.log(&name, "loaded legacy migration child");
        }
        for entry in &self.knowledge_children {
            let mut child = entry.write().unwrap_or_else(|e| e.into_inner());
            let name = child.name().to_string();
            host.log(&name, "loading");
            child.on_load(host)?;
            host.log(&name, "loaded");
        }
        Ok(())
    }

    pub fn tick_legacy_all(&self) -> Vec<Toy> {
        let mut toys = vec![];
        for entry in &self.legacy_children {
            if let Ok(mut child) = entry.write() {
                toys.extend(child.tick());
            }
        }
        toys
    }

    pub fn run_knowledge_cycles(
        &self,
        runtime: &KnowledgeRuntimeStore,
        lease_owner: &str,
    ) -> Result<()> {
        for entry in &self.knowledge_children {
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
        let mut statuses = Vec::new();
        for child in &self.knowledge_children {
            let child = match child.read() {
                Ok(child) => child,
                Err(_) => continue,
            };
            statuses.push((child.name().to_string(), child.health()));
        }
        for child in &self.legacy_children {
            let child = match child.read() {
                Ok(child) => child,
                Err(_) => continue,
            };
            statuses.push((child.name().to_string(), child.health()));
        }
        statuses
    }

    /// Route a request to a child by name.
    pub fn handle(&self, child_name: &str, request: &ChildRequest) -> Result<ChildResponse> {
        if let Some(child) = self
            .knowledge_children
            .iter()
            .find(|child| child.read().unwrap_or_else(|e| e.into_inner()).name() == child_name)
        {
            let child = child.read().unwrap_or_else(|e| e.into_inner());
            return child.handle(request);
        }

        if let Some(child) = self
            .legacy_children
            .iter()
            .find(|child| child.read().unwrap_or_else(|e| e.into_inner()).name() == child_name)
        {
            let child = child.read().unwrap_or_else(|e| e.into_inner());
            return child.handle(request);
        }

        Err(anyhow::anyhow!("unknown child: {}", child_name))
    }

    pub fn health(&self, child_name: &str) -> Result<ChildHealth> {
        if let Some(child) = self
            .knowledge_children
            .iter()
            .find(|child| child.read().unwrap_or_else(|e| e.into_inner()).name() == child_name)
        {
            let child = child.read().unwrap_or_else(|e| e.into_inner());
            return Ok(child.health());
        }

        if let Some(child) = self
            .legacy_children
            .iter()
            .find(|child| child.read().unwrap_or_else(|e| e.into_inner()).name() == child_name)
        {
            let child = child.read().unwrap_or_else(|e| e.into_inner());
            return Ok(child.health());
        }

        Err(anyhow::anyhow!("unknown child: {}", child_name))
    }

    pub fn legacy_len(&self) -> usize {
        self.legacy_children.len()
    }

    pub fn knowledge_len(&self) -> usize {
        self.knowledge_children.len()
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
        fn boxed(name: &str) -> Box<dyn MotherChild> {
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
        assert!(registry.register(StubChild::boxed("alpha")).is_ok());
        assert!(registry.register(StubChild::boxed("beta")).is_ok());
        assert_eq!(registry.knowledge_len() + registry.legacy_len(), 2);
    }

    #[test]
    fn register_duplicate_name_rejected() {
        let mut registry = ChildRegistry::new();
        assert!(registry.register(StubChild::boxed("alpha")).is_ok());
        let err = registry.register(StubChild::boxed("alpha")).unwrap_err();
        assert!(
            err.to_string().contains("duplicate child name: alpha"),
            "got: {}",
            err
        );
        assert_eq!(registry.knowledge_len() + registry.legacy_len(), 1);
    }
}
