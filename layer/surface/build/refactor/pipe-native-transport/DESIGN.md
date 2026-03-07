# Design: Pipe Native Transport — Child Trait + stdio JSON-RPC

## Approach

New workspace member `crates/patina-pipe/` providing the `Child`
trait and `run()` entry point for native children. Follows the exact
same stdio JSON-RPC 2.0 pattern as `src/mcp/server/mod.rs` +
`src/mcp/protocol.rs`: BufReader on stdin, line-delimited JSON-RPC,
dispatch by method name, write responses to stdout.

The crate is the child-side library. Mother-side spawn logic lives
in the main binary (`src/broker/spawn.rs` — built by mother-broker
spec), but a minimal test harness is included here to verify the
protocol end-to-end.

## 1. Crate Structure

```
crates/patina-pipe/
  Cargo.toml
  src/
    lib.rs              # Child trait + run() + re-exports
    protocol.rs         # JSON-RPC 2.0 types (Request, Response, Notification)
    emitter.rs          # FactEmitter (streaming fact delivery)
    signing.rs          # content_hash + signature stub

  examples/
    test-child.rs       # Minimal child for integration testing
```

### 1.1 Cargo.toml

```toml
[package]
name = "patina-pipe"
version = "0.1.0"
edition = "2021"
description = "Native transport binding for Patina pipe protocol"

[dependencies]
patina-pipe-types = { path = "../patina-pipe-types" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[[example]]
name = "test-child"
```

No new deps beyond what patina-pipe-types already brings. The child
author adds their own deps (reqwest, etc.) in their binary crate.

Workspace Cargo.toml adds:
```toml
[workspace]
members = [..., "crates/patina-pipe-types", "crates/patina-pipe"]
```

## 2. Child Trait

The trait a native child implements. Three required methods, one
optional.

```rust
use patina_pipe_types::{
    Capabilities, FetchParams, FetchResult, HealthStatus, InitializeParams, PipeError,
};

use crate::emitter::FactEmitter;

/// Trait for native pipe protocol children.
///
/// Implement this trait and pass your struct to `run()`.
/// The transport layer handles JSON-RPC protocol, stdio I/O,
/// and fact streaming. You write domain logic only.
///
/// `&mut self` — allows mutable state across calls (connection
/// pools, rate limit tracking, cached auth). Audit fix from
/// session 12: `&self` prevents mutable state.
pub trait Child {
    /// Declare this child's capabilities.
    ///
    /// Called during pipe/initialize handshake. Mother uses this
    /// to validate fetch requests and schema declarations.
    fn capabilities(&self) -> Capabilities;

    /// Called after pipe/initialize with the config Mother sent.
    ///
    /// Store auth, params, or any config you need for fetch/health.
    /// Default impl does nothing (for children that don't need
    /// initialization beyond capabilities).
    fn initialize(&mut self, _params: &InitializeParams) -> Result<(), PipeError> {
        Ok(())
    }

    /// Fetch data from the external source and emit facts.
    ///
    /// Mother calls this with params specifying what to fetch.
    /// Use `emitter.emit()` to stream facts back — each fact is
    /// sent immediately as a pipe/fact notification, no buffering.
    ///
    /// Return FetchResult with count and optional cursor update.
    fn fetch(
        &mut self,
        params: &FetchParams,
        emitter: &mut FactEmitter,
    ) -> Result<FetchResult, PipeError>;

    /// Health check. Mother calls this to monitor child status.
    ///
    /// For connectors: check API reachability, rate limit status.
    /// Return quickly — this is called periodically.
    fn health(&self) -> Result<HealthStatus, PipeError>;
}
```

### 2.1 Why No Generics on Child

The trait uses concrete types (`FetchParams`, `FactEmitter`) not
generics. This means:
- `run()` works with `dyn Child` (object-safe)
- No monomorphization bloat in child binaries
- Trade-off: child can't use custom param types without parsing
  from `params.params` (the `serde_json::Value` field)

This matches how MCP tools work — the tool handler receives
`serde_json::Value` and parses what it needs.

