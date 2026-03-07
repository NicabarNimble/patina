# Design: Mother Broker — The Routing Engine

## Why This Work Exists

Mother today manages WASM plugins: spawn wasmtime, call handle(), write
events. But the pipe architecture introduces native children, multiple
runtimes, and explicit source declarations. Mother needs broker
responsibilities — the ability to read "I want GitHub issues for this
repo" and make it happen, regardless of whether the connector is WASM
or native.

The Netflix/Kafka pattern from [[session-20260306-123021]]: Mother is
the broker. She routes facts from sources to destinations based on
declarations. She never transforms data. She manages lifecycle,
resolves credentials, validates schemas, and writes events. Children
produce facts. Mother routes them.

[[mother-holds-connections-pipes-transform]] captures this: "Mother
manages connections, spawns children, routes facts." The broker module
is where this responsibility lives in code.

**Origin:** [[session-20260306-123021]] (Mother as broker, Netflix
pattern, pub/sub declarations), [[session-20260306-174214]] (audit:
transactional cursor+fact writes in same SQLite txn, no fan-out
optimization, WASM fact routing bypass acknowledged).

## What Exists Today

Mother already has substantial infrastructure:

- `src/mother/mod.rs` — project registry, graph, client
- `src/mother/child.rs` — MotherChild trait (WASM children)
- `src/plugin/internal/mother_child.rs` — WasmChild adapter
- `src/plugin/internal/host_support.rs` — emit validation, HTTP proxy
- `src/commands/mother/` — CLI commands, daemon

The broker does NOT replace any of this. It adds a routing layer that
uses existing pieces and extends Mother's CLI with `run` and `sources`.

## Design Decisions

### 1. Unified Child Lifecycle (BrokerChild Trait)

The broker needs to talk to both WASM and native children through one
interface. But the existing `MotherChild` trait is WASM-specific
(handle, health, on_load/on_unload). A new trait bridges both:

```rust
pub trait BrokerChild {
    fn name(&self) -> &str;
    fn fetch(&mut self, params: &FetchParams,
             on_fact: &mut dyn FnMut(Fact) -> Result<()>) -> Result<FetchResult>;
    fn health(&self) -> Result<HealthStatus>;
    fn shutdown(&mut self) -> Result<()>;
}
```

Key difference from Child trait (patina-pipe): BrokerChild is
Mother-side. It wraps the communication channel. NativeChild wraps
a subprocess (stdin/stdout JSON-RPC). WasmBrokerChild wraps an
existing MotherChild (in-process WASM calls).

