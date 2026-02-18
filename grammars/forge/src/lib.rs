//! Forge staging file extraction plugin for Patina pipeline.
//!
//! Reads staged forge JSON files (.forge-issue, .forge-pr) written by
//! `patina scrape forge` and returns ExtractedPayload-compatible JSON
//! with the `kind` tag for host routing.
//!
//! Pure compute: no tree-sitter, no API calls, no DB access.

use patina_sdk::pipeline::{parse_request, PipelinePlugin};
use patina_sdk::register_pipeline;

#[derive(Default)]
struct ForgeGrammar;

impl PipelinePlugin for ForgeGrammar {
    fn name(&self) -> String {
        "grammar-forge".into()
    }

    fn handle(&mut self, request: &str) -> Result<String, String> {
        let req = parse_request(request)?;

        if req.op != "parse" {
            return Err(format!("unsupported op: {}", req.op));
        }

        let source = req.payload["source"]
            .as_str()
            .ok_or("missing payload.source")?;

        // The language field carries the file extension: "forge-issue" or "forge-pr"
        let language = req.payload["language"]
            .as_str()
            .ok_or("missing payload.language")?;

        // Parse the staged JSON file content
        let mut payload: serde_json::Value =
            serde_json::from_str(source).map_err(|e| format!("invalid staged JSON: {}", e))?;

        // Add the kind tag based on file extension
        let kind = match language {
            "forge-issue" => "issue",
            "forge-pr" => "pull-request",
            other => return Err(format!("unknown forge extension: {}", other)),
        };

        // Inject the kind field into the JSON object
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("kind".to_string(), serde_json::Value::String(kind.to_string()));
        } else {
            return Err("staged JSON is not an object".to_string());
        }

        serde_json::to_string(&payload).map_err(|e| format!("serialize error: {}", e))
    }
}

register_pipeline!(ForgeGrammar);