## 3. JSON-RPC Protocol Types

Follow `src/mcp/protocol.rs` pattern exactly. Separate from MCP
types because pipe protocol has different methods and the crates
shouldn't depend on each other.

```rust
use serde::{Deserialize, Serialize};

/// Incoming JSON-RPC 2.0 request (from Mother on stdin).
#[derive(Debug, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Outgoing JSON-RPC 2.0 response (to Mother on stdout).
#[derive(Debug, Serialize)]
pub struct Response {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Outgoing JSON-RPC 2.0 notification (no id, no response expected).
/// Used for pipe/fact streaming during fetch.
#[derive(Debug, Serialize)]
pub struct Notification {
    pub jsonrpc: &'static str,
    pub method: String,
    pub params: serde_json::Value,
}

impl Response {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self { jsonrpc: "2.0", id, result: Some(result), error: None }
    }

    pub fn error(id: Option<serde_json::Value>, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0", id, result: None,
            error: Some(RpcError { code, message: message.to_string(), data: None }),
        }
    }

    pub fn pipe_error(id: Option<serde_json::Value>, err: &PipeError) -> Self {
        Self {
            jsonrpc: "2.0", id, result: None,
            error: Some(RpcError {
                code: err.jsonrpc_code(),
                message: err.to_string(),
                data: serde_json::to_value(err).ok(),
            }),
        }
    }
}

impl Notification {
    pub fn fact(fact: &patina_pipe_types::Fact) -> Self {
        Self {
            jsonrpc: "2.0",
            method: "pipe/fact".to_string(),
            params: serde_json::to_value(fact).unwrap_or_default(),
        }
    }

    pub fn progress(fetched: u64) -> Self {
        Self {
            jsonrpc: "2.0",
            method: "pipe/progress".to_string(),
            params: serde_json::json!({ "fetched": fetched }),
        }
    }
}
```

## 4. FactEmitter

Streaming fact delivery. Each `emit()` call writes a pipe/fact
notification to stdout immediately — no buffering, O(1) memory.

```rust
use std::io::Write;

use patina_pipe_types::{Fact, PipeError};
use patina_pipe_types::canonical::content_hash;

use crate::protocol::Notification;

/// Streaming fact emitter — writes pipe/fact notifications to stdout.
///
/// Created by `run()` for each pipe/fetch call. The child calls
/// `emitter.emit()` to send facts. Each fact is written immediately
/// as a JSON-RPC notification — no accumulation, O(1) memory.
///
/// Content hashing happens here — the child provides (schema,
/// fact_type, data) and the emitter computes the blake3 hash
/// over canonical JSON before sending.
pub struct FactEmitter<'a> {
    writer: &'a mut dyn Write,
    count: u64,
}

impl<'a> FactEmitter<'a> {
    pub(crate) fn new(writer: &'a mut dyn Write) -> Self {
        Self { writer, count: 0 }
    }

    /// Emit a fact. Computes content_hash, sends pipe/fact notification.
    ///
    /// This is the main API for child authors:
    /// ```ignore
    /// emitter.emit("github", "issue", &issue_json)?;
    /// ```
    pub fn emit(
        &mut self,
        schema: &str,
        fact_type: &str,
        data: &serde_json::Value,
    ) -> Result<(), PipeError> {
        let hash = content_hash(data);

        let fact = Fact {
            schema: schema.to_string(),
            fact_type: fact_type.to_string(),
            data: data.clone(),
            content_hash: hash,
            signature: String::new(), // stub until persona-federation
        };

        let notification = Notification::fact(&fact);
        let line = serde_json::to_string(&notification).map_err(|e| {
            PipeError::Fatal { message: format!("serialize fact: {}", e) }
        })?;

        writeln!(self.writer, "{}", line).map_err(|e| {
            PipeError::Fatal { message: format!("write to stdout: {}", e) }
        })?;
        self.writer.flush().map_err(|e| {
            PipeError::Fatal { message: format!("flush stdout: {}", e) }
        })?;

        self.count += 1;
        Ok(())
    }

    /// Number of facts emitted so far in this fetch.
    pub fn count(&self) -> u64 {
        self.count
    }
}
```

### 4.1 Why Clone data

`emit()` takes `&serde_json::Value` and clones into the Fact struct.
The alternative (taking ownership) would force children to build
values they can't reuse. Since facts are written to stdout immediately
and dropped, the clone lifetime is brief. For large payloads (>1MB),
this is a potential optimization point — but for GitHub issues (~1-5KB
each), negligible.

## 5. run() — The Entry Point

Follows `src/mcp/server/mod.rs` exactly: BufReader on stdin, read
lines, parse JSON-RPC, dispatch by method, write response to stdout.

Design constraint: **run() must handle all protocol errors
explicitly.** Every error path is enumerable, never panics.

```rust
use std::io::{self, BufRead, BufReader, Write};

