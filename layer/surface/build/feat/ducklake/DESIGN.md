# Design: DuckLake — DuckDB Child with Local Parquet Lake

## Approach

The DuckLake child has agency. Mother grants it two toys —
a connector (authority to fetch) and storage (authority to write)
— via `pipe/initialize`, then gets out of the way. The child
uses its approved toys independently: spawns the connector,
derives HTTP enforcement from the connector grant, drives the
fetch cycle, handles partial failures, stores results. Mother is
capability grantor, not runtime dispatcher.

This is a different model from the current broker flow, where
Mother orchestrates every step. Here, Mother grants capabilities
and the child makes all workflow decisions. Per
[[children-have-agency-toys-are-capabilities]],
[[initialize-is-capability-grant]], and
[[connector-toy-is-indivisible-authority]].

## §1 — DuckLake Child

### Binary

```
children/ducklake/
  Cargo.toml
  child.toml
  src/main.rs
```

```toml
# Cargo.toml
[dependencies]
patina-pipe = { path = "../../crates/patina-pipe" }
patina-pipe-types = { path = "../../crates/patina-pipe-types" }
duckdb = { version = "1", features = ["bundled"] }
serde_json = "1"
```

```toml
# child.toml
[child]
name = "ducklake"
version = "0.1.0"
type = "lakehouse"
runtime = "native"
lifecycle = "poll"
description = "Autonomous DuckDB + DuckLake data lake"

[capabilities]
methods = ["run"]
agency = true
```

The `duckdb` crate bundles DuckDB as a C library (like `rusqlite`
with `bundled`). Builds in the child crate only — does not affect
the main patina binary.

### Initialization — Capability Grant

Mother grants a typed capability via `pipe/initialize`. The
grant is a concrete `DuckLakeGrant` on `InitializeParams` — not
an untyped blob. Authority is modeled, not improvised.

```rust
// crates/patina-pipe-types/src/config.rs

/// Parameters sent by Mother during pipe/initialize.
pub struct InitializeParams {
    pub protocol_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
    /// Typed capability grant for DuckLake children.
    /// Concrete before generic — no Option<Value> at trust boundaries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ducklake: Option<DuckLakeGrant>,
}

/// Capability grant for the DuckLake child.
/// Two toys: connector (authority to fetch) and storage
/// (authority to write). Fail closed on missing or malformed.
pub struct DuckLakeGrant {
    pub connector: ConnectorToy,
    pub storage: StorageToy,
}

/// Connector toy — indivisible capability bundle.
/// The child derives HTTP enforcement from this grant;
/// the proxy is not a separate toy. Per
/// [[connector-toy-is-indivisible-authority]].
pub struct ConnectorToy {
    pub binary: String,                // executable identity: "github-connector"
    pub credential: Option<String>,    // secret material: OAuth token
    pub injection: InjectionStrategy,  // how credential reaches the API
    pub allowed_domains: Vec<String>,  // policy boundary: approved domains
    pub params: Value,                 // behavior scope: { owner, repo }
    pub types: Vec<String>,            // behavior scope: ["issues", "prs"]
}

/// Storage toy — where the child writes results.
pub struct StorageToy {
    pub lake_path: String,             // ~/.patina/lakes/<name>/
}
```

The grant is typed all the way through — Mother constructs a
`DuckLakeGrant`, serde serializes it, the child deserializes it
back to the same type. No manual `Value` parsing. If the grant
is malformed, deserialization fails and init is rejected (fail
closed).

The connector toy bundles executable identity, secret material,
policy boundary, and behavior scope as one indivisible authority.
The child derives its HTTP proxy from this grant — it cannot have
"connector without policy" or "policy without connector."

When a second agentic child type exists, it gets its own typed
grant field on `InitializeParams`. Duplication is acceptable
until a real abstraction emerges. The pattern to copy is clear.

On initialize, the child extracts its typed grant — fail closed
if missing:

