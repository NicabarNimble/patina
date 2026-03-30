# Design: Mother Enterprise Hardening

## Why This Design

Mother is Patina's local daemon — the infrastructure that coordinates children, manages project state, and mediates secrets. The greenfield-mother-patina-data-platform work gave her clean per-project database ownership. Now she needs to be reliable enough to run unattended as always-on infrastructure.

This is not a feature build. Mother's surface (7 HTTP routes, custom microserver, thread-per-connection) is lean and correct. The design hardens internals: error handling, resource bounding, observability, lifecycle management. Same surface, bulletproof internals.

Gjengset's principle throughout: library code never panics on bad input. Errors propagate via Result. Callers decide how to handle failures. Panic is reserved for programming bugs (unreachable states), never for runtime conditions (bad input, missing files, locked databases).

## Architecture: What Mother Is

```
Mother (local daemon, ~/.patina/mother/)
│
├── HTTP Transport (microserver.rs — 150 lines, don't touch)
│   ├── UDS: ~/.patina/run/serve.sock (default, file-based auth)
│   └── TCP: 127.0.0.1:{port} (optional, bearer token auth)
│
├── Router (http_routes.rs — 7 routes)
│   ├── GET  /health          → public, deep health data
│   ├── GET  /version          → public
│   ├── POST /api/scry         → auth required
│   ├── GET  /secrets/cache    → auth required
│   ├── POST /secrets/cache    → auth required
│   ├── POST /secrets/lock     → auth required
│   └── *    /child/{name}/*   → auth required, routes to ChildRegistry
│
├── ChildRegistry (registry.rs)
│   └── Vec<Arc<RwLock<Box<dyn KnowledgeChild>>>>
│       └── handle() → observe_handle() wraps with boundary metrics
│
├── Heartbeat Thread (daemon_heartbeat.rs)
│   └── Every 60s: drain → tick → handle for each child
│   └── NEW: every 5min: WAL checkpoint on project databases
│
├── Per-project Databases (from data platform)
│   └── ~/.patina/mother/projects/{uid}/
│       ├── events.db, patina.db, runtime.db
│
├── Mother State (state.db)
│   └── Sessions, project registry, belief/graph mutations
│
└── Lifecycle (daemon_lifecycle.rs)
    ├── PID file (0600)
    ├── Signal handlers (SIGINT/SIGTERM)
    └── NEW: stale PID detection, graceful drain, supervisor integration
```

## What the data platform enables for hardening

The greenfield-mother-patina-data-platform gave Mother:

- **Project registry** — Mother can enumerate all known projects. This enables WAL checkpoint scheduling (MEH-G5) and database size reporting in health (MEH-G7).
- **Known database paths** — `paths::mother::projects::{events_db, patina_db, runtime_db}(uid)`. Mother knows where every database lives. Checkpoint scheduling and health reporting read these paths.
- **Per-project isolation** — Each project's databases are independent. A corrupt WAL in one project can't affect another.
- **Clean init sequence** — Project registration happens on first use. Mother's startup has a clear lifecycle to guard with stale PID detection (MEH-G9).

## Design Decisions

### 1. Bounded thread pool, not async