The `on_fact` callback enables streaming — the broker processes each
fact as it arrives without buffering. For native children, this means
reading interleaved `pipe/fact` notifications from stdout. For WASM
children, facts currently bypass the broker (see decision #5).

### 2. sources.toml — Declarative Source Configuration

Per-project file at `.patina/sources.toml`. Declares what external data
this project wants. Mother reads these across all registered projects.

```toml
[sources.github]
connection = "github"                    # ~/.patina/connections/github.toml
params = { owner = "NicabarNimble", repo = "patina" }
types = ["issues", "prs"]
schedule = "on-scrape"

[sources.github-docs]
connection = "github"                    # same connection, different repo
params = { owner = "NicabarNimble", repo = "docs" }
types = ["issues"]
schedule = "daily"
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `connection` | string | yes | Name in `~/.patina/connections/` |
| `params` | table | no | Provider-specific (passed to FetchParams.params) |
| `types` | array | no | Data types to fetch (default: all child capabilities) |
| `schedule` | string | no | "on-scrape", "hourly", "daily", "manual" (default) |

### 3. Transactional Cursor + Fact Writes

The Session 12 audit established this constraint: cursor update and
fact writes MUST be in the same SQLite transaction. Both succeed or
both fail.

**Why this matters:** If facts are written but the cursor isn't
advanced, the next run skips them (thinks they were already fetched).
If the cursor advances but facts aren't written, data is lost. The
only safe model is both in one transaction.

On rollback: cursor stays at the old position, no partial facts
written. Next run re-emits (at-least-once) and content-hash dedup
handles overlap.

```rust
pub fn write_facts_with_cursor(
    conn: &Connection,
    source_name: &str,
    facts: &[ValidatedFact],
    cursor: Option<&str>,
) -> Result<WriteResult> {
    let tx = conn.unchecked_transaction()?;
    // write all facts + update cursor
    tx.commit()?;
}
```

### 4. No Fan-Out Optimization

One child spawn per `run_source()` call. Multiple projects referencing
the same connection get separate spawns. Content-hash dedup handles
data overlap.

This is correct because:
- Each source has its own cursor
- Each source writes to its own project's events.db
- No shared state between runs

The Session 12 audit explicitly decided: no fan-out optimization.
Optimize when measured need exists. Simple is correct until proven
insufficient.

### 5. WASM Children Bypass Broker Routing

WasmBrokerChild wraps existing MotherChild, but WASM facts go directly
to events.db via `host_emit` — the existing `host_support::emit_fact`
path. The broker doesn't intercept them. No content-hash dedup for
WASM children.

This is a conscious trade-off during development:
- Only forge is WASM. It works. Don't break it during broker buildout.
- Unifying WASM emission through the broker requires changing
  host_emit to route through broker instead of writing directly.
  That's a deeper refactor with risk.
- The asymmetry is acceptable while the native broker path is being
  proven, but it MUST be resolved before this spec can close.

[[temporal-layering-causes-drift]] warns about this pattern. The
deadline is the spec itself: EC `wasm-routing-resolved` gates
completion. Before mother-broker is marked complete, either:
(a) forge routes through the broker (content-hash dedup, schema
    validation, transactional writes — same path as native children),
    OR
(b) forge is explicitly declared legacy with a documented decision
    that no future WASM child may bypass the broker. The host_emit
    direct-write path becomes a frozen legacy codepath, not a pattern
    to follow.

### 6. Fact Validation Extracted from host_support

The validation logic in `host_support::validate_emit()` (schema check,
fact_type check) is reused by the broker's routing engine. The broker
adds content-hash verification on top:

1. `fact.schema` exists in the child's declared schemas
2. `fact.fact_type` exists in that schema
3. `fact.content_hash` matches recomputed hash of `fact.data`
4. Dedup check (data string comparison for now, content_hash index later)

Invalid facts are logged and dropped. Mother never writes unvalidated
data.

### 7. Child Binary Resolution

Search order for finding a child binary:

1. `~/.patina/children/<name>/<name>` — installed children
2. `PATH` — system-installed children
3. `./target/release/<name>` — development builds

This supports both production installs and development workflows.
During development, `cargo build --release` puts the binary in
`target/release/` and `patina mother run github` finds it.

### 8. Connection Config Format (Gap 12)

Connection configs live at `~/.patina/connections/{name}.toml`. The
`connection` field in `sources.toml` maps 1:1 to the filename.

Two formats are supported — rich (from `patina connect`) and minimal
(hand-authored). Mother reads both identically via the shared
`[connection]` section:

```toml
# Rich format (created by patina connect)
schema_version = 0

[connection]
name = "github"
provider = "github"
credential = "github:user"       # vault secret name
child = "github-connector"       # child binary to spawn
created = "2026-03-06T00:00:00Z"

[oauth]
client_id = "Iv1.xxxxxxxx"
scopes = ["repo", "read:org"]
```

```toml
# Minimal format (hand-authored, pre-connect)
schema_version = 0

[connection]
provider = "github"
credential = "github-token"      # vault secret name
child = "github-connector"
```

`schema_version` is required in every file. Mother's compatibility
rules:

| schema_version | Mother behavior |
|----------------|-----------------|
| 0              | Full support (current) |
| Missing        | Reject with error: "connection config missing schema_version" |
| > 0 (unknown)  | Warn: "schema_version {n} newer than supported (0), attempting read" — parse `[connection]` section anyway. Error only if `[connection]` is missing or unparsable. |

This is forward-compatible by design: Mother only reads `[connection]`.
A newer patina-connect emitting `schema_version = 1` with additional
sections (OAuth refresh metadata, multi-account fields) won't break
an older Mother — the new sections are opaque. The warn-and-attempt
strategy prevents configuration outages when team members update CLI
tools ahead of the broker.

**Origin:** [[spec-patina-connect]] defines the rich format. The
minimal shim exists so early adopters can `patina secrets add` +
hand-write a connection TOML before the OAuth flow lands.

### 9. Credential Delivery Path (Gap 13)

Two-tier model: Mother always injects credentials transparently for
`pipe/http` requests. The child optionally receives the raw token
via `pipe/initialize` for APIs that require body signing.

**Tier 1 — Transparent injection (default, all children):**

Mother decrypts the vault secret, holds it in memory, and injects it
as a Bearer header during `pipe/http` proxy. The child never sees the
raw credential. No manifest flag needed — if Mother has a credential
for the target domain, it injects automatically.

**Tier 2 — In-process token (opt-in, rare):**

`auth.requires_in_process_token = true` in `child.toml`. Mother
includes `auth.token` in the `pipe/initialize` params. Required for
APIs that need body signing (e.g., AWS SigV4) where header injection
is insufficient.

When this flag is set, Mother logs a per-run warning:
```
[broker] github-connector: child holds raw credential (audit trail active)
```

This is a v1 escape hatch, not the long-term model. Known limitations:
- No capability negotiation: child discovers missing token at runtime
  if the flag is absent and signing is needed. Mitigation: structured
  error at `pipe/initialize` response if auth is required but no token
  delivered — child can abort early.
- No revocation/rotation: token lives in child memory for the process
  lifetime. Token refresh and short-lived scoped tokens are
  [[spec-patina-connect]] scope (OAuth token refresh), not broker v1.
- No per-request scopes: the full credential is injected. Per-call
  credential hints (request labels that Mother understands, scoped
  ephemeral tokens) are a future upgrade documented here as the v2
  security model.

The delivery flow:

1. `sources.toml` → `connection = "github"`
2. Mother reads `~/.patina/connections/github.toml` →
   `credential = "github:user"`, `child = "github-connector"`
3. Mother decrypts `"github:user"` via `secrets::get_global_secret()`
4. Mother reads `child.toml` manifest → checks
   `auth.requires_in_process_token`
5. If false: `pipe/initialize { protocol_version: "1.0" }` (no token)
6. If true: `pipe/initialize { auth: { token: "<value>" }, ... }`
   + audit warning logged
7. For `pipe/http`: Mother injects credential as Bearer header
   regardless of the flag

If a child requests a `pipe/http` call to a domain requiring auth but
the credential is missing (vault empty, connection config incomplete),
Mother returns a structured JSON-RPC error:

```json
{"code": -32004, "message": "credential required for api.github.com but not configured"}
```

This surfaces immediately to the integrator — no silent failures.

### 10. Cursor Storage Schema (Gap 14)

Dedicated table in events.db — not `scrape_meta`. Reusing `scrape_meta`
introduces hidden coupling: CLI scrape stores `last_processed_git` in
the same table, and there's no enforcement preventing key collisions
between broker cursors and scrape metadata. A separate table eliminates
the coupling entirely.

```sql
CREATE TABLE IF NOT EXISTS broker_cursors (
    source_name TEXT PRIMARY KEY,
    cursor_value TEXT NOT NULL CHECK(length(cursor_value) <= 4096),
    updated_at TEXT NOT NULL
);
```

The `CHECK(length(cursor_value) <= 4096)` constraint prevents children
from accidentally emitting megabyte cursor blobs that would round-trip
over JSON on every run. 4 KiB is generous for timestamps, page tokens,
and opaque cursors. If a child exceeds this, the INSERT fails with a
constraint error — loud, immediate, fixable.

```rust
pub fn get_cursor(conn: &Connection, source: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT cursor_value FROM broker_cursors WHERE source_name = ?1",
        [source], |row| row.get(0),
    ).optional()
}

