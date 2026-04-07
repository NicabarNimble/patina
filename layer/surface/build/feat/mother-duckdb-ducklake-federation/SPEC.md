---
type: feat
id: mother-duckdb-ducklake-federation
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-135124-249836000
  tightened: 20260407-063612-748374000
blocked_by: []
beliefs:
  - "[[projects-are-sovereign-mother-coordinates]]"
  - "[[standards-are-storage-coordination-sits-above]]"
  - "[[core-verbs-standalone-mother-additive]]"
  - "[[five-boundaries-no-overlap]]"
  - "[[events-are-local-beliefs-federate]]"
related:
  - mother/src/state.rs
  - mother/src/registry.rs
  - mother/src/http_routes.rs
  - mother/src/protocol.rs
  - mother/src/daemon_lifecycle.rs
  - src/paths.rs
  - src/child/toy_host/lake.rs
  - layer/surface/build/feat/multiproject-belief-share/SPEC.md
  - layer/surface/build/feat/persona-lake-mvp1/SPEC.md
  - layer/surface/build/feat/child-construction-canon/SPEC.md
exit_criteria:

  - id: mdf1-federation-db
    text: "Mother opens `~/.patina/mother/federation.duckdb` at startup. `paths::mother::federation_db()` helper exists and returns the canonical path. DuckDB provides transactional WAL-backed recovery natively — no startup pragma needed (unlike SQLite). Mother daemon owns the file exclusively (same PID-lock pattern as state.db)."
    checked: false

  - id: mdf2-ducklake-required
    text: "Mother loads DuckLake extension at federation DB open: try `LOAD ducklake` first; if not installed, federation is unavailable with diagnostic `DuckLake extension not installed — run: patina mother federation install-extensions`. No implicit network fetch at boot. A one-time `patina mother federation install-extensions` command runs `INSTALL ducklake` and persists locally. Local project verbs are unaffected regardless."
    checked: false

  - id: mdf3-attach-registry
    text: "Mother scans `project_registry` in state.db, attempts `ATTACH` for each project's patina.db using deterministic alias `p_{project_uid}`. Records per-project attach status: attached, failed (with reason), stale (path missing on disk). Unreachable project is skipped — does not abort other attaches."
    checked: false

  - id: mdf4-schema-compat
    text: "On ATTACH, Mother reads schema version from project's `scrape_meta` table. Compatible (major version equals expected): attach normally. Any other major version (older or newer): skip with diagnostic `project {uid} schema v{n} incompatible, expected v{expected} — run patina scrape to upgrade`. Unreadable version: skip with error diagnostic."
    checked: false

  - id: mdf5-query-safety
    text: "Federation queries are read-only SQL subset only: SELECT statements, parameter binding for user-supplied values, enforced LIMIT (default 1000, max 10000), table allowlist populated from attach registry. No DDL, no DML, no raw string interpolation. Violations return structured error before execution."
    checked: false

  - id: mdf6-query-timeout
    text: "Federation queries that exceed timeout (default 30s, configurable) return a structured error with `timeout: true` and elapsed duration. No partial results — consumer retries with narrower scope. One response shape, deterministic."
    checked: false

  - id: mdf7-http-surface
    text: "Three federation HTTP routes added to Mother: `POST /api/federation/status`, `POST /api/federation/refresh`, `POST /api/federation/query`. Auth follows existing transport convention: Bearer token on TCP, file permissions on UDS (same as all other Mother routes). FederationPayload variants added to protocol.rs. Request/response JSON schemas defined and documented in spec."
    checked: false

  - id: mdf8-telemetry
    text: "Federation emits metrics via existing observe pattern (events.db, event_type=measure.metric): `mother:federation:refresh_latency_ms` (gauge), `mother:federation:attach_count` (gauge), `mother:federation:query_latency_ms` (gauge), `mother:federation:attach_failure` (counter), `mother:federation:query_error` (counter)."
    checked: false

  - id: mdf9-failure-matrix
    text: "Failure behavior is deterministic per the failure matrix in this spec: federation.duckdb can't open → federation unavailable; single ATTACH fails → skip project; DuckLake unavailable → federation unavailable; query timeout → error (no partial); stale project path → mark stale, skip on refresh. Each failure emits a diagnostic event."
    checked: false

  - id: mdf10-multiproject-unblock
    text: "The multiproject-belief-share spec `blocked_by` is updated to reference this spec's completed federation substrate. persona-lake-mvp1 `blocked_by` is similarly updated."
    checked: false

  - id: mdf11-proof
    text: "Proof: `cargo check --workspace -q`, `cargo test -q --lib -p mother -- federation` (package-scoped, not workspace filter), `patina mother start && patina mother federation status`. Tests are unit tests that don't require a running daemon."
    checked: false
