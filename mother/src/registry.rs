//! Child registry — loads, iterates, and provides access to children.
//!
//! Immutable after setup: children are registered before the daemon
//! starts accepting connections. Individual children use per-child
//! RwLock for concurrent handle() vs exclusive tick().

use anyhow::Result;
use std::sync::{Arc, RwLock};

use crate::{
    ChildHealth, ChildRequest, ChildResponse, KnowledgeChild, KnowledgeRuntimeStore, MotherHost,
    RunStatus,
};

/// Registry of Mother's children.
pub struct ChildRegistry {
    children: Vec<Arc<RwLock<Box<dyn KnowledgeChild>>>>,
}

impl Default for ChildRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ChildRegistry {
    pub fn new() -> Self {
        Self { children: vec![] }
    }

    pub fn register_knowledge(&mut self, child: Box<dyn KnowledgeChild>) -> Result<()> {
        let name = child.name().to_string();
        if self.child_name_exists(&name) {
            anyhow::bail!("duplicate child name: {}", name);
        }
        self.children.push(Arc::new(RwLock::new(child)));
        Ok(())
    }

    fn child_name_exists(&self, name: &str) -> bool {
        self.children
            .iter()
            .any(|child| child.read().unwrap_or_else(|e| e.into_inner()).name() == name)
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

    pub fn run_knowledge_cycles(
        &self,
        runtime: &KnowledgeRuntimeStore,
        lease_owner: &str,
    ) -> Result<()> {
        for entry in &self.children {
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
        for child in &self.children {
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
            .children
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
            .children
            .iter()
            .find(|child| child.read().unwrap_or_else(|e| e.into_inner()).name() == child_name)
        {
            let child = child.read().unwrap_or_else(|e| e.into_inner());
            return Ok(child.health());
        }

        Err(anyhow::anyhow!("unknown child: {}", child_name))
    }

    pub fn knowledge_len(&self) -> usize {
        self.children.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal knowledge child for testing registry logic.
    struct StubChild {
        child_name: String,
    }

    impl StubChild {
        fn boxed(name: &str) -> Box<dyn KnowledgeChild> {
            Box::new(Self {
                child_name: name.to_string(),
            })
        }
    }

    impl KnowledgeChild for StubChild {
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
        assert!(registry
            .register_knowledge(StubChild::boxed("alpha"))
            .is_ok());
        assert!(registry
            .register_knowledge(StubChild::boxed("beta"))
            .is_ok());
        assert_eq!(registry.knowledge_len(), 2);
    }

    #[test]
    fn register_duplicate_name_rejected() {
        let mut registry = ChildRegistry::new();
        assert!(registry
            .register_knowledge(StubChild::boxed("alpha"))
            .is_ok());
        let err = registry
            .register_knowledge(StubChild::boxed("alpha"))
            .unwrap_err();
        assert!(
            err.to_string().contains("duplicate child name: alpha"),
            "got: {}",
            err
        );
        assert_eq!(registry.knowledge_len(), 1);
    }
}
