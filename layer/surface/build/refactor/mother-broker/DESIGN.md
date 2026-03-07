# Design: Mother Broker — Routing Engine + Child Lifecycle

## Approach

New `src/broker/` module in the main binary. Mother gains broker
responsibilities: read sources.toml, spawn children (WASM or native),
route emitted facts to destination events.db, validate schemas.

This builds on top of existing Mother infrastructure:
- `src/mother/` — project registry, graph, client
- `src/mother/child.rs` — MotherChild trait (WASM children)
- `src/commands/mother/` — CLI commands, daemon
- `src/plugin/internal/host_support.rs` — emit validation (extract)

The broker does not replace any of this. It adds a routing layer that
uses existing pieces and extends Mother's CLI with `run` and `sources`.

## 1. Module Structure

```
src/
  broker/
    mod.rs              # public API: run_source, status
    sources.rs          # sources.toml reader
    lifecycle.rs        # unified child lifecycle (WASM + native)
    spawn.rs            # native child spawn (fork+exec, sandbox)
    routing.rs          # fact routing + schema validation
    cursor.rs           # cursor management (transactional)
```

## 2. sources.toml Format Specification

Per-project file at `.patina/sources.toml`. Declares what external
data this project wants and how to get it.

```toml
# .patina/sources.toml

[sources.github]
connection = "github"                    # which connection (from ~/.patina/connections/)
params = { owner = "NicabarNimble", repo = "patina" }
types = ["issues", "prs"]               # which data types to fetch
schedule = "on-scrape"                   # when to fetch

[sources.github-docs]
connection = "github"                    # same connection, different repo
params = { owner = "NicabarNimble", repo = "docs" }
types = ["issues"]
schedule = "daily"
```

### 2.1 Source Entry Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `connection` | string | yes | Name of connection in `~/.patina/connections/` |
| `params` | table | no | Provider-specific params (passed to FetchParams.params) |
| `types` | array | no | Data types to fetch (default: all child capabilities) |
| `schedule` | string | no | When to run: "on-scrape", "hourly", "daily", "manual" (default: "manual") |

### 2.2 sources.rs — Reader

```rust
// src/broker/sources.rs

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A source declaration from sources.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct SourceEntry {
    pub connection: String,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default = "default_schedule")]
    pub schedule: String,
}

fn default_schedule() -> String { "manual".to_string() }

/// All sources declared in a project's sources.toml.
#[derive(Debug, Clone)]
pub struct ProjectSources {
    pub project_root: PathBuf,
    pub sources: HashMap<String, SourceEntry>,
}

impl ProjectSources {
    /// Load sources.toml from a project directory.
    /// Returns empty sources if file doesn't exist.
    pub fn load(project_root: &Path) -> Result<Self> {
        let path = project_root.join(".patina/sources.toml");

        if !path.exists() {
            return Ok(Self {
                project_root: project_root.to_path_buf(),
                sources: HashMap::new(),
            });
        }

        let content = std::fs::read_to_string(&path)?;
        let table: toml::Table = content.parse()?;

        let sources_table = table.get("sources")
            .and_then(|v| v.as_table())
            .cloned()
            .unwrap_or_default();

        let mut sources = HashMap::new();
        for (name, value) in sources_table {
            let entry: SourceEntry = value.try_into()
                .map_err(|e| anyhow::anyhow!("invalid source '{}': {}", name, e))?;
            sources.insert(name, entry);
        }

        Ok(Self {
            project_root: project_root.to_path_buf(),
            sources,
        })
    }
}

/// Scan all registered projects for sources.toml files.
///
/// Returns a flat list of (project_root, source_name, entry) tuples.
pub fn scan_all_sources() -> Result<Vec<(PathBuf, String, SourceEntry)>> {
    let graph = patina::mother::Graph::open()?;
    let projects = graph.list_nodes()?;

    let mut all_sources = Vec::new();
    for project in projects {
        let project_sources = ProjectSources::load(Path::new(&project.path))?;
        for (name, entry) in project_sources.sources {
            all_sources.push((project_sources.project_root.clone(), name, entry));
        }
    }

    Ok(all_sources)
}
```

