//! Minimal pipeline plugin for conformance testing.
//!
//! Handles "echo" op — returns payload unchanged.
//! Unknown ops return an error.

use patina_sdk::{parse_request, register_pipeline, PipelinePlugin};

#[derive(Default)]
struct EchoPipeline;

impl PipelinePlugin for EchoPipeline {
    fn name(&self) -> String {
        "echo-pipeline".into()
    }

    fn handle(&mut self, request: &str) -> Result<String, String> {
        let req = parse_request(request)?;

        // Validate version
        if req.version != "1" {
            return Err(format!("unsupported version: {}", req.version));
        }

        match req.op.as_str() {
            "echo" => {
                // Return payload unchanged
                Ok(req.payload.to_string())
            }
            other => Err(format!("unknown op: {}", other)),
        }
    }
}

register_pipeline!(EchoPipeline);