```rust
fn initialize(&mut self, params: &InitializeParams) -> Result<(), PipeError> {
    let grant = params.ducklake.as_ref().ok_or_else(|| PipeError::Fatal {
        message: "ducklake child requires a DuckLakeGrant in pipe/initialize".into(),
    })?;

    let lake_path = PathBuf::from(&grant.storage.lake_path);
    let db = Connection::open_in_memory()?;
    db.execute_batch("INSTALL ducklake; LOAD ducklake;")?;

    let catalog = lake_path.join("lake.ducklake");
    db.execute_batch(&format!(
        "ATTACH 'ducklake:{}' AS lake (DATA_PATH '{}')",
        catalog.display(), lake_path.display()
    ))?;

    db.execute_batch("
        CREATE TABLE IF NOT EXISTS lake._sync_cursors (
            source_name VARCHAR,
            data_type   VARCHAR,
            cursor      VARCHAR,
            last_run    TIMESTAMP,
            records_written BIGINT DEFAULT 0,
            status      VARCHAR DEFAULT 'ok',
            last_error  VARCHAR,
            PRIMARY KEY (source_name, data_type)
        )
    ")?;

    self.db = Some(db);
    self.grant = Some(grant.clone());
    Ok(())
}
```

### Fetch Cycle — Child Drives

After initialization, the child runs the full pipeline:

```rust
fn run(&mut self) -> Result<RunReport> {
    let config = &self.config;
    let db = &self.db;

    // 1. Spawn connector with credentials Mother provided
    let mut connector = spawn_connector(
        &config.connector.binary,
        &config.connector.credential,
        &config.connector.allowed_domains,
    )?;

    let mut report = RunReport::default();

    // 2. Fetch each type independently
    for data_type in &config.connector.types {
        let cursor = self.load_cursor(&config.source_name(), data_type);

        match self.fetch_and_store(&mut connector, data_type, cursor) {
            Ok(result) => {
                self.advance_cursor(data_type, &result);
                report.succeeded(data_type, result.written);
            }
            Err(e) if e.is_auth_error() => {
                // Escalate to Mother — child can't fix auth
                report.escalate(data_type, e);
            }
            Err(e) => {
                // Data flow error — record it, continue with other types
                self.record_error(data_type, &e);
                report.failed(data_type, e);
            }
        }
    }

    // 3. Shutdown connector
    connector.shutdown()?;
    Ok(report)
}

fn fetch_and_store(
    &self,
    connector: &mut ChildConnection,
    data_type: &str,
    cursor: Option<String>,
) -> Result<StoreResult> {
    // Send pipe/fetch for this type
    let fetch_params = json!({
        "types": [data_type],
        "since": cursor,
        "limit": 10000,
        "params": self.config.connector.params,
    });

    let (facts, fetch_result) = connector.request("pipe/fetch", fetch_params)?;

    // Auto-create table
    let table = event_type_to_table(data_type);
    self.db.execute_batch(&format!("
        CREATE TABLE IF NOT EXISTS lake.{table} (
            _ingested_at TIMESTAMP DEFAULT current_timestamp,
            _source_id VARCHAR,
            _content_hash VARCHAR,
            data JSON
        )
    "))?;

    // Batch insert
    let mut written = 0u64;
    let mut stmt = self.db.prepare(&format!(
        "INSERT INTO lake.{table} (_source_id, _content_hash, data)
         VALUES (?, ?, ?)"
    ))?;

    for fact in &facts {
        stmt.execute(params![
            &self.config.connector.binary,
            &fact.content_hash,
            &fact.data.to_string(),
        ])?;
        written += 1;
    }

    Ok(StoreResult {
        written,
        cursor: fetch_result.cursor,
    })
}
```

Key: each data type is fetched and stored independently. If
issues succeeds and PRs fails, the issues cursor advances and
the PR failure is recorded. Next run only re-fetches PRs.

### Using the Approved Connector Toy

The child uses its connector toy via `ChildConnection` from
`patina-pipe` — the same substrate Mother uses. The critical
step: the child **derives** its HTTP proxy from the connector
grant. The proxy is not a separate toy — it is the enforcement
mechanism that makes connector use safe. Per
[[connector-toy-is-indivisible-authority]].