## 3. Unified Child Lifecycle

### 3.1 BrokerChild Trait

A unified interface over WASM and native children for the broker's
routing engine. This is NOT a replacement for MotherChild — it's a
broker-specific adapter that wraps both runtimes.

```rust
// src/broker/lifecycle.rs

use anyhow::Result;
use patina_pipe_types::*;

/// A spawned child that the broker can communicate with.
///
/// Abstracts over WASM children (in-process, via MotherChild trait)
/// and native children (subprocess, via stdio pipe protocol).
pub trait BrokerChild {
    /// Child's name (from manifest).
    fn name(&self) -> &str;

    /// Send pipe/fetch and receive facts.
    ///
    /// The callback `on_fact` is called for each fact received.
    /// This allows the routing engine to process facts as they
    /// arrive without buffering.
    fn fetch(
        &mut self,
        params: &FetchParams,
        on_fact: &mut dyn FnMut(Fact) -> Result<()>,
    ) -> Result<FetchResult>;

    /// Health check.
    fn health(&self) -> Result<HealthStatus>;

    /// Graceful shutdown.
    fn shutdown(&mut self) -> Result<()>;
}
```

### 3.2 NativeChild Implementation

```rust
/// Native child spawned as a subprocess.
pub struct NativeChild {
    name: String,
    conn: ChildConnection,
    process: std::process::Child,
}

impl BrokerChild for NativeChild {
    fn name(&self) -> &str { &self.name }

    fn fetch(
        &mut self,
        params: &FetchParams,
        on_fact: &mut dyn FnMut(Fact) -> Result<()>,
    ) -> Result<FetchResult> {
        // Send pipe/fetch request
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.conn.next_id(),
            "method": "pipe/fetch",
            "params": params,
        });
        self.conn.write_request(&request)?;

        // Read interleaved notifications and response
        loop {
            let msg = self.conn.read_message()?;

            // Notification: pipe/fact
            if msg.get("method").and_then(|m| m.as_str()) == Some("pipe/fact") {
                if let Some(params) = msg.get("params") {
                    let fact: Fact = serde_json::from_value(params.clone())?;
                    on_fact(fact)?;
                }
                continue;
            }

            // Skip other notifications (pipe/progress)
            if msg.get("method").is_some() { continue; }

            // Response with result or error
            if msg.get("id").is_some() {
                if let Some(error) = msg.get("error") {
                    let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-32002) as i32;
                    let message = error.get("message").and_then(|m| m.as_str())
                        .unwrap_or("unknown").to_string();
                    let data = error.get("data").cloned();
                    return Err(PipeError::from_jsonrpc(code, message, data).into());
                }
                let result: FetchResult = serde_json::from_value(
                    msg.get("result").cloned().unwrap_or_default()
                )?;
                return Ok(result);
            }
        }
    }

    fn health(&self) -> Result<HealthStatus> {
        // TODO: send pipe/health, read response
        Ok(HealthStatus { status: Status::Ok, message: None, latency_ms: None })
    }

    fn shutdown(&mut self) -> Result<()> {
        // Send pipe/shutdown, wait for response, then wait for process exit
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.conn.next_id(),
            "method": "pipe/shutdown",
            "params": {},
        });
        self.conn.write_request(&request)?;
        let _ = self.conn.read_response(); // best-effort
        let _ = self.process.wait(); // reap child
        Ok(())
    }
}
```

### 3.3 WasmBrokerChild Adapter