---

# feat: Mother DuckDB + DuckLake Federation

## Problem

Patina's per-project SQLite model is the right ownership boundary, but there is
no federation substrate in Mother for cross-project analytic queries. The
greenfield data-platform spec explicitly queued DuckDB federation as future work,
and MVP 2 (`multiproject-belief-share`) and `persona-lake-mvp1` both need this
substrate for cross-project reads and lakehouse management.

## Goal

Build a Mother-owned DuckDB federation layer with DuckLake extension that sits
above project SQLite stores, without replacing project sovereignty. Provide
a concrete query surface with safety guarantees, deterministic failure behavior,
and telemetry that follows existing Mother observation conventions.

## Non-Goals

- Replacing per-project SQLite as source of truth.
- Per-project ACLs or authorization filtering in v1 (Mother is single-user,
  local-only; all registered projects are queryable by the authenticated user).
- Cross-Mother transport or P2P federation (downstream specs).
- Real-time sync or streaming queries.
- Persona-scoped query filtering (future: persona-lake-mvp1 or downstream).

## Resolved Decisions

### DuckLake is required, not optional

Federation needs lakehouse semantics (time travel, schema evolution, deletion
vectors) for belief and persona lanes. At federation DB open, Mother tries
`LOAD ducklake`. If the extension isn't installed locally, federation is
entirely unavailable with guidance to run a one-time install command. No
implicit network fetch at boot — offline-safe. Local project verbs are
unaffected regardless.

### Two data planes in one DuckDB process

```
federation.duckdb
├── SQLite ATTACH plane (native DuckDB capability, no extension needed)
│   └── Project patina.db files attached as p_{uid}
│   └── Cross-project belief queries, commit searches, pattern matching
│
└── DuckLake plane (requires ducklake extension)
    └── Lakehouse tables for persona lanes, belief lakes
    └── Time travel, schema evolution, compaction
```

Both planes run inside the same `federation.duckdb` process. SQLite ATTACH
is native DuckDB — no extension. DuckLake adds lakehouse tables on top.
`persona-lake-mvp1` builds on the DuckLake plane. `multiproject-belief-share`
query-responder reads from both planes.

### Read-only SQL subset for federation queries

Federation queries are `SELECT`-only with safety constraints:
- Parameter binding for all user-supplied values
- Enforced `LIMIT` (default 1000, max 10000)
- Table allowlist populated from attach registry metadata
- No DDL, no DML, no raw string interpolation
- Violations return structured error before execution

This is distinct from the `wasi:sql` toy host in `lake.rs` which accepts raw
SQL from sandboxed WASM children — different trust domain.

### Timeout returns error, never partial results

Federation query exceeding timeout returns:
```json
{
  "error": "federation_query_timeout",
  "timeout": true,
  "elapsed_ms": 30012,
  "limit_ms": 30000
}
```
No partial results. Consumer retries with narrower scope or higher limit.
One response shape, deterministic.

### Daemon exclusivity is the lock boundary