use patina_pipe_types::{FetchParams, InitializeParams, PipeError};

use crate::protocol::{Request, Response};
use crate::emitter::FactEmitter;
use crate::Child;

/// Run a native pipe protocol child.
///
/// Call this from main(). It reads JSON-RPC requests from stdin,
/// dispatches to the Child trait methods, and writes responses
/// to stdout. stderr is available for child logging (eprintln!).
///
/// The function returns when:
/// - pipe/shutdown is received (graceful)
/// - stdin is closed (Mother exited or pipe broken)
/// - An unrecoverable I/O error occurs
///
/// Exit code: 0 on graceful shutdown, 1 on error.
pub fn run<C: Child>(mut child: C) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = BufReader::new(stdin.lock());

    // Track whether we've been initialized
    let mut initialized = false;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[pipe] stdin read error: {}", e);
                break;
            }
        };

        if line.is_empty() {
            continue;
        }

        // Parse JSON-RPC request
        let request: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = Response::error(None, -32700, &format!("Parse error: {}", e));
                write_response(&mut stdout, &resp)?;
                continue;
            }
        };

        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            let resp = Response::error(
                request.id.clone(),
                -32600,
                &format!("Invalid JSON-RPC version: expected 2.0, got {}", request.jsonrpc),
            );
            write_response(&mut stdout, &resp)?;
            continue;
        }

        // Dispatch by method
        match request.method.as_str() {
            "pipe/initialize" => {
                let params: InitializeParams = match serde_json::from_value(request.params.clone()) {
                    Ok(p) => p,
                    Err(e) => {
                        let resp = Response::error(
                            request.id.clone(), -32602,
                            &format!("Invalid initialize params: {}", e),
                        );
                        write_response(&mut stdout, &resp)?;
                        continue;
                    }
                };

                match child.initialize(&params) {
                    Ok(()) => {
                        initialized = true;
                        let caps = child.capabilities();
                        let resp = Response::success(
                            request.id.clone(),
                            serde_json::json!({
                                "protocol_version": "1.0",
                                "capabilities": caps,
                            }),
                        );
                        write_response(&mut stdout, &resp)?;
                    }
                    Err(e) => {
                        let resp = Response::pipe_error(request.id.clone(), &e);
                        write_response(&mut stdout, &resp)?;
                    }
                }
            }

            "pipe/fetch" => {
                if !initialized {
                    let resp = Response::error(
                        request.id.clone(), -32002,
                        "pipe/fetch before pipe/initialize",
                    );
                    write_response(&mut stdout, &resp)?;
                    continue;
                }

                let params: FetchParams = match serde_json::from_value(request.params.clone()) {
                    Ok(p) => p,
                    Err(e) => {
                        let resp = Response::error(
                            request.id.clone(), -32602,
                            &format!("Invalid fetch params: {}", e),
                        );
                        write_response(&mut stdout, &resp)?;
                        continue;
                    }
                };

                let mut emitter = FactEmitter::new(&mut stdout);
                match child.fetch(&params, &mut emitter) {
                    Ok(result) => {
                        let resp = Response::success(
                            request.id.clone(),
                            serde_json::to_value(&result).unwrap_or_default(),
                        );
                        write_response(&mut stdout, &resp)?;
                    }
                    Err(e) => {
                        let resp = Response::pipe_error(request.id.clone(), &e);
                        write_response(&mut stdout, &resp)?;
                    }
                }
            }

            "pipe/health" => {
                match child.health() {
                    Ok(status) => {
                        let resp = Response::success(
                            request.id.clone(),
                            serde_json::to_value(&status).unwrap_or_default(),
                        );
                        write_response(&mut stdout, &resp)?;
                    }
                    Err(e) => {
                        let resp = Response::pipe_error(request.id.clone(), &e);
                        write_response(&mut stdout, &resp)?;
                    }
                }
            }

            "pipe/shutdown" => {
                let resp = Response::success(
                    request.id.clone(),
                    serde_json::json!({}),
                );
                write_response(&mut stdout, &resp)?;
                eprintln!("[pipe] shutdown received, exiting");
                break;
            }

            _ => {
                let resp = Response::error(
                    request.id.clone(), -32601,
                    &format!("Method not found: {}", request.method),
                );
                write_response(&mut stdout, &resp)?;
            }
        }
    }

    Ok(())
}