```rust
/// WASM child wrapped for broker interface.
///
/// Adapts existing MotherChild (WASM plugin) to BrokerChild.
/// WASM children use handle("sync", payload) internally, and facts
/// are emitted via host_emit directly to events.db (the existing
/// host_support path). The broker doesn't intercept WASM emissions.
///
/// For WASM children, the broker handles lifecycle and scheduling.
/// Fact routing goes through the existing host_support::emit_fact
/// which writes directly to events.db. Content hashing and dedup
/// are NOT applied to WASM facts yet.
pub struct WasmBrokerChild {
    name: String,
    inner: Box<dyn crate::mother::MotherChild>,
}

impl BrokerChild for WasmBrokerChild {
    fn name(&self) -> &str { &self.name }

    fn fetch(
        &mut self,
        params: &FetchParams,
        _on_fact: &mut dyn FnMut(Fact) -> Result<()>,
    ) -> Result<FetchResult> {
        // WASM children use handle("sync", ...) — facts go directly
        // to events.db via host_emit, not through on_fact callback.
        let request = crate::mother::ChildRequest {
            action: "sync".to_string(),
            payload: serde_json::json!({
                "owner": params.params.get("owner").and_then(|v| v.as_str()).unwrap_or(""),
                "repo": params.params.get("repo").and_then(|v| v.as_str()).unwrap_or(""),
                "limit": params.limit,
                "since": params.since,
            }),
        };
        let response = self.inner.handle(&request)?;

        let issues = response.payload.get("issues_emitted").and_then(|v| v.as_u64()).unwrap_or(0);
        let prs = response.payload.get("prs_emitted").and_then(|v| v.as_u64()).unwrap_or(0);

        Ok(FetchResult { emitted: issues + prs, cursor: None })
    }

    fn health(&self) -> Result<HealthStatus> {
        let h = self.inner.health();
        Ok(match h {
            crate::mother::ChildHealth::Healthy =>
                HealthStatus { status: Status::Ok, message: None, latency_ms: None },
            crate::mother::ChildHealth::Degraded(msg) =>
                HealthStatus { status: Status::Degraded, message: Some(msg), latency_ms: None },
            crate::mother::ChildHealth::Unhealthy(msg) =>
                HealthStatus { status: Status::Down, message: Some(msg), latency_ms: None },
        })
    }

    fn shutdown(&mut self) -> Result<()> {
        self.inner.on_unload();
        Ok(())
    }
}
```

## 4. Native Child Spawn