```rust
fn use_connector_toy(connector: &ConnectorToy) -> Result<ChildConnection> {
    // Resolve approved binary (executable identity)
    let path = resolve_child_binary(&connector.binary)?;

    // Derive HTTP enforcement from the connector grant.
    // The proxy is built FROM the grant — policy boundary +
    // secret material → enforcement mechanism. The child cannot
    // have connector access without going through this step.
    let http_handler = patina_pipe::http_proxy::build_http_proxy(HttpProxyConfig {
        allowed_domains: connector.allowed_domains.clone(),
        credential: connector.credential.as_ref().map(|cred| ProxyCredential {
            value: cred.clone(),
            injection: match &connector.injection {
                InjectionStrategy::Bearer => ProxyInjection::Bearer,
                InjectionStrategy::Header { name } => ProxyInjection::Header { name: name.clone() },
                InjectionStrategy::InProcess => ProxyInjection::InProcess,
            },
        }),
    });

    // Spawn connector with derived enforcement
    let mut conn = ChildConnection::spawn_with_http(&path, http_handler)?;

    // Initialize connector — InProcess credential delivery if applicable
    let auth = match &connector.injection {
        InjectionStrategy::InProcess => connector.credential.as_ref().map(|c| json!({
            "token": c, "provider": "oauth"
        })),
        _ => None,
    };
    conn.request("pipe/initialize", json!({
        "protocol_version": "1.0",
        "auth": auth,
    }))?;

    Ok(conn)
}
```

The chain of authority:
1. Mother grants `ConnectorToy` (indivisible capability)
2. Child derives HTTP proxy from the grant (`build_http_proxy`)
3. Child spawns connector with derived enforcement (`ChildConnection`)
4. Connector has no direct network access — all HTTP proxied

The child becomes a mini-broker for its connector toy, using
the same substrate Mother uses. The connector toy is not raw
network permission — it is permission to spawn a specific
connector and proxy its HTTP through policy derived from the
grant. The child cannot expand beyond what Mother granted.

### Error Escalation

The child classifies errors and handles them differently:

```rust
enum LakeError {
    // Child handles — save partial, continue
    FetchFailed { data_type: String, detail: String },
    StoreFailed { data_type: String, detail: String },
    ConnectorCrashed { data_type: String },

    // Escalate to Mother — child can't fix
    AuthFailed { detail: String },
    ConnectorNotFound { binary: String },
}
```

Auth errors are detected from connector responses:
- HTTP 401/403 from the proxied API → auth error
- Connector reports `PipeError::Fatal` with auth context
- Child reports back to Mother: "connector auth broken"

Mother receives the escalation and tells the user:
```
Lake "github-data" reports: auth failed for github connector
Run: patina connect refresh github
```

## §2 — Mother Capability Grant

### Broker Changes

The broker gains a lake branch in `run_source()`. Critical: the
branch happens BEFORE spawning. For `Destination::Project`, Mother
spawns the connector and drives fetch. For `Destination::Lake`,
Mother spawns the lakehouse child and grants toy approvals:

```rust
// broker/mod.rs — run_source() branches before spawn
match &source.destination {
    Destination::Project => {
        // Current path: Mother spawns connector, drives fetch
        let (mut child, manifest) = spawn_native_with_plan(&auth_plan, ...)?;
        write_to_project(source, project_root, &mut child, &manifest)
    }
    Destination::Lake { name } => {
        // New path: Mother spawns lakehouse child, grants toy approvals
        grant_lake_capabilities(source, name, &auth_plan, no_sandbox)
    }
}
```

`grant_lake_capabilities` is simpler than `write_to_project` —
Mother grants capabilities and the child does the work:

