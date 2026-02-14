//! Pipeline plugin that deliberately panics in handle().
//!
//! Used by the cross-world WASM trap handling conformance test.
//! The host MUST catch the wasmtime trap and return a clean error —
//! never crash, never unwrap() across the WASM boundary.

use patina_pipeline_api::{register_pipeline, PipelinePlugin};

#[derive(Default)]
struct PanicPipeline;

impl PipelinePlugin for PanicPipeline {
    fn name(&self) -> String {
        "panic-pipeline".into()
    }

    fn handle(&mut self, _request: &str) -> Result<String, String> {
        panic!("deliberate panic for trap handling conformance test");
    }
}

register_pipeline!(PanicPipeline);