/// Write a JSON-RPC response to stdout (one line, flushed).
fn write_response(stdout: &mut impl Write, resp: &Response) -> Result<(), Box<dyn std::error::Error>> {
    let line = serde_json::to_string(resp)?;
    writeln!(stdout, "{}", line)?;
    stdout.flush()?;
    Ok(())
}
```

### 5.1 FactEmitter and stdout Sharing

During pipe/fetch, both `run()` and `FactEmitter` need stdout.
The emitter borrows `&mut stdout` for the duration of the fetch
call. After fetch returns, `run()` reclaims stdout to write the
response.

This means notifications (pipe/fact) and the final response are
interleaved on the same stdout stream — exactly matching the
protocol diagram in DESIGN.md §1.3:

```
Mother → child:  pipe/fetch {since, types}
child → Mother:  pipe/fact {fact_1}          (notification, no id)
child → Mother:  pipe/fact {fact_2}          (notification, no id)
child → Mother:  {"jsonrpc":"2.0","id":1,"result":{"emitted":2}}  (response)
```

Mother reads lines and dispatches: lines with `"method"` and no
`"id"` are notifications, lines with `"id"` and `"result"`/`"error"`
are the fetch response.

### 5.2 Error Handling Summary

| Error | Detection | Response |
|-------|-----------|----------|
| Malformed JSON on stdin | `serde_json::from_str` fails | JSON-RPC -32700 Parse error |
| Wrong JSON-RPC version | `request.jsonrpc != "2.0"` | JSON-RPC -32600 Invalid request |
| Unknown method | `match` default arm | JSON-RPC -32601 Method not found |
| Invalid params | `serde_json::from_value` fails | JSON-RPC -32602 Invalid params |
| Fetch before initialize | `!initialized` flag | JSON-RPC -32002 Fatal |
| Child returns PipeError | `child.fetch()` returns Err | PipeError mapped to JSON-RPC code |
| stdin closed | `reader.lines()` ends | Loop exits, process exits 0 |
| stdout broken pipe | `writeln!` fails | Process exits with error |

No panics. Every path is handled.

## 6. OS Sandbox Profiles

Design constraint: **Sandbox must fail loud.** The sandbox profile
allows declared domains only. When a connection is blocked, the child
sees a connection error — the sandbox doesn't silently swallow packets.

### 6.1 macOS sandbox-exec Profile

```scheme
;; resources/sandbox/macos-child.sb
;;
;; macOS sandbox profile for native pipe protocol children.
;; Applied via: sandbox-exec -f macos-child.sb <child-binary>
;;
;; Deny everything, then allow:
;; - stdin/stdout/stderr (pipe protocol + logging)
;; - Network to declared domains (injected at spawn time)
;; - Basic system operations (process info, time)

(version 1)
(deny default)

