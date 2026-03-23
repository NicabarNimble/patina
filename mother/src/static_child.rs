use anyhow::{bail, Result};

use crate::{ChildHealth, ChildRequest, ChildResponse, MotherChild, MotherHost};

/// Named healthy child marker for daemon status/manifest resolution.
pub struct StaticChild {
    name: &'static str,
}

impl StaticChild {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl MotherChild for StaticChild {
    fn name(&self) -> &str {
        self.name
    }

    fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
        Ok(())
    }

    fn health(&self) -> ChildHealth {
        ChildHealth::Healthy
    }

    fn handle(&self, request: &ChildRequest) -> Result<ChildResponse> {
        match request.action.as_str() {
            "health" => Ok(ChildResponse {
                payload: serde_json::json!({"status": "healthy"}),
            }),
            other => bail!("{}: unsupported action '{}'", self.name, other),
        }
    }
}
