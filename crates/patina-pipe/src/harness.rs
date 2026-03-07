//! Mother-side test harness for spawning and communicating with native children.
//!
//! Provides `spawn_child()` and `ChildConnection` for integration testing.
//! Not production lifecycle management — that's mother-broker scope.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

use patina_pipe_types::{PipeHttpRequest, PipeHttpResponse};

/// Handler for pipe/http requests from native children.
///
/// Mother provides this when spawning a child. The handler receives the
/// child's HTTP request, validates the domain against the manifest
/// allowlist, executes the HTTP call, and returns the response.
///
/// If no handler is provided, pipe/http requests are rejected with an error.
pub type HttpHandler = Box<dyn FnMut(&PipeHttpRequest) -> Result<PipeHttpResponse, String>>;

/// Connection to a spawned native child process.
///
/// Owns the child process and provides request/response communication
/// over stdio. Reads both responses (with `id`) and notifications
/// (pipe/fact, no `id`). Handles pipe/http requests from the child
/// by delegating to the provided HttpHandler.
pub struct ChildConnection {
    process: Child,
    stdin: ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    next_id: u64,
    http_handler: Option<HttpHandler>,
}

impl ChildConnection {
    /// Send a JSON-RPC request and read all output lines until the response
    /// (line with matching `id`) arrives. Returns (notifications, response).
    ///
    /// During reading, pipe/http requests from the child are handled
    /// transparently — the handler executes the HTTP call and writes
    /// the response back to the child's stdin.
    pub fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(Vec<serde_json::Value>, serde_json::Value), Box<dyn std::error::Error>> {
        let id = self.next_id;
        self.next_id += 1;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        writeln!(self.stdin, "{}", serde_json::to_string(&request)?)?;
        self.stdin.flush()?;

        let mut notifications = Vec::new();

        loop {
            let mut line = String::new();
            let bytes_read = self.reader.read_line(&mut line)?;
            if bytes_read == 0 {
                return Err("child process closed stdout".into());
            }

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parsed: serde_json::Value = serde_json::from_str(line)?;

            // Distinguish requests/notifications (have "method") from responses
            if let Some(method_name) = parsed.get("method").and_then(|m| m.as_str()) {
                match method_name {
                    "pipe/fact" => {
                        notifications.push(parsed);
                    }
                    "pipe/http" => {
                        self.handle_pipe_http(&parsed)?;
                    }
                    _ => {
                        // Unknown notification from child — ignore
                        eprintln!(
                            "[harness] unknown child notification: {}",
                            method_name
                        );
                    }
                }
                continue;
            }

            // Check if this is the response (has our id, no method field)
            if parsed.get("id") == Some(&serde_json::json!(id)) {
                return Ok((notifications, parsed));
            }

            // Unexpected message — log and continue
            eprintln!("[harness] unexpected message from child: {}", parsed);
        }
    }

    /// Handle a pipe/http request from the child.
    ///
    /// Delegates to the http_handler, writes the response (or error)
    /// back to the child's stdin as a JSON-RPC response.
    fn handle_pipe_http(
        &mut self,
        parsed: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let request_id = parsed.get("id").cloned();

        let http_request: PipeHttpRequest = match parsed
            .get("params")
            .map(|p| serde_json::from_value::<PipeHttpRequest>(p.clone()))
        {
            Some(Ok(req)) => req,
            Some(Err(e)) => {
                let resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "error": {
                        "code": -32602,
                        "message": format!("invalid pipe/http params: {}", e)
                    }
                });
                writeln!(self.stdin, "{}", serde_json::to_string(&resp)?)?;
                self.stdin.flush()?;
                return Ok(());
            }
            None => {
                let resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "error": {
                        "code": -32602,
                        "message": "missing params in pipe/http request"
                    }
                });
                writeln!(self.stdin, "{}", serde_json::to_string(&resp)?)?;
                self.stdin.flush()?;
                return Ok(());
            }
        };

        let result = match &mut self.http_handler {
            Some(handler) => handler(&http_request),
            None => Err("pipe/http not available: no handler configured".to_string()),
        };

        let resp = match result {
            Ok(http_response) => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": serde_json::to_value(&http_response).unwrap_or_default()
                })
            }
            Err(err_msg) => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "error": {
                        "code": -32003,
                        "message": err_msg
                    }
                })
            }
        };

        writeln!(self.stdin, "{}", serde_json::to_string(&resp)?)?;
        self.stdin.flush()?;
        Ok(())
    }

    /// Wait for the child process to exit and return its exit status.
    pub fn wait(mut self) -> Result<std::process::ExitStatus, std::io::Error> {
        drop(self.stdin);
        self.process.wait()
    }

    /// Kill the child process.
    pub fn kill(&mut self) -> Result<(), std::io::Error> {
        self.process.kill()
    }
}