pub fn set_cursor(conn: &Connection, source: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO broker_cursors (source_name, cursor_value, updated_at)
         VALUES (?1, ?2, ?3)",
        params![source, value, chrono::Utc::now().to_rfc3339()],
    )?;
    Ok(())
}
```

**Migration path:** `CREATE TABLE IF NOT EXISTS` in `ensure_events_db()`
creates the table on first broker run. No backfill from `scrape_meta` —
first broker run for each source starts with no cursor (equivalent to
a full sync). This is correct: existing `scrape_meta` keys like
`last_processed_git` are CLI scrape state, not broker cursors. The
broker has never run before, so there's nothing to migrate. If a team
was previously running a prototype broker that stored cursors in
`scrape_meta`, they accept a one-time full re-fetch — content-hash
dedup handles the overlap.

**Concurrency and failure modes:**

The transactional write wraps fact inserts + `set_cursor` in one
`unchecked_transaction()` — see §3. events.db uses WAL mode and
`busy_timeout = 5000` (5 seconds). Concurrent broker behavior:

| Scenario | Behavior |
|----------|----------|
| Two mothers, same source, same events.db | Second mother blocks on SQLite write lock up to 5s. If lock acquired: both runs succeed, dedup handles overlap. If BUSY after 5s: second run fails with `database is locked` error. |
| Two mothers, different sources, same events.db | Both succeed — they write to different cursor rows. SQLite WAL allows concurrent readers. Writers serialize at commit. |
| BUSY timeout exceeded | The run fails, cursor is NOT advanced (transaction rolled back). Next run re-fetches from the old cursor position. At-least-once guarantee holds. |

**Operator guidance:** If BUSY errors appear in logs, stagger source
schedules or increase `busy_timeout`. Do NOT retry the same run
immediately — that worsens contention. The at-least-once model means
a failed run is safe to skip; the next scheduled run catches up.

**Stale cursor cleanup:**

`updated_at` supports `patina mother sources` status display (when was
this source last fetched?). Source renames or removals leave dead rows.
Cleanup rules:

1. `patina mother sources` shows all cursors. Cursors with no matching
   `sources.toml` entry are flagged `(orphaned)`.
2. `patina mother sources --prune` deletes orphaned cursors after
   confirmation. No `--force` flag — always interactive.
3. Orphaned cursors are inert (no reads, no writes, no performance
   impact). Teams can safely ignore them — the prune command exists
   for hygiene, not correctness.

**Integration tests required:**

1. Single-mother: assert fact inserts + cursor update share one
   transaction (rollback on fact validation error leaves cursor
   unchanged).
2. Concurrent-mother: spawn two broker runs against the same
   events.db, verify both complete (or one fails with BUSY), no
   double-advanced cursor, dedup prevents duplicate facts.

### 11. Content-Hash Dedup (Gap 15)

Add `content_hash TEXT` column to events.db eventlog with a partial
UNIQUE index (NULL-safe — existing events without hashes unaffected).

Schema migration (applied by `ensure_events_db()`):

```sql
ALTER TABLE eventlog ADD COLUMN content_hash TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_eventlog_content_hash
    ON eventlog(content_hash) WHERE content_hash IS NOT NULL;