Mother daemon owns `federation.duckdb` exclusively, same pattern as `state.db`:
- PID file check at startup (daemon_lifecycle.rs)
- Alive → bail; stale → clean up and proceed
- DuckDB is single-writer — Mother is the only process
- Crash recovery via DuckDB WAL (automatic on next open)
- No file-level locking beyond daemon PID exclusivity

### Schema compatibility on ATTACH

Projects evolve independently. When Mother ATTACHes a project's patina.db:
- Read schema version from `scrape_meta` table
- Major version equals expected → attach normally
- Any other major version (older or newer) → skip with diagnostic:
  `"project {uid} schema v{n} incompatible, expected v{expected} — run patina scrape to upgrade"`
- Unreadable version → skip with error diagnostic
- No adapter views, no automatic migration — project must be re-scraped

### Authorization is single-user in v1

Mother daemon binds to localhost. On TCP: Bearer token auth (constant-time
comparison, token at `~/.patina/run/serve.token`). On UDS: file permissions
enforce access, no Bearer token needed (router runs with `require_auth=false`).
Both transports: if you can reach Mother, you can query all attached projects.
This is the same trust model as existing `/api/scry` and `/child/*` routes.

Per-project or per-persona filtering is a downstream concern — document as
explicit non-goal for v1.

## HTTP Surface

### POST /api/federation/status

Returns federation state: DuckLake loaded, attached projects, per-project status.

**Request:** empty body or `{}`

**Response:**
```json
{
  "federation": "available",
  "ducklake": "loaded",
  "projects_attached": 3,
  "projects_failed": 1,
  "projects_stale": 0,
  "projects": [
    { "uid": "2bdc808e", "alias": "p_2bdc808e", "status": "attached", "schema_version": 3 },
    { "uid": "a1b2c3d4", "alias": "p_a1b2c3d4", "status": "attached", "schema_version": 3 },
    { "uid": "deadbeef", "alias": "p_deadbeef", "status": "attached", "schema_version": 3 },
    { "uid": "00ff00ff", "alias": "p_00ff00ff", "status": "failed", "reason": "schema v1 incompatible, expected v3" }
  ]
}
```

**Error (federation unavailable):**
```json
{
  "federation": "unavailable",
  "reason": "DuckLake extension not installed — run: patina mother federation install-extensions"
}
```

### POST /api/federation/refresh

Re-scans project registry, re-attaches databases, updates schema compatibility.

**Request:** empty body or `{}`

**Response:** same shape as status, reflecting post-refresh state.

### POST /api/federation/query

Executes a read-only SQL query across attached project databases.

**Request:**
```json
{
  "sql": "SELECT project_uid, belief_id, text FROM beliefs WHERE facet = ?",
  "params": ["architecture"],
  "limit": 100,
  "timeout_ms": 15000
}
```

**Response (success):**
```json
{
  "columns": ["project_uid", "belief_id", "text"],
  "rows": [
    ["2bdc808e", "unix-philosophy", "One tool, one job..."],
    ["a1b2c3d4", "unix-philosophy", "One tool, one job..."]
  ],
  "row_count": 2,
  "truncated": false,
  "elapsed_ms": 42
}
```

**Response (timeout):**
```json
{
  "error": "federation_query_timeout",
  "timeout": true,
  "elapsed_ms": 15003,
  "limit_ms": 15000
}
```

**Response (validation error):**
```json
{
  "error": "federation_query_invalid",
  "reason": "only SELECT statements allowed"
}
```

## Failure Matrix

| Failure | Behavior | Diagnostic |
|---------|----------|------------|
| `federation.duckdb` can't open | Federation unavailable; local verbs unaffected | `mother:federation:open_failure` counter |
| DuckLake extension not installed | Federation unavailable entirely | Health reports `federation: unavailable` with install guidance |
| Single project ATTACH fails | Skip project; others still queryable | `mother:federation:attach_failure` counter + per-project reason |
| Project path stale (moved/deleted) | Mark stale in attach registry; skip on refresh | Per-project status = `stale` |
| Schema version incompatible | Skip project with diagnostic | Per-project status = `failed` with reason |
| Schema version unreadable | Skip project with error | Per-project status = `failed` with reason |
| Query timeout | Return timeout error; no partial results | `mother:federation:query_error` counter |
| Query validation failure | Return error before execution | `mother:federation:query_error` counter |
| Query runtime error | Return structured error | `mother:federation:query_error` counter |