Mother is sync-first (`patina-identity` architectural invariant #6). The accept loop spawns threads. Today: unbounded `thread::spawn`. After: bounded pool.

Implementation: semaphore-guarded spawn. An `Arc<Semaphore>` with `max_connections` permits. Each connection acquires a permit before spawning. If no permits available, return 503 immediately on the connection. When the handler thread finishes, the permit is released.

Why not a channel-based worker pool: simpler. Workers don't need to be pre-spawned — Rust threads are cheap to create. The semaphore bounds concurrency without pre-allocating resources.

Why not async: Mother's design is sync-first. Converting the accept loop to tokio would infect the entire codebase. The semaphore approach keeps the sync model.

Default pool size: 16. Rationale: single-user daemon, local machine. 16 concurrent connections is plenty for multiple CLI commands + one daemon heartbeat + headroom for concurrent interface agents.

### 2. catch_unwind as safety net, not design

Request handlers should never panic. They return `HttpResponse` which is always constructable. But defense in depth: wrap the handler call in `catch_unwind`. If it panics:

```rust
let response = match std::panic::catch_unwind(AssertUnwindSafe(|| handler(request))) {
    Ok(response) => response,
    Err(panic) => {
        // Log the panic payload (if it's a string or &str)
        tracing::error!(?panic, "request handler panicked");
        json_error(500, "internal server error")
    }
};
```

The accept loop is unaffected — the panic was caught in the spawned thread. The client gets a 500 instead of a dropped connection.

### 3. Structured logging replaces eprintln

Mother currently logs via `eprintln!`. This works when running interactively but fails under supervision (launchd captures stdout/stderr but doesn't structure it).

Replace with `tracing` (already in the workspace dependencies):

```rust
// Daemon bootstrap, before any logging
let log_path = paths::mother::logs_dir().join("mother.jsonl");
let file = std::fs::File::create(&log_path)?;
let subscriber = tracing_subscriber::fmt()
    .json()
    .with_writer(file)
    .with_max_level(tracing::Level::INFO)
    .finish();
tracing::subscriber::set_global_default(subscriber)?;
```

Log levels:
- `INFO`: lifecycle events (startup, shutdown, child loaded, project registered)
- `WARN`: degradation (checkpoint failed, child health degraded, stale PID cleaned)
- `ERROR`: failures (handler panic, database open failed, auth rejection)

### 4. WAL checkpoint in heartbeat

The heartbeat thread already runs every 60s for child tick/drain cycles. Add a checkpoint counter:

```rust
let mut checkpoint_counter = 0u64;
loop {
    thread::sleep(Duration::from_secs(60));
    // Child lifecycle
    registry.run_knowledge_cycles(&runtime, "mother-heartbeat")?;
    // WAL checkpoint every 5 heartbeats (5 minutes)
    checkpoint_counter += 1;
    if checkpoint_counter % 5 == 0 {
        checkpoint_project_databases();
    }
}
```

`checkpoint_project_databases()` enumerates `~/.patina/mother/projects/*/`, opens each database file, runs `PRAGMA wal_checkpoint(PASSIVE)`, logs results. PASSIVE mode doesn't block writers — it checkpoints what it can without waiting.

### 5. Graceful shutdown with drain

Current signal handler: cleanup PID + socket, exit immediately. In-flight requests are killed.

New design:

```
SIGINT/SIGTERM received
  │
  ├── Set AtomicBool SHUTDOWN = true
  ├── Log "shutting down, draining in-flight requests"
  │
  ├── Accept loop: check SHUTDOWN before each accept()
  │   └── If true: stop accepting, break out of loop
  │
  ├── Wait up to 5s for in-flight handler threads to complete
  │   └── (semaphore drain: wait for all permits to be returned)
  │
  ├── Checkpoint all project databases (TRUNCATE mode — blocks, ensures clean state)
  ├── Stop heartbeat thread (join)
  ├── Remove PID file
  ├── Remove socket file
  └── Exit 0
```

The drain window ensures clients get their responses. The final checkpoint ensures WAL files are clean. The exit is orderly.

### 6. launchd integration

`patina mother install` generates:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.patina.mother</string>
    <key>ProgramArguments</key>
    <array>
        <string>/path/to/patina</string>
        <string>mother</string>
        <string>start</string>
        <string>--uds</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>~/.patina/mother/logs/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>~/.patina/mother/logs/stderr.log</string>
    <key>WorkingDirectory</key>
    <string>~</string>
</dict>
</plist>
```

`KeepAlive=true` means launchd restarts Mother automatically on crash. Combined with MEH-G2 (panic recovery), crashes should be rare — but if they happen, recovery is automatic.

`patina mother install`: write plist, `launchctl load`.
`patina mother uninstall`: `launchctl unload`, delete plist.

### 7. Deep health endpoint

Current `/health` returns minimal JSON. After hardening:

```json
{
  "status": "healthy",
  "uptime_seconds": 3621,
  "children": {
    "count": 3,
    "health": {
      "file-system-monitor": "healthy",
      "content-extractor": "healthy",
      "lakehouse-catalog": "degraded: stale checkpoint"
    }
  },
  "projects": {
    "registered": 2,
    "active": {
      "2bdc808e": {
        "events_db_bytes": 11534336,
        "patina_db_bytes": 206569472,
        "runtime_db_bytes": 54272
      }
    }
  },
  "state_db_bytes": 3276800,
  "heartbeat_interval_secs": 60,
  "last_checkpoint_at": "2026-03-30T20:45:00Z",
  "pool": {
    "max_connections": 16,
    "active_connections": 2
  }
}
```

This is Mother observing herself at her own boundary. Same principle as children being observed by Mother — Mother is observed by whoever calls `/health` (CLI, monitoring, supervisor).

### 8. Constant-time token comparison

`subtle` crate is a transitive dependency via `age` (which Mother already depends on). Use `subtle::ConstantTimeEq`:

```rust
use subtle::ConstantTimeEq;

fn check_auth(&self, request: &HttpRequest) -> bool {
    request
        .header("Authorization")
        .map(|h| {
            let expected = format!("Bearer {}", self.token);
            h.as_bytes().ct_eq(expected.as_bytes()).into()
        })
        .unwrap_or(false)
}
```

No new dependency. Eliminates timing side channel.

## Direct Code Targets

| File | Gate | Change |
|------|------|--------|
| mother/src/state.rs:1145+ | G1 | Fix 2 test methods to use new_with_project() |
| mother/src/http_daemon.rs:100-138 | G2, G3, G6 | catch_unwind, semaphore pool, shutdown flag |
| mother/src/daemon_runner.rs:26-80 | G3 | Pass pool config through launch structs |
| mother/src/daemon_bootstrap_config.rs | G3 | Add max_connections field |
| mother/src/daemon_heartbeat.rs | G5, G6 | WAL checkpoint counter, shutdown check |
| mother/src/daemon_lifecycle.rs | G6, G9 | Drain window, stale PID check |
| mother/src/http_routes.rs:77-82 | G10 | Constant-time token compare |
| mother/src/http_api.rs (health handler) | G7 | Deep health response |
| mother/src/secrets_authority_backend/vault.rs:158 | G11 | expect → bail |
| mother/src/secrets_authority_backend/encrypted_file.rs:305 | G11 | expect → ok_or_else |
| All mother/src/*.rs eprintln! calls | G4 | Replace with tracing macros |
| src/commands/mother/ | G8 | Install/uninstall subcommands |

## Verification Plan

1. `cargo test -q -p mother` — all mother tests pass (target: zero failures)
2. `cargo check --workspace -q` — workspace compiles
3. `cargo test -q --lib` — full lib tests pass
4. Manual: `patina mother start` + `curl /health` + `patina mother stop`
5. Manual: launchd install/uninstall cycle
6. Confirm: `rg "eprintln!" mother/src/ --type rust` zero in non-test code
7. Confirm: `rg "\.expect\(" mother/src/ --type rust` non-test hits are startup-only

## Future: DuckDB Cross-Project Federation Layer

After Mother is hardened, the next capability is cross-project queries via DuckDB. This is a separate spec (`mother-duckdb-federation` or similar) that builds on both the data platform and hardening work.

### What DuckDB enables

Mother can enumerate registered projects (from data platform, project_registry in state.db). DuckDB can ATTACH multiple SQLite databases and query across them:

```sql
-- Mother opens DuckDB in-memory, ATTACHes project SQLite databases
ATTACH 'projects/abc123/patina.db' AS p1 (TYPE SQLITE);
ATTACH 'projects/def456/patina.db' AS p2 (TYPE SQLITE);

-- Cross-project belief discovery
SELECT 'abc123' as project, id, statement, facets, entrenchment
FROM p1.beliefs WHERE facets LIKE '%rust%'
UNION ALL
SELECT 'def456', id, statement, facets, entrenchment
FROM p2.beliefs WHERE facets LIKE '%rust%';

-- Cross-project event analytics
ATTACH 'projects/abc123/events.db' AS e1 (TYPE SQLITE);
SELECT event_type, COUNT(*) FROM e1.eventlog GROUP BY event_type;
```

### What it would build

- **DuckDB connection management** in Mother — open in-memory, ATTACH project databases on demand
- **Cross-project scry** — unified semantic search across all registered projects
- **Belief discovery** — find relevant beliefs by facet match for new projects (enables multiproject-belief-share MVP 2)
- **Project health dashboard** — aggregate metrics across all projects
- **Ref repo query** — ATTACH ref repo databases (at `~/.patina/cache/repos/*/`) alongside project databases

### Why it depends on hardening

DuckDB queries are analytical — they can be slow on large databases. The bounded thread pool (MEH-G3) prevents a long-running DuckDB query from exhausting capacity. Structured logging (MEH-G4) makes query performance visible. WAL checkpointing (MEH-G5) ensures DuckDB reads clean SQLite state. Graceful shutdown (MEH-G6) ensures DuckDB connections are closed properly.

### The standard pattern

Per `standards-are-storage-coordination-sits-above`: SQLite is the portable storage unit (per-project, sovereign). DuckDB is the coordination layer (queries across many SQLite databases). Neither replaces the other. DuckDB is read-only against project data — it never writes to project SQLite files.

This is the same pattern as Git (storage) → jj (coordination), and the same pattern Cloudflare uses with per-Durable-Object SQLite databases queryable via their analytics platform.

## Build Readiness

Mother is 11K lines. The surface is correct. The internals need 12 targeted improvements — each one independent, each one testable. No gate changes the external interface. After this spec, Mother is the same daemon that stays up, handles errors, observes herself, and restarts automatically.