```

**Validation gate:** The broker rejects facts without `content_hash`
at validation time (§6 step 3). The pipe protocol already requires it
— `test-child` emits `blake3:`-prefixed hashes. Children that omit
`content_hash` get a validation error, not a silent NULL insert that
bypasses dedup.

**Dedup mechanism:** `INSERT OR IGNORE` with the unique index. SQLite
checks at B-tree insert time — no separate SELECT. On duplicate
detection (rows_affected == 0), log at debug level:

```
[broker] dedup: skipped fact content_hash=blake3:abc123... (already in eventlog)
```

This makes dedup visible in debug logs rather than silently counting.
Blake3 collision probability is ~2^-128 — not a realistic concern for
data integrity. However, if an operator suspects corruption, they can
query `SELECT content_hash, count(*) FROM eventlog GROUP BY
content_hash HAVING count(*) > 1` to verify no collisions exist.

`WriteResult` tracks dedup:

```rust
pub struct WriteResult {
    pub inserted: u64,
    pub dedup_skipped: u64,
    pub cursor: Option<String>,
}
```

`dedup_skipped` is logged per-run and emitted as a metric so
operators can see dedup effectiveness. Historical events with NULL
hashes are NOT backfilled by migration — children re-emit on next
run and dedup handles overlap naturally (at-least-once guarantee).

### 12. Pipe/HTTP Production Handler (Gap 16)

The production handler lives in `src/broker/http.rs` and reuses
`host_support` functions directly. Those functions change visibility
from `pub(super)` to `pub(crate)`:

- `host_support::validate_http_url()` — URL parsing + HTTPS enforcement
- `host_support::build_http_client()` — redirect rejection policy
- `host_support::inject_credential()` — Bearer header injection
- `host_support::leak_check()` — response body credential scanning

```rust
// src/broker/http.rs
pub fn build_production_handler(
    manifest: &ChildManifest,
    credential: Option<(String, String)>,  // (secret_name, value)
) -> Result<HttpHandler> {
    let client = host_support::build_http_client()?;  // fallible, no unwrap
    let allowed: HashSet<String> = manifest.domains.allowed
        .iter().map(|d| d.to_lowercase()).collect();  // normalized
    let cred = credential.clone();

    Ok(Box::new(move |req: &PipeHttpRequest| {
        let domain = host_support::validate_http_url(&req.url)?;
        // validate_http_url returns lowercase, port-stripped domain
        if !allowed.contains(&domain) {
            return Err(format!("domain '{}' not in allowlist", domain));
        }
        // build request, inject credential, execute, leak_check response
    }))
}
```

**Client construction is fallible** — returns `Result`, never panics.
If TLS or platform issues prevent client creation, the broker reports
a structured error for the source rather than crashing.

**Domain normalization:** `validate_http_url()` must return a
canonicalized domain: lowercased, port-stripped (`:443` is implicit
for HTTPS), ASCII-only (`url::Url` handles punycode via IDNA
transparently). The allowlist in `child.toml` is also lowercased at
load time. This prevents mismatches like `api.GitHub.com` vs
`api.github.com` or `api.github.com:443` vs `api.github.com`.

The HTTP client is created once per child (in `build_production_handler`)
and reused across all `pipe/http` requests from that child — avoids
TLS handshake churn. Existing tests on `validate_http_url` and
`leak_check` continue to cover the shared logic; broker tests add
domain normalization cases (case, port, punycode).

### 13. NativeChild Adapter (Gap 17)

`NativeChild` wraps `ChildConnection` with adapter logic:

```rust
impl BrokerChild for NativeChild {
    fn fetch(&mut self, params: &FetchParams,
             on_fact: &mut dyn FnMut(Fact) -> Result<()>) -> Result<FetchResult> {
        let (notifications, response) = self.conn.request("pipe/fetch", params.to_json())?;
        for notif in notifications {
            let fact = parse_pipe_fact(&notif)?;
            on_fact(fact)?;
        }
        Ok(FetchResult::from_response(&response)?)
    }
}
```

**Buffering limitation:** `ChildConnection::request()` collects all
notifications in memory before returning. For a first sync against a
large source, this means the entire batch lives in memory until the
child completes. This has concrete consequences:

- **OOM risk:** A source emitting 100k+ facts will accumulate all of
  them before any database write occurs.
- **All-or-nothing failure:** One bad fact at position 99,999 drops
  the entire batch (transactional rollback). No partial progress.
- **Pipe backpressure:** stdout buffer limits may cause the child to
  block if Mother doesn't drain fast enough.

**v1 mitigation:** Configurable `max_batch_size` (default: 10,000).
If a child emits more facts than the limit, the broker aborts with
a clear error:

```
[broker] github: batch limit exceeded (10247 > 10000). Configure
  smaller page size via source params or increase max_batch_size.
