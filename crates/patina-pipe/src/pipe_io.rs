//! PipeIo — unified context for native children during fetch.
//!
//! Combines fact emission (FactEmitter) with pipe/http proxied HTTP.
//! Children use `io.emit()` for facts and `io.get(url).send()` for HTTP.
//! All HTTP goes through Mother — the child never opens sockets directly.

use std::collections::HashMap;
use std::io::{BufRead, Write};

use patina_pipe_types::{Fact, PipeError, PipeHttpRequest, PipeHttpResponse};

use crate::protocol::Notification;

/// Unified I/O context for native children.
///
/// Passed to `Child::fetch()` — provides both fact emission and
/// pipe/http proxied HTTP. The writer goes to stdout (Mother reads),
/// the reader comes from stdin (Mother writes pipe/http responses).
pub struct PipeIo<'a, W: Write, R: BufRead> {
    writer: &'a mut W,
    reader: &'a mut R,
    fact_count: u64,
    next_http_id: u64,
}

impl<'a, W: Write, R: BufRead> PipeIo<'a, W, R> {
    pub fn new(writer: &'a mut W, reader: &'a mut R) -> Self {
        Self {
            writer,
            reader,
            fact_count: 0,
            // Start HTTP request IDs at 10000 to avoid collision with
            // Mother's request IDs (which start at 1).
            next_http_id: 10000,
        }
    }

    /// Emit a single fact as a pipe/fact notification.
    ///
    /// Computes canonical_json + content_hash automatically. Writes to
    /// stdout immediately — O(1) memory, no buffering.
    pub fn emit(
        &mut self,
        schema: &str,
        fact_type: &str,
        data: &serde_json::Value,
    ) -> Result<(), PipeError> {
        let hash = patina_pipe_types::content_hash(data);
        let fact = Fact {
            schema: schema.to_string(),
            fact_type: fact_type.to_string(),
            data: data.clone(),
            content_hash: hash,
            signature: String::new(),
        };

        let notification =
            Notification::fact(serde_json::to_value(&fact).map_err(|e| PipeError::Fatal {
                message: format!("serialize fact: {}", e),
            })?);

        let line = serde_json::to_string(&notification).map_err(|e| PipeError::Fatal {
            message: format!("serialize notification: {}", e),
        })?;

        writeln!(self.writer, "{}", line).map_err(|e| PipeError::Fatal {
            message: format!("write to stdout: {}", e),
        })?;

        self.writer.flush().map_err(|e| PipeError::Fatal {
            message: format!("flush stdout: {}", e),
        })?;

        self.fact_count += 1;
        Ok(())
    }

    /// Number of facts emitted so far.
    pub fn count(&self) -> u64 {
        self.fact_count
    }

    /// Start building an HTTP GET request.
    ///
    /// The request is sent to Mother via pipe/http. Mother validates the
    /// domain and executes the HTTP call.
    pub fn get(&mut self, url: &str) -> HttpRequestBuilder<'_, 'a, W, R> {
        HttpRequestBuilder::new(self, "GET", url)
    }

    /// Start building an HTTP POST request.
    pub fn post(&mut self, url: &str) -> HttpRequestBuilder<'_, 'a, W, R> {
        HttpRequestBuilder::new(self, "POST", url)
    }

    /// Start building an HTTP PUT request.
    pub fn put(&mut self, url: &str) -> HttpRequestBuilder<'_, 'a, W, R> {
        HttpRequestBuilder::new(self, "PUT", url)
    }

    /// Start building an HTTP PATCH request.
    pub fn patch(&mut self, url: &str) -> HttpRequestBuilder<'_, 'a, W, R> {
        HttpRequestBuilder::new(self, "PATCH", url)
    }

    /// Start building an HTTP DELETE request.
    pub fn delete(&mut self, url: &str) -> HttpRequestBuilder<'_, 'a, W, R> {
        HttpRequestBuilder::new(self, "DELETE", url)
    }

