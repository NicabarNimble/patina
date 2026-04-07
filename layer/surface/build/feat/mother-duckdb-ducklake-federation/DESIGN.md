# Design: Mother DuckDB + DuckLake Federation

## Why This Design

Mother coordinates cross-project knowledge but had no query substrate spanning
multiple project databases. Each project has its own SQLite stores
(`patina.db`, `events.db`, `runtime.db`) under `~/.patina/mother/projects/{uid}/`.
DuckDB can ATTACH SQLite files natively and run cross-database queries. DuckLake
adds lakehouse semantics (time travel, schema evolution) on top for persona and
belief lanes.

The design follows [[standards-are-storage-coordination-sits-above]]: SQLite is
the storage foundation, DuckDB + DuckLake is the coordination layer above it.
Federation never replaces project sovereignty — it reads from project stores,
never writes to them.

## Build Target

Phase A (substrate): open federation DB, load DuckLake, build attach registry,
schema compatibility, telemetry. Phase B (query surface): HTTP routes, query
safety, timeout handling. Phase C (integration): wire downstream specs, full
failure matrix tests.

## Module Layout

```
src/commands/mother/federation.rs    ← Federation startup, attach registry, runtime
src/commands/mother/daemon.rs        ← Wires federation into daemon lifecycle
src/commands/mother/mod.rs           ← Module registration
src/paths.rs                         ← paths::mother::federation_db() helper
src/eventlog.rs                      ← set_schema_version() for scrape_meta
mother/src/http_api.rs               ← HealthDetails federation fields
```

Federation lives in `src/commands/mother/federation.rs`, not in `mother/src/`,
because it depends on `patina::paths`, `patina::eventlog`, and `patina::mother`
— it's a command-layer orchestration module, not a core Mother library. Same
pattern as `src/commands/mother/daemon.rs`.

## Data Flow

```
Daemon startup
  │
  ├── Open ~/.patina/mother/state.db (existing)
  │
  ├── Open ~/.patina/mother/federation.duckdb (new)
  │     ├── LOAD ducklake
  │     │     └── fail → federation unavailable, daemon continues
  │     │
  │     ├── Read project_registry from state.db
  │     │
  │     └── For each project:
  │           ├── Resolve ~/.patina/mother/projects/{uid}/patina.db
  │           ├── Check path exists → stale if missing
  │           ├── Read scrape_meta.schema_version → major must equal 3
  │           ├── ATTACH '{path}' AS p_{uid} (TYPE SQLITE)
  │           └── Record status: attached / failed / stale
  │
  └── Start HTTP server with federation status in health endpoint
```

## Key Types

```rust
// Federation availability (available or unavailable with reason)
pub enum FederationAvailability { Available, Unavailable { reason: String } }

// Per-project attach state
pub enum ProjectAttachState { Attached, Failed, Stale }

// Per-project attach status with reason and schema version
pub struct ProjectAttachStatus {
    pub uid: String,
    pub alias: String,        // deterministic p_{uid}
    pub state: ProjectAttachState,
    pub reason: Option<String>,
    pub schema_version_major: Option<u32>,
}

// Overall federation status
pub struct FederationStatus {
    pub availability: FederationAvailability,
    pub ducklake_loaded: bool,
    pub projects: Vec<ProjectAttachStatus>,
}

// Runtime holder — owns the DuckDB connection for daemon lifetime
pub struct FederationRuntime {
    _connection: Option<duckdb::Connection>,
    status: FederationStatus,
}
```

`FederationRuntime` owns the DuckDB connection. The leading `_` on `_connection`
is intentional — Phase A keeps it alive but doesn't expose query methods.
Phase B adds query execution through this connection.

## Schema Version Contract

`scrape_meta` table has a `schema_version` key (written by `set_schema_version()`
in `src/eventlog.rs:171` during scrape initialization). Current version: `"3"`.

