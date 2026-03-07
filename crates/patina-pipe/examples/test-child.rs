//! Minimal test child for pipe protocol integration testing.
//!
//! Implements Child trait with fake GitHub-like data. Used to verify
//! the end-to-end protocol: pipe/initialize -> pipe/fetch (with streaming
//! pipe/fact notifications) -> pipe/shutdown.
//!
//! Usage: cargo run -p patina-pipe --example test-child

use std::io::{BufRead, Write};

use patina_pipe::{
    run, Capabilities, Child, FetchParams, FetchResult, HealthStatus, InitializeParams, PipeError,
    PipeIo, Status,
};

struct TestChild {
    initialized: bool,
}

impl Child for TestChild {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            provider: "test-github".to_string(),
            data_types: vec!["issues".to_string(), "prs".to_string()],
            supports_incremental: true,
        }
    }

    fn initialize(&mut self, _params: &InitializeParams) -> Result<(), PipeError> {
        self.initialized = true;
        Ok(())
    }

    fn fetch(
        &mut self,
        _params: &FetchParams,
        io: &mut PipeIo<impl Write, impl BufRead>,
    ) -> Result<FetchResult, PipeError> {
        // Emit some fake issues
        io.emit(
            "github",
            "issue",
            &serde_json::json!({
                "number": 42,
                "title": "Auth flow broken",
                "state": "open",
                "url": "https://github.com/example/repo/issues/42"
            }),
        )?;

        io.emit(
            "github",
            "issue",
            &serde_json::json!({
                "number": 43,
                "title": "Add rate limiting",
                "state": "closed",
                "url": "https://github.com/example/repo/issues/43"
            }),
        )?;

        // Emit a fake PR
        io.emit(
            "github",
            "pr",
            &serde_json::json!({
                "number": 100,
                "title": "Fix auth flow",
                "state": "merged",
                "url": "https://github.com/example/repo/pull/100"
            }),
        )?;

        Ok(FetchResult {
            emitted: io.count(),
            cursor: Some("2026-03-07T00:00:00Z".to_string()),
        })
    }

    fn health(&self) -> Result<Status, PipeError> {
        Ok(Status {
            status: HealthStatus::Ok,
            latency_ms: Some(12),
            message: None,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(TestChild { initialized: false })
}
