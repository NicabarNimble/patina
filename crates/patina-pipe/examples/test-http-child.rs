//! Test child that uses pipe/http to fetch data through Mother.
//!
//! Used to verify the pipe/http protocol end-to-end: child sends HTTP
//! requests via pipe/http, Mother handles domain enforcement, and
//! responses flow back through the pipe.
//!
//! Usage: cargo run -p patina-pipe --example test-http-child

use std::io::{BufRead, Write};

use patina_pipe::{
    run, Capabilities, Child, FetchParams, FetchResult, HealthStatus, InitializeParams, PipeError,
    PipeIo, Status,
};

struct TestHttpChild;

impl Child for TestHttpChild {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            provider: "test-http".to_string(),
            data_types: vec!["items".to_string()],
            supports_incremental: false,
        }
    }

    fn fetch(
        &mut self,
        _params: &FetchParams,
        io: &mut PipeIo<impl Write, impl BufRead>,
    ) -> Result<FetchResult, PipeError> {
        // Make an HTTP GET request through Mother
        let resp = io
            .get("https://api.allowed.com/data")
            .header("Accept", "application/json")
            .send()?;

        // Emit the response as a fact
        io.emit(
            "test",
            "http_result",
            &serde_json::json!({
                "status": resp.status,
                "body_length": resp.body.len(),
            }),
        )?;

        // Try a request to a domain that should be denied
        match io.get("https://evil.com/steal").send() {
            Ok(_) => {
                // Should not reach here
                io.emit(
                    "test",
                    "security_violation",
                    &serde_json::json!({"error": "evil.com was not blocked"}),
                )?;
            }
            Err(_) => {
                // Expected — domain denied
                io.emit(
                    "test",
                    "domain_denied",
                    &serde_json::json!({"domain": "evil.com", "blocked": true}),
                )?;
            }
        }

        Ok(FetchResult {
            emitted: io.count(),
            cursor: None,
        })
    }

    fn health(&self) -> Result<Status, PipeError> {
        Ok(Status {
            status: HealthStatus::Ok,
            latency_ms: None,
            message: None,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(TestHttpChild)
}