/// Spawn a native child binary and return a connection for communication.
///
/// The child's stdin/stdout are connected for JSON-RPC. stderr is inherited
/// for child logging.
///
/// Note: This harness does NOT apply OS sandbox (sandbox_init/Landlock).
/// The spawn->sandbox->exec path is [[spec-mother-broker]] scope — Mother
/// applies the sandbox after fork, before exec. The sandbox primitives
/// are tested independently via fork-based tests in sandbox.rs.
pub fn spawn_child(binary_path: &str) -> Result<ChildConnection, Box<dyn std::error::Error>> {
    spawn_child_with_handler(binary_path, None)
}

/// Spawn a native child with an HTTP handler for pipe/http requests.
///
/// The handler receives pipe/http requests from the child, validates
/// domains, executes HTTP calls, and returns responses. Without a handler,
/// pipe/http requests are rejected with an error.
pub fn spawn_child_with_handler(
    binary_path: &str,
    http_handler: Option<HttpHandler>,
) -> Result<ChildConnection, Box<dyn std::error::Error>> {
    let mut process = Command::new(binary_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let stdin = process.stdin.take().ok_or("failed to open child stdin")?;
    let stdout = process.stdout.take().ok_or("failed to open child stdout")?;
    let reader = BufReader::new(stdout);

    Ok(ChildConnection {
        process,
        stdin,
        reader,
        next_id: 1,
        http_handler,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build the test-child example and return its absolute path.
    fn build_test_child() -> String {
        // Use --message-format=json to get the exact binary path from cargo
        let output = Command::new("cargo")
            .args([
                "build",
                "-p",
                "patina-pipe",
                "--example",
                "test-child",
                "--message-format=json",
            ])
            .output()
            .expect("failed to run cargo build");

        assert!(
            output.status.success(),
            "cargo build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Parse cargo's JSON output to find the executable path
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
                if msg["reason"] == "compiler-artifact"
                    && msg["target"]["name"] == "test-child"
                    && msg["target"]["kind"]
                        .as_array()
                        .map(|a| a.iter().any(|v| v == "example"))
                        .unwrap_or(false)
                {
                    if let Some(exe) = msg["executable"].as_str() {
                        return exe.to_string();
                    }
                }
            }
        }

        panic!("could not find test-child binary in cargo build output");
    }

    #[test]
    fn integration_spawn_initialize_fetch_shutdown() {
        let binary = build_test_child();
        let mut conn = spawn_child(&binary).expect("failed to spawn test-child");

        // pipe/initialize
        let (notifs, resp) = conn
            .request(
                "pipe/initialize",
                serde_json::json!({"protocol_version": "1.0"}),
            )
            .expect("initialize failed");
        assert!(notifs.is_empty());
        assert_eq!(resp["result"]["provider"], "test-github");
        assert_eq!(
            resp["result"]["data_types"],
            serde_json::json!(["issues", "prs"])
        );

        // pipe/fetch — should get 3 notifications then response
        let (notifs, resp) = conn
            .request(
                "pipe/fetch",
                serde_json::json!({"types": ["issues", "prs"], "limit": 100}),
            )
            .expect("fetch failed");
        assert_eq!(notifs.len(), 3, "expected 3 pipe/fact notifications");
        for notif in &notifs {
            assert_eq!(notif["method"], "pipe/fact");
            assert!(notif.get("id").is_none() || notif["id"].is_null());
            // Verify content hash is present
            let params = &notif["params"];
            assert!(params["content_hash"]
                .as_str()
                .unwrap()
                .starts_with("blake3:"));
        }
        assert_eq!(resp["result"]["emitted"], 3);
        assert_eq!(resp["result"]["cursor"], "2026-03-07T00:00:00Z");

        // pipe/health
        let (notifs, resp) = conn
            .request("pipe/health", serde_json::json!({}))
            .expect("health failed");
        assert!(notifs.is_empty());
        assert_eq!(resp["result"]["status"], "ok");

        // pipe/shutdown
        let (notifs, resp) = conn
            .request("pipe/shutdown", serde_json::json!({}))
            .expect("shutdown failed");
        assert!(notifs.is_empty());
        assert_eq!(resp["result"], serde_json::json!({}));

        // Child should exit cleanly
        let status = conn.wait().expect("wait failed");
        assert!(status.success());
    }
}