```rust
// src/broker/spawn.rs

use anyhow::Result;
use patina_pipe_types::{ChildManifest, InitializeParams, AuthConfig};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Command, Stdio};

/// Spawn a native child in OS sandbox with credentials.
pub fn spawn_native(
    child_name: &str,
    manifest: &ChildManifest,
    connection_name: &str,
) -> Result<super::lifecycle::NativeChild> {
    let binary = resolve_child_binary(child_name)?;
    let init_params = build_init_params(connection_name)?;

    // Build spawn command (with or without sandbox)
    let mut cmd = if cfg!(target_os = "macos") && !is_sandbox_debug() {
        let profile = generate_sandbox_profile(manifest);
        let profile_path = write_temp_profile(&profile)?;
        let mut c = Command::new("sandbox-exec");
        c.args(["-f", profile_path.to_str().unwrap_or("")]);
        c.arg(&binary);
        c
    } else {
        Command::new(&binary)
    };

    cmd.stdin(Stdio::piped())
       .stdout(Stdio::piped())
       .stderr(Stdio::inherit());

    let mut process = cmd.spawn()?;
    let stdin = process.stdin.take().expect("stdin piped");
    let stdout = process.stdout.take().expect("stdout piped");

    let mut conn = ChildConnection {
        writer: BufWriter::new(stdin),
        reader: BufReader::new(stdout),
        id_counter: 1,
    };

    // pipe/initialize handshake
    let init_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": conn.next_id(),
        "method": "pipe/initialize",
        "params": init_params,
    });
    conn.write_request(&init_request)?;
    let _init_response = conn.read_response()?;

    Ok(super::lifecycle::NativeChild {
        name: child_name.to_string(),
        conn,
        process,
    })
}

/// Search order: ~/.patina/children/<name>/, PATH, ./target/release/
fn resolve_child_binary(name: &str) -> Result<std::path::PathBuf> {
    let home_path = dirs::home_dir()
        .unwrap_or_default()
        .join(format!(".patina/children/{}/{}", name, name));
    if home_path.exists() { return Ok(home_path); }

    if let Ok(path) = which::which(name) { return Ok(path); }

    let dev_path = std::path::PathBuf::from(format!("target/release/{}", name));
    if dev_path.exists() { return Ok(dev_path); }

    anyhow::bail!("child binary '{}' not found", name);
}

/// Build InitializeParams from connection config + vault.
fn build_init_params(connection_name: &str) -> Result<InitializeParams> {
    let config = crate::connect::ConnectionConfig::load(connection_name)?;
    let token = crate::secrets::get_global_secret(&config.connection.credential)?
        .ok_or_else(|| anyhow::anyhow!(
            "credential '{}' not found. Run `patina connect {}`",
            config.connection.credential, connection_name
        ))?;

    Ok(InitializeParams {
        protocol_version: "1.0".to_string(),
        auth: Some(AuthConfig { token, provider: config.connection.provider }),
    })
}

fn is_sandbox_debug() -> bool {
    std::env::var("PATINA_SANDBOX_DEBUG").is_ok()
}

/// Stdio connection to a child process.
pub(super) struct ChildConnection {
    writer: BufWriter<std::process::ChildStdin>,
    reader: BufReader<std::process::ChildStdout>,
    id_counter: u64,
}

impl ChildConnection {
    pub fn next_id(&mut self) -> u64 {
        let id = self.id_counter;
        self.id_counter += 1;
        id
    }

    pub fn write_request(&mut self, request: &serde_json::Value) -> Result<()> {
        writeln!(self.writer, "{}", serde_json::to_string(request)?)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn read_message(&mut self) -> Result<serde_json::Value> {
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        Ok(serde_json::from_str(line.trim())?)
    }

    pub fn read_response(&mut self) -> Result<serde_json::Value> {
        loop {
            let msg = self.read_message()?;
            if msg.get("id").is_some() { return Ok(msg); }
        }
    }
}
```

## 5. Routing Engine

### 5.1 Schema Validation

Extracts the validation pattern from `host_support::validate_emit()`
for use with native children. Same checks, different context.

```rust
// src/broker/routing.rs

use anyhow::Result;
use patina_pipe_types::Fact;
use std::collections::HashMap;

/// Validate a fact against the child's declared schemas.
///
/// Checks:
/// 1. Fact.schema exists in the child's declared schemas
/// 2. Fact.fact_type exists in that schema
/// 3. Fact.content_hash matches recomputed hash of Fact.data
///
/// Returns the event_type string for events.db on success.
pub fn validate_fact(
    fact: &Fact,
    schema_facts: &HashMap<String, HashMap<String, String>>,
    child_name: &str,
) -> Result<String> {
    let facts = schema_facts.get(&fact.schema)
        .ok_or_else(|| anyhow::anyhow!(
            "schema '{}' not declared by child '{}'", fact.schema, child_name
        ))?;

    let event_type = facts.get(&fact.fact_type)
        .ok_or_else(|| anyhow::anyhow!(
            "fact_type '{}' not in schema '{}'", fact.fact_type, fact.schema
        ))?;

    let expected_hash = patina_pipe_types::canonical::content_hash(&fact.data);
    if fact.content_hash != expected_hash {
        anyhow::bail!(
            "content_hash mismatch for {}.{}: expected {}, got {}",
            fact.schema, fact.fact_type, expected_hash, fact.content_hash
        );
    }

    Ok(event_type.clone())
}

/// Write a validated fact to events.db with dedup check.
pub fn write_fact_to_eventlog(
    conn: &rusqlite::Connection,
    fact: &Fact,
    event_type: &str,
    child_name: &str,
) -> Result<u64> {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let source_id = format!("child:{}", child_name);
    let data_str = fact.data.to_string();

    // Dedup: check if identical data already exists for this event type
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM eventlog WHERE event_type = ?1 AND data = ?2",
        rusqlite::params![event_type, &data_str],
        |row| row.get(0),
    ).unwrap_or(false);

    if exists { return Ok(0); } // duplicate, skip

    conn.execute(
        "INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data, provenance)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![event_type, &timestamp, &source_id, Option::<&str>::None, &data_str, "external"],
    )?;

    Ok(conn.last_insert_rowid() as u64)
}
```