```rust
fn grant_lake_capabilities(
    source: &SourceEntry,
    lake_name: &str,
    auth_plan: &AuthPlan,
    no_sandbox: bool,
) -> Result<WriteResult> {
    // 1. Resolve lake path (storage toy)
    let lake_path = resolve_lake_path(lake_name)?;

    // 2. Spawn DuckLake child (the child, not the connector)
    let lake_child_path = resolve_child_binary("ducklake")?;
    let mut lake_child = spawn_child(&lake_child_path.to_string_lossy())?;

    // 3. Build typed capability grant — no untyped blobs at trust boundaries
    let grant = DuckLakeGrant {
        connector: ConnectorToy {
            binary: auth_plan.child.clone(),
            credential: auth_plan.credential.as_ref().map(|c| c.value.clone()),
            injection: auth_plan.credential.as_ref()
                .map(|c| c.injection.clone())
                .unwrap_or(InjectionStrategy::Bearer),
            allowed_domains: auth_plan.allowed_domains.clone(),
            params: serde_json::to_value(&source.params)?,
            types: source.types.clone(),
        },
        storage: StorageToy {
            lake_path: lake_path.to_string_lossy().to_string(),
        },
    };

    let init_params = InitializeParams {
        protocol_version: "1.0".to_string(),
        auth: None,  // credential is in the grant, not auth
        ducklake: Some(grant),
    };

    lake_child.request("pipe/initialize", serde_json::to_value(&init_params)?)?;

    // 4. Tell child to run — child uses toys from here
    let (_notifs, result) = lake_child.request("pipe/run", json!({}))?;

    // 5. Handle escalation if any
    if let Some(escalation) = result.get("escalation") {
        handle_escalation(escalation)?;
    }

    Ok(WriteResult::from(&result))
}
```

Mother's involvement ends after step 3. Step 4 is a single call
— the child uses its approved toys and reports back.

### Lake Path Resolution

`~/.patina/lakes/<name>/lake.toml` — TOML file per lake:

```toml
name = "github-data"
created_at = "2026-03-10T12:00:00Z"
```

`resolve_lake_path` scans `~/.patina/lakes/` for the named lake.

### patina lake create

```rust
// src/commands/lake.rs
fn create(name: &str) -> Result<()> {
    let lake_dir = patina_home().join("lakes").join(name);
    fs::create_dir_all(&lake_dir)?;

    let config = format!(
        "name = \"{}\"\ncreated_at = \"{}\"",
        name, Utc::now().to_rfc3339()
    );
    fs::write(lake_dir.join("lake.toml"), config)?;

    println!("Lake \"{}\" created at {}", name, lake_dir.display());
    Ok(())
}
```

### patina mother status

Scans `~/.patina/lakes/*/lake.toml`, optionally opens each
`lake.ducklake` read-only to show table names and cursor state
from `_sync_cursors`.

## §3 — Pipe Protocol Extension

The child needs a `run` method beyond the existing `ingest`:

```
pipe/initialize  — Mother → child: capability grant (toy approvals)
pipe/run         — Mother → child: "go" — child uses toys independently
pipe/status      — Mother → child: query child state (optional)
```

`pipe/initialize` is a capability grant, not just startup config.
It carries the full set of toy approvals. Per
[[initialize-is-capability-grant]].

`pipe/run` replaces `pipe/ingest` for children with agency. The
child is not a passive receiver of records — it's an autonomous
actor that uses its approved toys. The response to `pipe/run`
includes the full report:

```json
{
  "result": {
    "types": {
      "issues": { "status": "ok", "written": 847, "cursor": "2026-03-12..." },
      "prs": { "status": "failed", "error": "rate limited", "written": 0 }
    },
    "escalation": null
  }
}
```

Or with auth escalation:
```json
{
  "result": {
    "types": {},
    "escalation": {
      "type": "auth_failed",
      "message": "GitHub API returned 401 — token may be expired",
      "action": "patina connect refresh github"
    }
  }
}
```

## §4 — Dependencies

### In the child binary only

- `duckdb` with `bundled` — compiles DuckDB from C source (~2-5
  min first build, cached after). ~50MB binary.