```

Children control batch size via `params.limit` in `sources.toml`.
The broker enforces the hard cap as a safety net.

**v2 path (streaming):** Refactor `ChildConnection::request()` into
a streaming iterator that drains notifications in chunks, writing
intermediate batches with intermediate cursors. This changes the
transactional model from "all-or-nothing" to "chunked progress with
at-least-once replay from last checkpoint." Documented here as the
upgrade path — do not attempt before v1 is proven on real workloads.

**Error attribution:** Wrap `on_fact` calls with tracing context so
failures are attributed to either the child (malformed fact) or the
broker (validation/write error). Currently errors bubble up without
attribution — this tracing ensures operators can diagnose batch
abort causes.

### 14. Schema Validation Scope (Gap 18)

Schemas load from the **destination project's** `.patina/schemas/`
directory — same path WASM children use today (see
`PluginManifest::parse_schema_facts` in `src/plugin/internal/mod.rs`).

If the schema isn't installed: **warn + pass-through**. Facts are
written without schema validation. Rationale:

- Hard-fail blocks useful data from arriving
- Security is enforced by domain allowlist + sandbox, not schemas
- Schema enforcement is a correctness tool, not a security gate

Warning behavior: one log line per missing schema per run. Not per
fact — keeps logs readable while ensuring teams notice:

```
[broker] schema 'github' not installed for project 'patina' — facts written without validation
```

**CLI support:** `patina schemas status` (future, not this spec)
enumerates which schemas are installed versus referenced across all
configured sources. Gives ops teams a single command to close the
deployment gap without combing logs. Noted here for the roadmap but
not gated as an EC for mother-broker.

## The Full Flow: run_source()

```
patina mother run github
  |
  +-- Find source "github" in .patina/sources.toml
  |
  +-- Load connection config (~/.patina/connections/github.toml)
  |     +-- Validate schema_version = 0
  |     +-- Read credential name, child name, provider
  |
  +-- Load child manifest (child.toml from resolved binary path)
  |
  +-- Decrypt credential from vault (secrets::get_global_secret)
  |
  +-- Build production pipe/http handler (§12: cached client + domain allowlist)
  |
  +-- Get stored cursor from events.db (scrape_meta cursor:{source})
  |
  +-- Spawn child (fork+exec in sandbox)
  |     |
  |     +-- pipe/initialize {protocol_version}
  |     |     +-- if auth.requires_in_process_token: include auth.token
  |     |     +-- default: no token (Mother injects via pipe/http)
  |     |
  |     +-- pipe/fetch {types, since: cursor, params: {owner, repo}}
  |     |     |
  |     |     +-- pipe/fact notifications (collected, schema-validated)
  |     |     +-- pipe/http requests (domain-checked, credential-injected)
  |     |
  |     +-- pipe/shutdown
  |
  +-- Write facts + cursor to events.db (single unchecked_transaction)
  |     +-- INSERT OR IGNORE with content_hash dedup
  |     +-- set_cursor in same transaction
  |
  +-- Report: "github: 47 written, 3 dedup, cursor: 2026-03-06T..."