### 5.2 Dedup Strategy

Simple data string comparison for now. Future: add `content_hash`
column to events.db for indexed dedup lookups. Current approach is
correct but O(n) per check against existing data.

## 6. Cursor Management

Design constraint: **Cursor update + fact writes in same SQLite
transaction.**

```rust
// src/broker/cursor.rs

use anyhow::Result;
use rusqlite::Connection;

fn cursor_key(source_name: &str) -> String {
    format!("cursor:{}", source_name)
}

/// Get stored cursor for a source.
pub fn get_cursor(conn: &Connection, source_name: &str) -> Result<Option<String>> {
    crate::eventlog::get_last_processed(conn, &cursor_key(source_name))
}

/// Write facts and update cursor in a single transaction.
///
/// Both succeed or both fail. On rollback: cursor is not advanced,
/// no partial facts written. Next run re-emits (at-least-once)
/// and dedup handles overlap.
pub fn write_facts_with_cursor(
    conn: &Connection,
    source_name: &str,
    facts: &[ValidatedFact],
    cursor: Option<&str>,
) -> Result<WriteResult> {
    let tx = conn.unchecked_transaction()?;

    let mut written = 0;
    let mut skipped = 0;

    for fact in facts {
        let seq = super::routing::write_fact_to_eventlog(
            &tx, &fact.fact, &fact.event_type, &fact.child_name,
        )?;
        if seq > 0 { written += 1; } else { skipped += 1; }
    }

    if let Some(cursor_value) = cursor {
        crate::eventlog::set_last_processed(&tx, &cursor_key(source_name), cursor_value)?;
    }

    tx.commit()?;

    Ok(WriteResult { written, skipped })
}

pub struct ValidatedFact {
    pub fact: patina_pipe_types::Fact,
    pub event_type: String,
    pub child_name: String,
}

pub struct WriteResult {
    pub written: u64,
    pub skipped: u64,
}
```

## 7. Broker Public API