- `patina-pipe` — Child trait, ChildConnection, pipe protocol
- `patina-pipe-types` — shared types
- `serde_json` — JSON handling
- `reqwest` — via patina-pipe's http_proxy module

### In the main patina binary

Nothing new. DuckDB is isolated in the child. The pipe protocol
boundary keeps it separate.

## Commits

1. **`pipe-types: extend protocol for agentic children`**
   In patina-pipe-types:
   - Add `DuckLakeGrant`, `ConnectorToy`, `StorageToy` structs.
     Typed capability grant — no `Option<Value>` at trust boundaries.
   - Add `ducklake: Option<DuckLakeGrant>` to `InitializeParams`.
     Connectors ignore it (it's `None`). Existing `build_init_params`
     in `spawn.rs` never sets it, so connector path is unchanged.
   - Add `RunResult`, `RunReport`, `TypeReport`, `Escalation` types.
   In patina-pipe: add `run()` to `Child` trait with default
   Fatal("not implemented"). Add `pipe/run` dispatch to `lib.rs`
   run loop (after `pipe/ingest`, before `pipe/health`).
   When a second agentic child type exists, it gets its own typed
   grant field. Duplication is acceptable until abstraction emerges.

2. **`lake: add patina lake create + lake.toml`**
   `src/commands/lake.rs` — create directory, write lake.toml.
   `patina lake list` scans and reports. Wire into CLI.

3. **`ducklake: scaffold child binary`**
   `children/ducklake/` — Cargo.toml, child.toml, main.rs.
   Initialize: receive toy approvals, set up DuckDB + DuckLake.
   Run: use approved connector toy, per-type fetch→store cycle,
   partial failure handling, cursor tracking. Tests with mock
   connector toy and in-memory DuckDB.

4. **`broker: add lake capability grant`**
   `src/broker/mod.rs` — `grant_lake_capabilities()`. Branch
   before spawn: Destination::Lake spawns lakehouse child (not
   connector). Grant toy approvals via pipe/initialize, call
   pipe/run, handle escalation. Tests with mock child.

5. **`lake: end-to-end litmus`**
   Configure github connection + lake for anthropics/claude-code.
   Run. Verify DuckDB query works. Verify incremental sync.
   Verify partial failure handling.

## Key Files

- `children/ducklake/src/main.rs` — autonomous child
- `children/ducklake/Cargo.toml` — duckdb dependency
- `children/ducklake/child.toml` — manifest
- `src/broker/mod.rs` — lake handoff branch
- `src/commands/lake.rs` — CLI commands
- `crates/patina-pipe-types/src/` — RunResult types

## Dependencies

- **[[http-proxy-extraction]]** must land first. The DuckLake
  child derives its HTTP enforcement from the connector grant
  using `patina_pipe::http_proxy::build_http_proxy`. This is
  not convenience reuse — it is the mechanism that preserves
  the broker security invariant when authority moves from
  Mother to child. Per [[connector-toy-is-indivisible-authority]].

- **[[measure-process-owned]]** must land first. The DuckLake
  child needs `MeasureEvent` and `VALID_VERBS` from patina-pipe
  to emit telemetry to its local `_measure` table using the
  shared vocabulary.

## Open Questions

1. **Table naming.** `github.issue` → `issues`? Or use event
   type directly: `github_issue`? Must be predictable for user
   queries. Recommendation: strip provider prefix, pluralize
   — `issue` → `issues`, `pr` → `prs`.

3. **Extension caching.** `INSTALL ducklake` needs internet on
   first run. After that it's cached in `~/.duckdb/extensions/`.
   For CI, pre-cache or mock. For users, first run handles it.

4. **Concurrent access.** DuckLake is single-writer MVCC. Child
   holds write lock during ingest. `duckdb -readonly` can query
   concurrently. Verify child releases lock between runs.

5. **Child lifecycle.** Spawn per run (poll mode) or keep alive?
   Poll mode is simpler — spawn, run, shutdown. Matches connector
   pattern. Long-running child is future optimization.
