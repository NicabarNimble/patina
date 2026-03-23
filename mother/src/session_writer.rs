use anyhow::{bail, Result};

use crate::{ChildHealth, ChildRequest, ChildResponse, MotherChild, MotherHost};

/// Session writer child placeholder surfaced in Mother registry.
///
/// This provides health visibility and explicit action failures until
/// session-writer operations are fully routed through child action handlers.
pub struct SessionWriterChild;

impl Default for SessionWriterChild {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionWriterChild {
    pub fn new() -> Self {
        Self
    }
}

impl MotherChild for SessionWriterChild {
    fn name(&self) -> &str {
        "session-writer"
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
            other => bail!("session-writer: unsupported action '{}'", other),
        }
    }
}