;; Allow reading/writing stdio file descriptors
;; (inherited from parent process via fork+exec)
(allow file-read*  (literal "/dev/stdin"))
(allow file-write* (literal "/dev/stdout"))
(allow file-write* (literal "/dev/stderr"))
(allow file-read*  (literal "/dev/null"))
(allow file-read*  (literal "/dev/urandom"))

;; Allow basic process operations
(allow process-info-pidinfo)
(allow sysctl-read)

;; Network: outbound TCP to declared domains only.
;; TEMPLATE: Mother replaces {{ALLOWED_DOMAINS}} with regex
;; generated from child.toml [domains].allowed at spawn time.
;;
;; Example for github-connector:
;;   (allow network-outbound
;;     (remote tcp (require-all
;;       (regex #"^api\.github\.com$")
;;       (remote port 443))))
;;
;; When a domain is blocked, the child gets EPERM on connect(),
;; which surfaces as a connection error — NOT a silent timeout.
(allow network-outbound
  (remote tcp (require-all
    (regex {{ALLOWED_DOMAINS}})
    (remote port 443))))

;; Allow DNS resolution (needed to resolve allowed domains)
(allow network-outbound (remote udp (remote port 53)))
(allow network-outbound (remote tcp (remote port 53)))

;; Allow TLS system certs
(allow file-read*
  (subpath "/private/etc/ssl")
  (subpath "/System/Library/Security")
  (subpath "/Library/Keychains/System.keychain"))
```

### 6.2 Sandbox Template Processing

Mother generates the sandbox profile at spawn time:

```rust
/// Generate macOS sandbox profile from child manifest.
///
/// Reads the template from resources/sandbox/macos-child.sb,
/// replaces {{ALLOWED_DOMAINS}} with regex from child.toml.
fn generate_sandbox_profile(manifest: &ChildManifest) -> String {
    let template = include_str!("../../resources/sandbox/macos-child.sb");

    let domain_regex = if manifest.allowed_domains.is_empty() {
        // No domains allowed — deny all network
        r#"#"^$""#.to_string() // matches nothing
    } else {
        // Build regex alternation: ^(api\.github\.com|hooks\.slack\.com)$
        let escaped: Vec<String> = manifest.allowed_domains.iter()
            .map(|d| d.replace('.', r"\."))
            .collect();
        format!(r#"#"^({})$""#, escaped.join("|"))
    };

    template.replace("{{ALLOWED_DOMAINS}}", &domain_regex)
}
```

### 6.3 Linux Landlock Stub

```rust
/// Apply Linux Landlock sandbox.
///
/// Compiles on all platforms. On Linux with Landlock support,
/// restricts filesystem and network. On unsupported kernels or
/// non-Linux, logs a warning and continues unsandboxed.
fn apply_landlock_sandbox(manifest: &ChildManifest) {
    #[cfg(target_os = "linux")]
    {
        // Landlock ABI v4+ supports network restrictions
        // Implementation deferred — log warning for now
        eprintln!(
            "[sandbox] Landlock stub: would restrict {} to domains {:?}",
            manifest.name, manifest.allowed_domains
        );
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = manifest; // suppress unused warning
    }
}
```

### 6.4 Sandbox Debug Mode

When a child fails with a network error and sandbox is suspected:

```
$ patina mother run github --sandbox-debug
[sandbox] Profile for github-connector:
[sandbox]   Allowed domains: ["api.github.com"]
[sandbox]   Sandbox: macOS sandbox-exec
[sandbox] Starting child WITHOUT sandbox for debugging...
[sandbox] WARNING: Running unsandboxed — for debugging only
```

Mother-side flag: `--sandbox-debug` skips sandbox-exec and logs the
profile that would have been applied. Not for production.

## 7. Mother-Side Spawn Logic (Test Harness)

Minimal spawn logic for integration testing. Full lifecycle
management is mother-broker scope — this is just enough to verify
the protocol works.

```rust
/// Spawn a native child process and communicate over stdio.
///
/// This is the test harness version. Production spawn logic in
/// src/broker/spawn.rs (mother-broker spec) adds sandbox, credential
/// delivery, health monitoring, and routing.
fn spawn_child_test(
    binary_path: &Path,
    init_params: &InitializeParams,
) -> Result<ChildProcess> {
    use std::process::{Command, Stdio};

    let mut process = Command::new(binary_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit()) // child logs visible
        .spawn()?;

    let stdin = process.stdin.take().expect("stdin piped");
    let stdout = process.stdout.take().expect("stdout piped");

    let mut conn = ChildConnection {
        writer: BufWriter::new(stdin),
        reader: BufReader::new(stdout),
        next_id: 1,
    };

    // pipe/initialize handshake
    let caps_response = conn.call("pipe/initialize", init_params)?;
    let capabilities: Capabilities = serde_json::from_value(
        caps_response.get("capabilities").cloned().unwrap_or_default()
    )?;

    Ok(ChildProcess { process, conn, capabilities })
}

/// Connection to a child process (stdin writer + stdout reader).
struct ChildConnection {
    writer: BufWriter<ChildStdin>,
    reader: BufReader<ChildStdout>,
    next_id: u64,
}

impl ChildConnection {
    /// Send a JSON-RPC request and read the response.
    ///
    /// For pipe/fetch, also reads interleaved pipe/fact notifications
    /// and collects them. The response (with "id") signals completion.
    fn call(&mut self, method: &str, params: impl Serialize) -> Result<serde_json::Value> {
        let id = self.next_id;
        self.next_id += 1;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        writeln!(self.writer, "{}", serde_json::to_string(&request)?)?;
        self.writer.flush()?;

        // Read lines until we get a response with matching id
        loop {
            let mut line = String::new();
            self.reader.read_line(&mut line)?;

            let msg: serde_json::Value = serde_json::from_str(line.trim())?;

            // Notification (no id) — could be pipe/fact or pipe/progress
            if msg.get("id").is_none() {
                // For testing: log notifications
                if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
                    eprintln!("[test] notification: {}", method);
                }
                continue;
            }

            // Response with matching id
            if let Some(resp_id) = msg.get("id").and_then(|i| i.as_u64()) {
                if resp_id == id {
                    if let Some(error) = msg.get("error") {
                        return Err(anyhow::anyhow!("RPC error: {}", error));
                    }
                    return Ok(msg.get("result").cloned().unwrap_or_default());
                }
            }
        }
    }
}
```

## 8. Test Child Binary

`examples/test-child.rs` — implements Child trait with fake data.
Used for protocol integration testing without a real API.

```rust
//! Test child — responds to pipe protocol with fake data.
//! Usage: cargo run --example test-child

use patina_pipe::{Child, run, FactEmitter};
use patina_pipe_types::*;

struct TestConnector {
    initialized: bool,
}

impl Child for TestConnector {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            provider: "test".to_string(),
            data_types: vec!["items".to_string()],
            supports_incremental: true,
        }
    }

    fn initialize(&mut self, _params: &InitializeParams) -> Result<(), PipeError> {
        self.initialized = true;
        eprintln!("[test-child] initialized");
        Ok(())
    }

    fn fetch(
        &mut self,
        params: &FetchParams,
        emitter: &mut FactEmitter,
    ) -> Result<FetchResult, PipeError> {
        let count = params.limit.min(5).max(1);

        for i in 0..count {
            let data = serde_json::json!({
                "id": i,
                "title": format!("Test item {}", i),
                "status": "open",
            });
            emitter.emit("test", "item", &data)?;
        }

        Ok(FetchResult {
            emitted: emitter.count(),
            cursor: Some(format!("cursor-{}", count)),
        })
    }

    fn health(&self) -> Result<HealthStatus, PipeError> {
        Ok(HealthStatus {
            status: Status::Ok,
            message: Some("test child healthy".to_string()),
            latency_ms: Some(1),
        })
    }
}

