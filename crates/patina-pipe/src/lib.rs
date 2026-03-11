//! Native transport binding for Patina pipe protocol.
//!
//! Provides the `Child` trait and `run()` entry point for native children —
//! normal Rust binaries that speak JSON-RPC 2.0 over stdio.

pub mod emitter;
pub mod harness;
#[cfg(feature = "http-proxy")]
pub mod http_proxy;
pub mod measure;
pub mod pipe_io;
pub mod protocol;
pub mod sandbox;

pub use emitter::FactEmitter;
pub use patina_pipe_types::*;
pub use pipe_io::PipeIo;

use std::io::{BufRead, BufReader, Write};

use protocol::{Request, Response};

// Pipe protocol error codes — PipeError variant -> JSON-RPC code mapping.
// These live in the transport layer, not in pipe-types.
const ERR_TRANSIENT: i32 = -32001;
const ERR_FATAL: i32 = -32002;
const ERR_RATE_LIMITED: i32 = -32003;
const ERR_PARTIAL: i32 = -32004;

/// Trait for native pipe protocol children.
///
/// Implement this and call `run()` to create a native child binary.
/// All methods use `&mut self` to allow mutable state (connection pools,
/// rate limit tracking, cached auth) across calls.
///
/// Connector children implement `fetch()`. Storage children (lakehouse)
/// implement `ingest()`. Both get the rest for free via defaults.
pub trait Child {
    /// Declare what this child can do. Called during pipe/initialize.
    fn capabilities(&self) -> Capabilities;

    /// Receive configuration from Mother. Called once during pipe/initialize.
    /// Default implementation does nothing — override if you need auth/config.
    fn initialize(&mut self, _params: &InitializeParams) -> Result<(), PipeError> {
        Ok(())
    }

    /// Fetch data from the external source and emit facts via the pipe I/O.
    ///
    /// Use `io.emit()` to stream facts and `io.get(url).send()` for HTTP
    /// calls proxied through Mother. Return `FetchResult` with the emitted
    /// count and optional cursor.
    ///
    /// Connector children implement this. Storage children leave the default.
    fn fetch(
        &mut self,
        _params: &FetchParams,
        _io: &mut PipeIo<impl Write, impl BufRead>,
    ) -> Result<FetchResult, PipeError> {
        Err(PipeError::Fatal {
            message: "pipe/fetch not implemented by this child".to_string(),
        })
    }

    /// Receive records from Mother for storage (pipe/ingest).
    ///
    /// Storage children (lakehouse) implement this. Connector children
    /// leave the default, which returns Fatal.
    /// Hard specification in [[raw-lake-ingestion]] DESIGN.md.
    fn ingest(&mut self, _params: &IngestParams) -> Result<IngestResult, PipeError> {
        Err(PipeError::Fatal {
            message: "pipe/ingest not implemented by this child".to_string(),
        })
    }

    /// Health check. Called by Mother to monitor child status.
    fn health(&self) -> Result<Status, PipeError> {
        Ok(Status {
            status: HealthStatus::Ok,
            latency_ms: None,
            message: None,
        })
    }
}

/// Map a PipeError to a JSON-RPC error Response.
fn pipe_error_to_response(id: Option<serde_json::Value>, err: &PipeError) -> Response {
    let (code, detail) = match err {
        PipeError::Transient {
            message,
            retry_after_ms,
        } => {
            let mut detail = serde_json::json!({"message": message});
            if let Some(retry) = retry_after_ms {
                detail["retry_after_ms"] = serde_json::json!(retry);
            }
            (ERR_TRANSIENT, detail)
        }
        PipeError::Fatal { message } => (ERR_FATAL, serde_json::json!({"message": message})),
        PipeError::RateLimited {
            message,
            retry_after_ms,
        } => (
            ERR_RATE_LIMITED,
            serde_json::json!({"message": message, "retry_after_ms": retry_after_ms}),
        ),
        PipeError::Partial { message, emitted } => (
            ERR_PARTIAL,
            serde_json::json!({"message": message, "emitted": emitted}),
        ),
    };

    Response::error(id, code, &detail.to_string())
}