Every failure emits a diagnostic event to the project's events.db (or Mother's
state.db for federation-level failures) following existing `measure.metric`
event conventions.

## Telemetry

Federation metrics follow the existing `observe_handle` pattern in
`mother/src/registry.rs` — inserted as events with `event_type = "measure.metric"`
and `source_id = "mother:federation:{metric_name}"`.

| Metric | Kind | When |
|--------|------|------|
| `mother:federation:refresh_latency_ms` | gauge | After each refresh cycle |
| `mother:federation:attach_count` | gauge | After refresh (number of successfully attached projects) |
| `mother:federation:query_latency_ms` | gauge | After each federation query |
| `mother:federation:attach_failure` | counter | Per ATTACH that fails |
| `mother:federation:query_error` | counter | Per query that fails (timeout, validation, runtime) |
| `mother:federation:open_failure` | counter | Federation DB failed to open |

Labels follow existing convention: `[["scope", "federation"], ["action", "{operation}"]]`.

## Phases

### Phase A — federation substrate

- Add `paths::mother::federation_db()` → `~/.patina/mother/federation.duckdb`
- Open federation DB at daemon startup with WAL mode
- Load DuckLake extension; report unavailable if missing
- Build attach registry from `project_registry` in state.db
- ATTACH project patina.db files with deterministic `p_{uid}` aliases
- Schema compatibility check on each ATTACH
- Emit startup telemetry (attach count, failures)

### Phase B — query surface

- Add `FederationPayload` variants to `mother/src/protocol.rs`
- Add 3 HTTP routes to `mother/src/http_routes.rs`
- Implement status, refresh, query handlers
- Query safety: SELECT-only validation, parameter binding, LIMIT enforcement,
  table allowlist
- Timeout handling (default 30s, configurable)
- Wire CLI: `patina mother federation status`, `patina mother federation query`,
  `patina mother federation install-extensions` (one-time DuckLake install)

### Phase C — integration and proof

- Update `multiproject-belief-share` blocked_by to reference this substrate
- Update `persona-lake-mvp1` blocked_by to reference this substrate
- Add failure matrix tests (each row in the matrix = one test)
- Add telemetry emission tests
- End-to-end: register 2+ projects, refresh, query across them

## Code Targets

| File | Change |
|------|--------|
| `src/paths.rs` | Add `mother::federation_db()` helper |
| `mother/src/daemon_bootstrap.rs` | Open federation DB after state.db, load DuckLake |
| `mother/src/state.rs` or new `mother/src/federation.rs` | Attach registry, schema compat, query execution |
| `mother/src/protocol.rs` | Add `FederationPayload` variants |
| `mother/src/http_routes.rs` | Add 3 federation routes |
| `src/commands/mother/` | Add `federation` CLI subcommands |

## Verification

```bash
cargo check --workspace -q
cargo test -q --lib -p mother -- federation

# UDS (default transport):
patina mother start
patina mother federation status

# TCP (alternative, for curl-based verification):
patina mother start --host 127.0.0.1
curl -s -H "Authorization: Bearer $(cat ~/.patina/run/serve.token)" \
  http://127.0.0.1:50051/api/federation/status | jq .
```

Note: `patina spec check` requires Mother running. Federation tests are unit
tests that mock the DuckDB connection — they don't require a running daemon.

## Build Readiness

Ready to start. DuckDB crate pinned to v1.5.1 (crate 1.10501.0). DuckLake
extension available. All blocker specs complete. Existing Mother infrastructure
(PID lifecycle, WAL, observation, HTTP routing, project registry) provides
the foundation.