    /// Send a pipe/http request to Mother and read the response.
    fn send_http(&mut self, request: PipeHttpRequest) -> Result<PipeHttpResponse, PipeError> {
        let id = self.next_http_id;
        self.next_http_id += 1;

        // Write pipe/http JSON-RPC request to stdout
        let rpc_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "pipe/http",
            "params": serde_json::to_value(&request).map_err(|e| PipeError::Fatal {
                message: format!("serialize pipe/http request: {}", e),
            })?,
        });

        let line = serde_json::to_string(&rpc_request).map_err(|e| PipeError::Fatal {
            message: format!("serialize JSON-RPC: {}", e),
        })?;

        writeln!(self.writer, "{}", line).map_err(|e| PipeError::Fatal {
            message: format!("write pipe/http request: {}", e),
        })?;
        self.writer.flush().map_err(|e| PipeError::Fatal {
            message: format!("flush pipe/http request: {}", e),
        })?;

        // Read response from stdin — Mother writes it back
        let mut response_line = String::new();
        self.reader
            .read_line(&mut response_line)
            .map_err(|e| PipeError::Fatal {
                message: format!("read pipe/http response: {}", e),
            })?;

        let response_line = response_line.trim();
        if response_line.is_empty() {
            return Err(PipeError::Fatal {
                message: "empty response from Mother for pipe/http".to_string(),
            });
        }

        let parsed: serde_json::Value =
            serde_json::from_str(response_line).map_err(|e| PipeError::Fatal {
                message: format!("parse pipe/http response: {}", e),
            })?;

        // Check for error response
        if let Some(error) = parsed.get("error") {
            let msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown pipe/http error");
            return Err(PipeError::Fatal {
                message: format!("pipe/http denied: {}", msg),
            });
        }

        // Parse the result as PipeHttpResponse
        let result = parsed.get("result").ok_or_else(|| PipeError::Fatal {
            message: "pipe/http response missing 'result' field".to_string(),
        })?;

        serde_json::from_value::<PipeHttpResponse>(result.clone()).map_err(|e| PipeError::Fatal {
            message: format!("parse pipe/http result: {}", e),
        })
    }
}

/// Builder for pipe/http requests. Mirrors reqwest::RequestBuilder ergonomics.
pub struct HttpRequestBuilder<'a, 'b, W: Write, R: BufRead> {
    io: &'a mut PipeIo<'b, W, R>,
    request: PipeHttpRequest,
}

impl<'a, 'b, W: Write, R: BufRead> HttpRequestBuilder<'a, 'b, W, R> {
    fn new(io: &'a mut PipeIo<'b, W, R>, method: &str, url: &str) -> Self {
        Self {
            io,
            request: PipeHttpRequest {
                method: method.to_string(),
                url: url.to_string(),
                headers: HashMap::new(),
                body: None,
            },
        }
    }

    /// Add a header to the request.
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.request
            .headers
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Set the request body.
    pub fn body(mut self, body: String) -> Self {
        self.request.body = Some(body);
        self
    }

    /// Send the request to Mother and return the response.
    pub fn send(self) -> Result<PipeHttpResponse, PipeError> {
        self.io.send_http(self.request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn emit_writes_notification() {
        let mut writer = Vec::new();
        let mut reader = Cursor::new(Vec::<u8>::new());
        {
            let mut io = PipeIo::new(&mut writer, &mut reader);
            io.emit("github", "issue", &serde_json::json!({"title": "test"}))
                .unwrap();
            assert_eq!(io.count(), 1);
        }
        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains("\"method\":\"pipe/fact\""));
        assert!(output.contains("\"schema\":\"github\""));
        assert!(output.contains("\"content_hash\":\"blake3:"));
    }

    #[test]
    fn emit_multiple_increments_count() {
        let mut writer = Vec::new();
        let mut reader = Cursor::new(Vec::<u8>::new());
        {
            let mut io = PipeIo::new(&mut writer, &mut reader);
            io.emit("gh", "issue", &serde_json::json!({"n": 1}))
                .unwrap();
            io.emit("gh", "pr", &serde_json::json!({"n": 2})).unwrap();
            assert_eq!(io.count(), 2);
        }
        let output = String::from_utf8(writer).unwrap();
        let lines: Vec<&str> = output.trim().lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn send_http_writes_request_and_reads_response() {
        let mut writer = Vec::new();

        // Simulate Mother's response on stdin
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10000,
            "result": {
                "status": 200,
                "headers": {"content-type": "application/json"},
                "body": "[{\"id\":1}]"
            }
        });
        let response_line = format!("{}\n", serde_json::to_string(&response).unwrap());
        let mut reader = Cursor::new(response_line.into_bytes());

        {
            let mut io = PipeIo::new(&mut writer, &mut reader);
            let resp = io
                .get("https://api.example.com/data")
                .header("Accept", "application/json")
                .send()
                .unwrap();

            assert_eq!(resp.status, 200);
            assert_eq!(resp.body, "[{\"id\":1}]");
        }

        // Verify the request was written to stdout
        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains("\"method\":\"pipe/http\""));
        assert!(output.contains("\"id\":10000"));
        assert!(output.contains("api.example.com"));
    }

    #[test]
    fn send_http_error_response() {
        let mut writer = Vec::new();

        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10000,
            "error": {
                "code": -32003,
                "message": "domain 'evil.com' not in allowlist"
            }
        });
        let response_line = format!("{}\n", serde_json::to_string(&response).unwrap());
        let mut reader = Cursor::new(response_line.into_bytes());

        {
            let mut io = PipeIo::new(&mut writer, &mut reader);
            let err = io.get("https://evil.com/steal").send().unwrap_err();

            assert!(err.to_string().contains("evil.com"));
        }
    }
}