fn main() {
    if let Err(e) = run(TestConnector { initialized: false }) {
        eprintln!("[test-child] fatal: {}", e);
        std::process::exit(1);
    }
}
```

## 9. Protocol Sequence Diagrams

### 9.1 Poll Mode (github-connector)

```
Mother                                     Child
  |                                          |
  |--- spawn child (fork+exec in sandbox) -->|
  |                                          |
  |--- pipe/initialize {auth, version} ----->|
  |<-- result {capabilities} ---------------|
  |                                          |
  |--- pipe/fetch {types, since, limit} ---->|
  |<-- pipe/fact {github, issue, data1} ----|  (notification)
  |<-- pipe/fact {github, issue, data2} ----|  (notification)
  |<-- pipe/fact {github, pr, data3} -------|  (notification)
  |<-- result {emitted: 3, cursor: "..."} --|
  |                                          |
  |--- pipe/shutdown ----------------------->|
  |<-- result {} ----------------------------|
  |                                          |
  |--- (child process exits) ---              |
```

### 9.2 Error During Fetch

```
Mother                                     Child
  |                                          |
  |--- pipe/fetch {types, since} ----------->|
  |<-- pipe/fact {github, issue, data1} ----|  (emitted before error)
  |<-- pipe/fact {github, issue, data2} ----|
  |<-- error {-32004, "API 500", emitted:2} |  (Partial error)
  |                                          |
  |--- (Mother keeps 2 facts, retries later)  |