```rust
// src/broker/mod.rs

mod sources;
mod lifecycle;
mod spawn;
mod routing;
mod cursor;

use anyhow::Result;
pub use sources::{ProjectSources, SourceEntry};

/// Run a single source — `patina mother run <name>`.
///
/// Flow: find source → load connection → load manifest → spawn child
/// → pipe/initialize → pipe/fetch → validate facts → write to
/// events.db + cursor (transactional) → pipe/shutdown
pub fn run_source(source_name: &str) -> Result<RunResult> {
    // 1. Find the source in registered projects
    let all_sources = sources::scan_all_sources()?;
    let (project_root, _, source) = all_sources.iter()
        .find(|(_, name, _)| name == source_name)
        .ok_or_else(|| anyhow::anyhow!(
            "source '{}' not found in any sources.toml", source_name
        ))?;

    // 2. Load connection + manifest
    let conn_config = crate::connect::ConnectionConfig::load(&source.connection)?;
    let manifest_path = resolve_child_manifest(&conn_config.connection.child)?;
    let manifest = patina_pipe_types::ChildManifest::from_path(&manifest_path)?;
    let schema_facts = load_schema_facts(&manifest, project_root)?;

    // 3. Get stored cursor
    let events_conn = crate::eventlog::open_events_db()?;
    let stored_cursor = cursor::get_cursor(&events_conn, source_name)?;

    // 4. Build fetch params
    let fetch_params = patina_pipe_types::FetchParams {
        types: source.types.clone(),
        since: stored_cursor,
        limit: 0,
        params: source.params.clone(),
    };

    // 5. Spawn child
    eprintln!("[broker] spawning {} for source '{}'",
        conn_config.connection.child, source_name);
    let mut child = spawn::spawn_native(
        &conn_config.connection.child, &manifest, &source.connection,
    )?;

    // 6. Fetch + validate
    let mut validated = Vec::new();
    let child_name = child.name().to_string();

    let result = child.fetch(&fetch_params, &mut |fact| {
        match routing::validate_fact(&fact, &schema_facts, &child_name) {
            Ok(event_type) => {
                validated.push(cursor::ValidatedFact {
                    fact, event_type, child_name: child_name.clone(),
                });
            }
            Err(e) => eprintln!("[broker] dropped invalid fact: {}", e),
        }
        Ok(())
    })?;

    // 7. Write facts + cursor (transactional)
    let write_result = cursor::write_facts_with_cursor(
        &events_conn, source_name, &validated, result.cursor.as_deref(),
    )?;

    // 8. Shutdown
    child.shutdown()?;

    eprintln!("[broker] '{}': {} written, {} dedup, cursor: {}",
        source_name, write_result.written, write_result.skipped,
        result.cursor.as_deref().unwrap_or("none"));

    Ok(RunResult {
        source: source_name.to_string(),
        emitted: result.emitted,
        written: write_result.written,
        skipped: write_result.skipped,
        cursor: result.cursor,
    })
}

/// Show all configured sources with status.
pub fn status() -> Result<()> {
    let all_sources = sources::scan_all_sources()?;
    if all_sources.is_empty() {
        eprintln!("No sources configured. Add [sources.<name>] to .patina/sources.toml");
        return Ok(());
    }

    let events_conn = crate::eventlog::open_events_db()?;
    for (root, name, source) in &all_sources {
        let cursor = cursor::get_cursor(&events_conn, name)?;
        let project = root.file_name().unwrap_or_default().to_string_lossy();
        eprintln!("  {} ({}/{}): schedule={}, types={:?}, cursor={}",
            name, project, source.connection, source.schedule,
            source.types, cursor.as_deref().unwrap_or("none"));
    }
    Ok(())
}

pub struct RunResult {
    pub source: String,
    pub emitted: u64,
    pub written: u64,
    pub skipped: u64,
    pub cursor: Option<String>,
}

fn resolve_child_manifest(child_name: &str) -> Result<std::path::PathBuf> {
    let global = dirs::home_dir().unwrap_or_default()
        .join(format!(".patina/children/{}/child.toml", child_name));
    if global.exists() { return Ok(global); }
    let local = std::path::PathBuf::from(format!("children/{}/child.toml", child_name));
    if local.exists() { return Ok(local); }
    anyhow::bail!("child manifest not found for '{}'", child_name);
}

fn load_schema_facts(
    manifest: &patina_pipe_types::ChildManifest,
    project_root: &std::path::Path,
) -> Result<std::collections::HashMap<String, std::collections::HashMap<String, String>>> {
    let mut all_facts = std::collections::HashMap::new();
    for schema_name in manifest.schemas.keys() {
        let path = project_root.join(format!(".patina/schemas/{}/schema.toml", schema_name));
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(table) = content.parse::<toml::Table>() {
                if let Some(arr) = table.get("facts").and_then(|v| v.as_array()) {
                    let map: std::collections::HashMap<String, String> = arr.iter()
                        .filter_map(|f| {
                            let t = f.as_table()?;
                            Some((
                                t.get("name")?.as_str()?.to_string(),
                                t.get("event_type")?.as_str()?.to_string(),
                            ))
                        }).collect();
                    all_facts.insert(schema_name.clone(), map);
                }
            }
        }
    }
    Ok(all_facts)
}
```

## 8. CLI Commands

Add to existing `patina mother` subcommands in
`src/commands/mother/mod.rs`:

```rust
/// Run a source (fetch, validate, route facts)
Run {
    /// Source name from sources.toml
    name: String,
},

/// Show configured sources and their status
Sources,
```