/// Run a native child over stdio JSON-RPC.
///
/// Reads line-delimited JSON-RPC from stdin, dispatches to the Child
/// implementation, writes responses to stdout. Follows the same pattern
/// as `src/mcp/server/mod.rs`.
///
/// During pipe/fetch, the child has access to both stdout (for emitting
/// facts and pipe/http requests) and stdin (for reading pipe/http
/// responses from Mother) via PipeIo.
///
/// Every error path is handled explicitly — no panics on protocol messages.
pub fn run<C: Child>(mut child: C) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());

    let mut initialized = false;
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader
            .read_line(&mut line)
            .map_err(|e| format!("stdin read error: {}", e))?;
        if bytes_read == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: Request = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let resp = Response::error(None, -32700, &format!("Parse error: {}", e));
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
                continue;
            }
        };

        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            let resp = Response::error(
                request.id.clone(),
                -32600,
                &format!(
                    "Invalid JSON-RPC version: expected 2.0, got {}",
                    request.jsonrpc
                ),
            );
            writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
            stdout.flush()?;
            continue;
        }

        match request.method.as_str() {
            "pipe/initialize" => {
                let params: InitializeParams = match serde_json::from_value(request.params.clone())
                {
                    Ok(p) => p,
                    Err(e) => {
                        let resp = Response::error(
                            request.id.clone(),
                            -32602,
                            &format!("Invalid initialize params: {}", e),
                        );
                        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                        stdout.flush()?;
                        continue;
                    }
                };

                match child.initialize(&params) {
                    Ok(()) => {
                        initialized = true;
                        let caps = child.capabilities();
                        let result = serde_json::to_value(&caps).unwrap_or_default();
                        let resp = Response::success(request.id.clone(), result);
                        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                        stdout.flush()?;
                    }
                    Err(e) => {
                        let resp = pipe_error_to_response(request.id.clone(), &e);
                        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                        stdout.flush()?;
                    }
                }
            }

            "pipe/fetch" => {
                if !initialized {
                    let resp = Response::error(
                        request.id.clone(),
                        ERR_FATAL,
                        "pipe/fetch before pipe/initialize",
                    );
                    writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                    stdout.flush()?;
                    continue;
                }

                let params: FetchParams = match serde_json::from_value(request.params.clone()) {
                    Ok(p) => p,
                    Err(e) => {
                        let resp = Response::error(
                            request.id.clone(),
                            -32602,
                            &format!("Invalid fetch params: {}", e),
                        );
                        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                        stdout.flush()?;
                        continue;
                    }
                };

                // PipeIo provides both fact emission and pipe/http to the child.
                // stdout borrow released after fetch returns.
                let fetch_result = {
                    let mut io = PipeIo::new(&mut stdout, &mut reader);
                    child.fetch(&params, &mut io)
                };

                match fetch_result {
                    Ok(result) => {
                        let result_val = serde_json::to_value(&result).unwrap_or_default();
                        let resp = Response::success(request.id.clone(), result_val);
                        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                        stdout.flush()?;
                    }
                    Err(e) => {
                        let resp = pipe_error_to_response(request.id.clone(), &e);
                        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                        stdout.flush()?;
                    }
                }
            }

            "pipe/ingest" => {
                if !initialized {
                    let resp = Response::error(
                        request.id.clone(),
                        ERR_FATAL,
                        "pipe/ingest before pipe/initialize",
                    );
                    writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                    stdout.flush()?;
                    continue;
                }

                let params: IngestParams = match serde_json::from_value(request.params.clone()) {
                    Ok(p) => p,
                    Err(e) => {
                        let resp = Response::error(
                            request.id.clone(),
                            -32602,
                            &format!("Invalid ingest params: {}", e),
                        );
                        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                        stdout.flush()?;
                        continue;
                    }
                };

                match child.ingest(&params) {
                    Ok(result) => {
                        let result_val = serde_json::to_value(&result).unwrap_or_default();
                        let resp = Response::success(request.id.clone(), result_val);
                        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                        stdout.flush()?;
                    }
                    Err(e) => {
                        let resp = pipe_error_to_response(request.id.clone(), &e);
                        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                        stdout.flush()?;
                    }
                }
            }

            "pipe/health" => match child.health() {
                Ok(status) => {
                    let result = serde_json::to_value(&status).unwrap_or_default();
                    let resp = Response::success(request.id.clone(), result);
                    writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                    stdout.flush()?;
                }
                Err(e) => {
                    let resp = pipe_error_to_response(request.id.clone(), &e);
                    writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                    stdout.flush()?;
                }
            },

            "pipe/shutdown" => {
                let resp = Response::success(request.id.clone(), serde_json::json!({}));
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
                break;
            }

            _ => {
                let resp = Response::error(request.id.clone(), -32601, "Method not found");
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Test child that returns fixed data.
    struct TestChild {
        initialized: bool,
    }

    impl TestChild {
        fn new() -> Self {
            Self { initialized: false }
        }
    }

    impl Child for TestChild {
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                provider: "test".to_string(),
                data_types: vec!["items".to_string()],
                supports_incremental: false,
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
            io.emit("test", "item", &serde_json::json!({"id": 1}))?;
            io.emit("test", "item", &serde_json::json!({"id": 2}))?;
            Ok(FetchResult {
                emitted: io.count(),
                cursor: None,
            })
        }
    }

    /// Helper: run a child with given input lines and return all output lines.
    fn run_with_input(input: &str) -> Vec<String> {
        let stdin_data = input.as_bytes();
        let mut reader = BufReader::new(stdin_data);
        let mut output = Vec::new();

        let mut child = TestChild::new();
        let mut initialized = false;
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).unwrap();
            if bytes_read == 0 {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let request: Request = match serde_json::from_str(trimmed) {
                Ok(r) => r,
                Err(e) => {
                    let resp = Response::error(None, -32700, &format!("Parse error: {}", e));
                    output.push(serde_json::to_string(&resp).unwrap());
                    continue;
                }
            };

            if request.jsonrpc != "2.0" {
                let resp = Response::error(
                    request.id.clone(),
                    -32600,
                    &format!(
                        "Invalid JSON-RPC version: expected 2.0, got {}",
                        request.jsonrpc
                    ),
                );
                output.push(serde_json::to_string(&resp).unwrap());
                continue;
            }

            match request.method.as_str() {
                "pipe/initialize" => {
                    let params: InitializeParams =
                        serde_json::from_value(request.params.clone()).unwrap();
                    child.initialize(&params).unwrap();
                    initialized = true;
                    let caps = child.capabilities();
                    let result = serde_json::to_value(&caps).unwrap();
                    let resp = Response::success(request.id.clone(), result);
                    output.push(serde_json::to_string(&resp).unwrap());
                }
                "pipe/fetch" => {
                    if !initialized {
                        let resp = Response::error(
                            request.id.clone(),
                            ERR_FATAL,
                            "pipe/fetch before pipe/initialize",
                        );
                        output.push(serde_json::to_string(&resp).unwrap());
                        continue;
                    }
                    let params: FetchParams =
                        serde_json::from_value(request.params.clone()).unwrap();

                    // Use a Cursor for the writer and an empty Cursor for the reader
                    // (test child doesn't use pipe/http)
                    let mut cursor = Cursor::new(Vec::new());
                    let mut empty_reader = Cursor::new(Vec::<u8>::new());
                    let mut io = PipeIo::new(&mut cursor, &mut empty_reader);
                    let result = child.fetch(&params, &mut io).unwrap();

                    // Collect emitted notifications
                    let emitted_bytes = cursor.into_inner();
                    let emitted_str = String::from_utf8(emitted_bytes).unwrap();
                    for notif_line in emitted_str.trim().lines() {
                        output.push(notif_line.to_string());
                    }

                    let result_val = serde_json::to_value(&result).unwrap();
                    let resp = Response::success(request.id.clone(), result_val);
                    output.push(serde_json::to_string(&resp).unwrap());
                }
                "pipe/ingest" => {
                    if !initialized {
                        let resp = Response::error(
                            request.id.clone(),
                            ERR_FATAL,
                            "pipe/ingest before pipe/initialize",
                        );
                        output.push(serde_json::to_string(&resp).unwrap());
                        continue;
                    }
                    let params: IngestParams =
                        serde_json::from_value(request.params.clone()).unwrap();
                    match child.ingest(&params) {
                        Ok(result) => {
                            let result_val = serde_json::to_value(&result).unwrap();
                            let resp = Response::success(request.id.clone(), result_val);
                            output.push(serde_json::to_string(&resp).unwrap());
                        }
                        Err(e) => {
                            let resp = pipe_error_to_response(request.id.clone(), &e);
                            output.push(serde_json::to_string(&resp).unwrap());
                        }
                    }
                }
                "pipe/shutdown" => {
                    let resp = Response::success(request.id.clone(), serde_json::json!({}));
                    output.push(serde_json::to_string(&resp).unwrap());
                }
                _ => {
                    let resp = Response::error(request.id.clone(), -32601, "Method not found");
                    output.push(serde_json::to_string(&resp).unwrap());
                }
            }
        }

        output
    }

    #[test]
    fn initialize_returns_capabilities() {
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"pipe/initialize","params":{"protocol_version":"1.0"}}"#;
        let output = run_with_input(&format!("{}\n", input));
        assert_eq!(output.len(), 1);
        let resp: serde_json::Value = serde_json::from_str(&output[0]).unwrap();
        assert_eq!(resp["result"]["provider"], "test");
        assert_eq!(resp["result"]["data_types"], serde_json::json!(["items"]));
    }

    #[test]
    fn fetch_emits_facts_then_result() {
        let init = r#"{"jsonrpc":"2.0","id":1,"method":"pipe/initialize","params":{"protocol_version":"1.0"}}"#;
        let fetch = r#"{"jsonrpc":"2.0","id":2,"method":"pipe/fetch","params":{"types":["items"],"limit":100}}"#;
        let input = format!("{}\n{}\n", init, fetch);
        let output = run_with_input(&input);

        // init response + 2 fact notifications + fetch response = 4 lines
        assert_eq!(output.len(), 4);

        // Check notifications
        let notif1: serde_json::Value = serde_json::from_str(&output[1]).unwrap();
        assert_eq!(notif1["method"], "pipe/fact");
        assert!(notif1.get("id").is_none() || notif1["id"].is_null());

        // Check final response
        let resp: serde_json::Value = serde_json::from_str(&output[3]).unwrap();
        assert_eq!(resp["result"]["emitted"], 2);
    }

    #[test]
    fn fetch_before_initialize_errors() {
        let fetch = r#"{"jsonrpc":"2.0","id":1,"method":"pipe/fetch","params":{"types":["items"],"limit":100}}"#;
        let output = run_with_input(&format!("{}\n", fetch));
        assert_eq!(output.len(), 1);
        let resp: serde_json::Value = serde_json::from_str(&output[0]).unwrap();
        assert!(resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("before pipe/initialize"));
    }

    #[test]
    fn unknown_method_errors() {
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"pipe/unknown","params":{}}"#;
        let output = run_with_input(&format!("{}\n", input));
        let resp: serde_json::Value = serde_json::from_str(&output[0]).unwrap();
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[test]
    fn malformed_json_errors() {
        let input = "this is not json\n";
        let output = run_with_input(input);
        let resp: serde_json::Value = serde_json::from_str(&output[0]).unwrap();
        assert_eq!(resp["error"]["code"], -32700);
    }

    #[test]
    fn shutdown_returns_empty() {
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"pipe/shutdown"}"#;
        let output = run_with_input(&format!("{}\n", input));
        let resp: serde_json::Value = serde_json::from_str(&output[0]).unwrap();
        assert_eq!(resp["result"], serde_json::json!({}));
    }

    #[test]
    fn pipe_error_mapping() {
        let err = PipeError::RateLimited {
            message: "slow down".into(),
            retry_after_ms: 60000,
        };
        let resp = pipe_error_to_response(Some(serde_json::json!(1)), &err);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(&format!("\"code\":{}", ERR_RATE_LIMITED)));
        assert!(json.contains("slow down"));
        assert!(json.contains("60000"));
    }

    #[test]
    fn ingest_before_initialize_errors() {
        let ingest = r#"{"jsonrpc":"2.0","id":1,"method":"pipe/ingest","params":{"lake_path":"/tmp/lake","persona":"default","provider":"github","source_path":"org/repo","schema":"github","schema_version":"1.0.0","records":[]}}"#;
        let output = run_with_input(&format!("{}\n", ingest));
        assert_eq!(output.len(), 1);
        let resp: serde_json::Value = serde_json::from_str(&output[0]).unwrap();
        assert!(resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("before pipe/initialize"));
    }

    #[test]
    fn ingest_default_returns_fatal() {
        // TestChild is a connector — it doesn't implement ingest().
        // The default impl should return a Fatal error.
        let init = r#"{"jsonrpc":"2.0","id":1,"method":"pipe/initialize","params":{"protocol_version":"1.0"}}"#;
        let ingest = r#"{"jsonrpc":"2.0","id":2,"method":"pipe/ingest","params":{"lake_path":"/tmp/lake","persona":"default","provider":"github","source_path":"org/repo","schema":"github","schema_version":"1.0.0","records":[]}}"#;
        let input = format!("{}\n{}\n", init, ingest);
        let output = run_with_input(&input);

        // init response + ingest error response = 2 lines
        assert_eq!(output.len(), 2);

        let resp: serde_json::Value = serde_json::from_str(&output[1]).unwrap();
        assert_eq!(resp["error"]["code"], ERR_FATAL);
        assert!(resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not implemented"));
    }
}