```

### 9.3 Rate Limited

```
Mother                                     Child
  |                                          |
  |--- pipe/fetch {types, since} ----------->|
  |<-- error {-32003, "429", retry: 60000} -|
  |                                          |
  |--- (Mother waits 60s, then retries) ----  |
```

## Commits

1. `pipe: add patina-pipe crate with protocol types`
   — Create crate, Cargo.toml, protocol.rs (Request, Response,
   Notification). Follow MCP protocol.rs pattern.

2. `pipe: implement Child trait and FactEmitter`
   — lib.rs with Child trait, emitter.rs with streaming FactEmitter
   that writes pipe/fact notifications to stdout.

3. `pipe: implement run() dispatcher`
   — Full run() function: stdin reader, method dispatch, error
   handling for all protocol errors. Follows MCP server/mod.rs
   pattern.

4. `pipe: add test-child example`
   — examples/test-child.rs implementing Child trait with fake
   data. Verify: `cargo run --example test-child` starts and
   responds to piped JSON-RPC input.

5. `pipe: add macOS sandbox profile`
   — resources/sandbox/macos-child.sb template with domain
   placeholder. generate_sandbox_profile() function. Linux
   Landlock stub (compiles, logs warning).

6. `pipe: add Mother-side test harness`
   — spawn_child_test(), ChildConnection for integration testing.
   Verify: test harness can spawn test-child, initialize, fetch
   facts, and shut down.

## Key Files

- `crates/patina-pipe/src/lib.rs` — Child trait + run() entry point
- `crates/patina-pipe/src/protocol.rs` — JSON-RPC types
- `crates/patina-pipe/src/emitter.rs` — FactEmitter (streaming)
- `crates/patina-pipe/examples/test-child.rs` — protocol test binary
- `resources/sandbox/macos-child.sb` — OS sandbox profile template
- `src/mcp/server/mod.rs` — pattern to follow (stdio JSON-RPC)
- `src/mcp/protocol.rs` — pattern to follow (Request/Response types)

## Open Questions

1. **sandbox-exec deprecation.** Apple has deprecated sandbox-exec in
   recent macOS versions. It still works but may be removed. The
   alternative is the App Sandbox entitlement (requires code signing)
   or a manual seccomp-like approach. For now, sandbox-exec works and
   matches the Chrome renderer pattern mentioned in the architecture.
   Monitor deprecation status.

2. **Stdout contention during fetch.** FactEmitter borrows `&mut stdout`
   during fetch, which prevents `run()` from writing responses until
   fetch completes. This is correct for the protocol (response comes
   after all notifications), but means a child can't emit facts AND
   respond to health checks concurrently. This is fine for poll mode
   (single fetch, then exit). Stream mode will need a different
   approach — likely a channel-based writer. Defer to stream mode
   spec if needed.