Dispatch:
```rust
MotherCommands::Run { name } => crate::broker::run_source(&name),
MotherCommands::Sources => crate::broker::status(),
```

## 9. On-Scrape Scheduling

After `patina scrape` completes local work, trigger on-scrape sources:

```rust
// Add to src/commands/scrape/mod.rs after local scrape:

fn trigger_on_scrape_sources() {
    let sources = match crate::broker::sources::scan_all_sources() {
        Ok(s) => s,
        Err(_) => return,
    };
    for (_, name, source) in sources {
        if source.schedule == "on-scrape" {
            eprintln!("[scrape] triggering source '{}'...", name);
            match crate::broker::run_source(&name) {
                Ok(r) => eprintln!("[scrape] '{}': {} facts", name, r.written),
                Err(e) => eprintln!("[scrape] '{}' failed: {}", name, e),
            }
        }
    }
}
```

Hourly/daily is `continuous-operation` scope — the daemon scheduler
calls `broker::run_source()` on interval. The broker exposes the
function; it doesn't own scheduling.

## 10. No Fan-Out Optimization

One child spawn per `run_source()` call. Multiple projects referencing
the same connection get separate spawns. Content-hash dedup handles
data overlap.

This is correct because:
- Each source has its own cursor
- Each source writes to its own project's events.db
- No shared state between runs

Optimize when measured need exists.

## Commits

1. `broker: add sources.toml reader`
   — src/broker/sources.rs with SourceEntry, ProjectSources,
   scan_all_sources(). Parse tests.

2. `broker: add BrokerChild trait with WASM and native adapters`
   — src/broker/lifecycle.rs with trait + NativeChild + WasmBrokerChild.

3. `broker: add native child spawn with sandbox`
   — src/broker/spawn.rs with spawn_native(), resolve_child_binary(),
   build_init_params(), ChildConnection.

4. `broker: add fact routing and schema validation`
   — src/broker/routing.rs with validate_fact(),
   write_fact_to_eventlog(). Dedup via data comparison.

5. `broker: add transactional cursor management`
   — src/broker/cursor.rs with write_facts_with_cursor() in single
   SQLite transaction.

6. `broker: add run_source() and status() public API`
   — src/broker/mod.rs orchestrating full flow end-to-end.

7. `mother: add run and sources CLI commands`
   — Wire into MotherCommands. Verify: `patina mother run github`
   and `patina mother sources` work.

8. `scrape: trigger on-scrape sources after local scrape`
   — Wire into scrape command.

## Key Files

- `src/broker/mod.rs` — public API (run_source, status)
- `src/broker/sources.rs` — sources.toml reader
- `src/broker/lifecycle.rs` — BrokerChild trait (WASM + native)
- `src/broker/spawn.rs` — native child spawn with sandbox
- `src/broker/routing.rs` — fact validation + dedup
- `src/broker/cursor.rs` — transactional cursor management
- `src/mother/child.rs` — existing MotherChild trait (WASM)
- `src/plugin/internal/host_support.rs` — validation reference
- `src/commands/mother/mod.rs` — CLI wiring

## Open Questions

1. **Events.db per project vs shared.** Currently `open_events_db()`
   opens `.patina/local/data/events.db` relative to cwd. The broker
   routes facts to a specific project's events.db. Need a variant
   that accepts a project root path parameter — `open_events_db_at(path)`.

2. **Schema loading scope.** Schemas could be installed globally
   (`~/.patina/schemas/`) or per-project. The design loads from the
   destination project's `.patina/schemas/`. If the schema isn't
   installed in the target project, validation fails. Auto-install
   from child manifest is future work.

3. **WASM child fact routing.** WasmBrokerChild bypasses the broker's
   routing pipeline — facts go directly to events.db via host_emit.
   No content-hash dedup for WASM children. Acceptable for now (only
   forge is WASM). Unify later by having host_emit route through
   the broker instead of writing directly.