```

## Module Structure

```
src/
  broker/
    mod.rs              # public API: run_source(), status()
    sources.rs          # sources.toml reader
    connection.rs       # connection config reader (schema_version 0)
    lifecycle.rs        # BrokerChild trait, NativeChild, WasmBrokerChild
    spawn.rs            # native child spawn (binary resolution, sandbox, init)
    http.rs             # production pipe/http handler (reuses host_support)
    routing.rs          # fact validation + dedup + eventlog write
    cursor.rs           # cursor management (transactional)
```

## CLI Integration

Add to existing `patina mother` subcommands:

```
patina mother run <name>    # run a source (fetch, validate, route)
patina mother sources       # show configured sources with status
```

### Sandbox Failure Surfacing

When a sandboxed child tries to contact an undeclared domain, the OS
sandbox blocks the connection with EPERM. The child sees a connection
error and returns `PipeError::Transient` or `PipeError::Fatal`. Mother
surfaces this in two places:

- **`patina mother run <name>` output:** The error message from the
  child includes the connection failure. Mother logs it with context:
  `[broker] github: Fatal — connection refused to api.slack.com
  (check child.toml [domains].allowed)`

- **`patina mother sources` output:** Shows last run status per source.
  A sandbox-blocked child shows as `last_run: error (Fatal)` with the
  error message. The `--sandbox-debug` flag or `PATINA_SANDBOX_DEBUG=1`
  env var re-runs without sandbox to confirm whether the sandbox is
  the cause.

## On-Scrape Scheduling

After `patina scrape` completes local work, trigger on-scrape sources.
The broker exposes `run_source()` — scrape calls it for each source
with `schedule = "on-scrape"`. Hourly/daily scheduling is
`continuous-operation` scope — the daemon calls the same function on
a timer.

## What's NOT In Scope

- **Scheduling daemon** — `continuous-operation` scope. The broker
  provides `run_source()`. The daemon calls it on schedule.
- **Fan-out optimization** — explicit decision: one spawn per source.
  Optimize when measured.
- **Stream mode lifecycle** — poll mode only. Stream children (always-on,
  health monitoring, restart on crash) is future work.
- **Data architecture changes** — events.db, patina.db, projections
  all stay as-is. The broker changes WHERE code lives, not how data
  flows.
- **Schema auto-installation** — the broker validates against installed
  schemas. Installing schemas from child manifests is future work.

## Belief Anchors

- [[mother-holds-connections-pipes-transform]] — Mother is the broker.
  She routes, she doesn't transform. Children produce facts, Mother
  routes them to destinations.
- [[pipe-protocol-is-transport-agnostic]] — BrokerChild abstracts
  over WASM and native. The broker doesn't care how facts arrived.
- [[host-proxied-io-is-the-security-model]] — credential delivery
  via pipe/initialize. Sandbox prevents direct vault access by
  children.
- [[temporal-layering-causes-drift]] — WASM bypass is acknowledged
  as drift risk. Set a deadline for unification.

## Resolved Questions

1. **Events.db per project vs shared.** The broker needs
   `open_events_db_at(path)` to write to a specific project's
   events.db. Add this as a thin wrapper around the existing
   `open_events_db()` pattern, accepting a project root path instead
   of relying on cwd. Implementation: same PRAGMAs, same schema,
   just parameterized path.

2. **Schema loading scope.** Resolved in §14: schemas load from the
   destination project's `.patina/schemas/`. Missing schemas produce
   a warning (once per schema per run) and facts pass through
   unvalidated. `patina schemas status` (future) gives ops visibility.

3. **WASM child fact routing.** Unchanged — gated by EC
   `wasm-routing-resolved`. See §5. Decision deferred to spec
   completion phase, not broker buildout.

4. **Connection config format.** Resolved in §8: `schema_version = 0`,
   minimal and rich formats both supported via shared `[connection]`
   section. patina-connect enriches, broker reads.

5. **Credential delivery.** Resolved in §9: Mother always injects
   for pipe/http. Raw token opt-in via `auth.requires_in_process_token`.
   Structured error on missing credential.

6. **Cursor storage.** Resolved in §10: dedicated `broker_cursors`
   table in events.db. Eliminates namespace coupling with scrape_meta.

7. **Content-hash dedup.** Resolved in §11: partial UNIQUE index on
   `content_hash`, `INSERT OR IGNORE`, `WriteResult.dedup_skipped`
   metric.

## Commits

1. `eventlog: add content_hash column with partial unique index` —
   Schema migration in ensure_events_db(). Backfill-safe (NULLs
   unaffected by unique constraint).

2. `broker: add connection config reader` — src/broker/connection.rs
   with ConnectionConfig, load_connection(). Validates schema_version.
   Supports both minimal and rich formats. Parse tests.

3. `broker: add sources.toml reader` — src/broker/sources.rs with
   SourceEntry, ProjectSources, scan_all_sources(). Parse tests.

4. `broker: add BrokerChild trait with NativeChild adapter` —
   src/broker/lifecycle.rs with trait + NativeChild. Error attribution
   tracing on on_fact. WasmBrokerChild stubbed (§5 gates full impl).

5. `broker: add production pipe/http handler` — src/broker/http.rs
   with build_production_handler(). Reuses host_support functions
   (pub(crate) visibility change). Per-child HTTP client caching.

6. `broker: add native child spawn with credential delivery` —
   src/broker/spawn.rs with spawn_native(), resolve_child_binary(),
   build_init_params(). Respects auth.requires_in_process_token.

7. `broker: add fact routing with content-hash dedup` —
   src/broker/routing.rs with validate_fact(), INSERT OR IGNORE.
   WriteResult with dedup_skipped metric.

8. `broker: add transactional cursor management` —
   src/broker/cursor.rs with write_facts_with_cursor(). Integration
   test: single transaction covers both facts and cursor.

9. `broker: add run_source() and status() public API` —
   src/broker/mod.rs orchestrating full flow.

10. `mother: add run and sources CLI commands` — Wire into
    MotherCommands.

11. `scrape: trigger on-scrape sources after local scrape` — Wire
    into scrape command.

## Key Files

- `src/broker/mod.rs` — public API (run_source, status)
- `src/broker/sources.rs` — sources.toml reader
- `src/broker/connection.rs` — connection config reader (schema_version 0)
- `src/broker/lifecycle.rs` — BrokerChild trait (WASM + native)
- `src/broker/spawn.rs` — native child spawn with sandbox
- `src/broker/http.rs` — production pipe/http handler
- `src/broker/routing.rs` — fact validation + dedup + content_hash
- `src/broker/cursor.rs` — transactional cursor management
- `src/mother/child.rs` — existing MotherChild trait (WASM)
- `src/plugin/internal/host_support.rs` — shared validation (pub(crate))
- `src/commands/mother/mod.rs` — CLI wiring
- `src/eventlog.rs` — events.db schema migration (content_hash column)