Federation checks major version on ATTACH:
- Major equals `EXPECTED_PROJECT_SCHEMA_MAJOR` (3) → attach
- Any other major (including 0 = missing key from pre-migration projects) → skip
  with diagnostic: `"project {uid} schema v{n} incompatible, expected v3 — run patina scrape to upgrade"`

This is symmetric: older and newer majors both fail. No adapter views.

## Telemetry Contract

Federation emits metrics to events.db following the `observe_handle` pattern
in `mother/src/registry.rs:74`:

| Metric | Kind | source_id | When |
|--------|------|-----------|------|
| `open_failure` | counter | `mother:federation:open_failure` | DB open, DuckLake load, or registry read fails |
| `attach_failure` | counter | `mother:federation:attach_failure` | Per-project ATTACH fails |
| `attach_count` | gauge | `mother:federation:attach_count` | After all ATTACHes complete |

Labels: `[["scope", "federation"], ["action", "{step}"]]` where step identifies
the specific failure point (e.g. `open_db`, `load_ducklake`, `schema_incompatible`).

Phase B adds: `query_latency_ms` (gauge), `query_error` (counter).

## Health Endpoint Integration

`HealthDetails` in `mother/src/http_api.rs:60` was extended with:
- `federation_available: bool`
- `federation_reason: Option<String>`
- `federation_ducklake_loaded: bool`
- `federation_projects_attached: usize`
- `federation_projects_failed: usize`
- `federation_projects_stale: usize`

Populated from `FederationStatus` in daemon's `ServerState`.

## Resolved Decisions

1. **Federation module location** — `src/commands/mother/federation.rs` not
   `mother/src/federation.rs`. It orchestrates across patina crate + mother
   crate boundaries. Same pattern as daemon.rs.

2. **Connection ownership** — `FederationRuntime` holds `Option<duckdb::Connection>`.
   `None` when federation is unavailable. Kept alive for daemon lifetime so
   ATTACHed databases remain accessible.

3. **Schema version in scrape_meta** — Added `set_schema_version()` to eventlog.rs
   initialization path. Called during DB init and scrape finalization. Missing key
   returns major 0 (pre-migration project).

4. **DuckLake loading is offline-safe** — `LOAD ducklake` only, no `INSTALL` at
   boot. Extension must be pre-installed. Phase B adds `install-extensions` command.

5. **Metric emission** — Opens events.db directly (not through Mother IPC) because
   federation startup runs before the HTTP server is ready. Same direct-write
   pattern used by child observation in registry.rs.

## Phase B Targets

When Phase B begins, these are the direct code targets:

| File | Change |
|------|--------|
| `mother/src/protocol.rs` | Add `FederationPayload` variants (status, refresh, query) |
| `mother/src/http_routes.rs` | Add 3 routes: `/api/federation/{status,refresh,query}` |
| `src/commands/mother/federation.rs` | Add `query()`, `refresh()` methods on `FederationRuntime` |
| `src/commands/mother/federation.rs` | Query safety: SELECT-only validation, LIMIT enforcement, table allowlist |
| `src/commands/mother/daemon.rs` | Wire federation query through ServerState |
| `src/commands/mother/mod.rs` | Add `federation` CLI subcommand (status, query, install-extensions) |

The `_connection` field drops its `_` prefix when Phase B exposes query methods.

## Verification Plan

Phase A (current):
```bash
cargo check --workspace -q
cargo test -q --lib -p mother -- federation
cargo test -q --lib -- eventlog::tests::test_initialize_sets_schema_version
```

Phase B:
```bash
patina mother start
patina mother federation status
patina mother federation query "SELECT count(*) FROM p_2bdc808e.beliefs"
```

## Open Questions

None for Phase A. Phase B will resolve:
- Table allowlist population strategy (auto from ATTACH metadata vs explicit config)
- Whether `refresh` detaches stale projects or just updates status
- DuckLake table creation for persona lanes (may defer to persona-lake-mvp1)
