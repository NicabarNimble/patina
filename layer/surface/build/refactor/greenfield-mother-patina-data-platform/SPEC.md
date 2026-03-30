---
type: refactor
id: greenfield-mother-patina-data-platform
status: ready
created: 2026-03-30
blocks:
- multiproject-belief-share
sessions:
  origin: 20260330-083255-177610000
related:
- mother/src/registry.rs
- mother/src/state.rs
- src/eventlog.rs
- src/measure.rs
- src/child/internal/host_support.rs
- src/commands/scrape/
- src/commands/oxidize/
- src/commands/scry/
- src/commands/eval/
- src/commands/measure/
- src/commands/assay/
- src/commands/session/
- src/session/
- layer/surface/build/refactor/greenfield-mother-clean-continued/
- layer/surface/build/feat/folder-text-to-parquet/
- layer/surface/build/feat/child-construction-canon/
beliefs:
- '[[standards-are-storage-coordination-sits-above]]'
- '[[observation-at-the-boundary]]'
- '[[children-have-agency-toys-are-capabilities]]'
- '[[four-roles-no-overlap]]'
- '[[mother-is-the-daemon]]'
- '[[core-verbs-standalone-mother-additive]]'
- '[[if-its-patina-its-git]]'
- '[[eventlog-is-truth]]'
exit_criteria:
- id: gmdp1-dead-databases-removed
  text: cortex.db, navigation.db, and project-level graph.db code references deleted. Zero hits in codebase.
  checked: true
- id: gmdp2-per-project-isolation
  text: Mother stores per-project databases at ~/.patina/mother/projects/{project_uid}/. Each project gets its own events.db, runtime.db, and patina.db. Two projects with the same child name have independent state.
  checked: true
- id: gmdp3-project-registration
  text: Projects register with Mother on first use (patina init or first command). Mother creates projects/{uid}/ directory structure. Registration is idempotent. Mother can enumerate registered projects.
  checked: true
- id: gmdp4-event-writing-unified
  text: 'One event-writing code path for all producers: scrape, child metrics (host_support), Mother boundary metrics (registry), sessions, measure. No duplicated schema DDL. No duplicated open_events_db(). All writers resolve path via project_uid to Mother''s project directory.'
  checked: true
- id: gmdp5-cli-uses-mother-paths
  text: CLI commands that read or write databases resolve paths through project_uid to ~/.patina/mother/projects/{uid}/. The ~56 direct events.db call sites and ~50 direct patina.db call sites use Mother-resolved paths. CLI opens local SQLite files directly — no daemon round-trip required for core protocol operations.
  checked: true
- id: gmdp6-scrape-oxidize-scry-adapted
  text: Scrape writes projections to Mother-scoped patina.db. Oxidize reads from Mother-scoped patina.db, writes embeddings to working-copy-local cache. Scry reads from both. Pipeline produces identical results to current system.
  checked: true
- id: gmdp7-observation-unified
  text: 'Mother boundary metrics for children flow through the same event path as all other producers. New events use source_id taxonomy per INV-4: ''mother:*'' for boundary observation, ''child:*'' for child events, ''core'' for scrape/measure. Registry''s duplicated open_events_db() eliminated. Query code handles both legacy ''plugin:*'' and new ''child:*'' prefixes.'
  checked: true
- id: gmdp8-project-directory-clean
  text: No SQLite databases in project .patina/local/data/. Remaining local contents (embeddings, forge cache, logs, interface-sessions, backups) are all rebuildable or runtime-only. See INV-6 for full inventory.
  checked: true
- id: gmdp9-mother-state-split
  text: Mother-level state (sessions, project registry, belief/graph mutations, lake cursors) lives in state.db. Per-project child state (keyvalue, offsets, tasks) lives in projects/{uid}/runtime.db. Current global runtime.db split along this boundary.
  checked: true
- id: gmdp10-persona-belief-store
  text: Persona beliefs have a designated store at ~/.patina/mother/persona/{persona_uid}/. Structure does not block future belief propagation across projects (multiproject-belief-share).
  checked: true
- id: gmdp11-existing-data-migrated
  text: One-time migration moves existing events.db and patina.db to Mother project directory. User informed of what moved and what was deleted.
  checked: true
- id: gmdp12-workspace-compiles-tests-pass
  text: cargo check --workspace -q. cargo test -q --lib. WASM integration tests pass. patina scrape && patina scry returns results from Mother-scoped databases.
  checked: true
---
# refactor: Greenfield Mother Data Platform

> Build Mother as Patina's local data platform. Per-project SQLite databases move from the project directory to Mother's project directory, scoped by project_uid. The CLI opens these local files directly for core protocol operations — no daemon round-trip. Mother daemon adds coordination: child orchestration, boundary observation, cross-project queries via DuckDB. Projects become plain text (layer/ + config). This is the foundation for multiproject-belief-share (MVP 2) and eventual multi-Mother federation via iroh.

## Status

- Current status: ready to complete
- Exit criteria: 12/12 checked
- Remaining work: archive via `patina spec complete`

## Implementation Order

1. GMDP-G1 dead database cleanup
2. GMDP-G2 path authority and uid validation
3. GMDP-G3 project registration
4. GMDP-G4 unified event writing path
5. GMDP-G5 runtime/state split
6. GMDP-G6 pipeline rewires to Mother path
7. GMDP-G7 remaining consumer rewires and taxonomy cleanup
8. GMDP-G8 project directory cleanup
9. GMDP-G9 persona store structure
10. GMDP-G10 migration protocol implementation + execution
11. GMDP-G11 migration verification on real project data
12. GMDP-G12 full verification and smoke tests

## Problem

Patina's data layer grew around a monolithic CLI that opens databases directly from 100+ call sites. Five problems:

**1. No ownership boundary.** The CLI opens `events.db` from ~56 call sites, `patina.db` from ~50, `runtime.db` from ~24 — all direct `Connection::open()`. No single owner, no service layer, no path authority.

**2. events.db is irreplaceable data in a "rebuildable" directory.** Lives at `.patina/local/data/events.db` — gitignored, claimed rebuildable by `patina rebuild`. But events.db is the project's autobiography. It cannot be rebuilt. The location contradicts its importance.

**3. Mother's registry duplicates eventlog infrastructure.** `mother/src/registry.rs` (ftp4, commit `2e4257ed`) copies the events.db path and full schema DDL because the dependency graph prevents Mother from importing `patina-ai`'s eventlog module. Schema drift: missing `busy_timeout`, no migration path, no `content_hash` dedup index.

**4. Child state has no project dimension.** `runtime.db` keys child state on `(plugin_name, key)`. Two projects using the same child collide. Blocks multi-project orchestration needed for multiproject-belief-share (MVP 2).

**5. No foundation for cross-project queries.** multiproject-belief-share needs Mother to query beliefs across project databases. Current layout has each project's data locked in its own `.patina/local/data/`. No clean way to enumerate or ATTACH project databases.

## Core Value Anchors

- `patina-identity`: "The binary is the pipeline. The layer is the product." Projects own `layer/`; Mother manages databases as local infrastructure.
- `four-roles-no-overlap`: Mother = infrastructure, Projects = development zone. Databases are infrastructure.
- `core-verbs-standalone-mother-additive`: scrape, scry, assay, oxidize work as CLI tools against local files. Mother adds coordination — she doesn't gate core operations.
- `standards-are-storage-coordination-sits-above`: SQLite per-project is the portable storage unit. DuckDB (future) ATTACHes across them for federation queries. Git is the history unit. Each layer respects the one below.
- `observation-at-the-boundary`: One event path for all producers. Mother observes at boundaries, writes through the same system as everything else.
- `if-its-patina-its-git`: layer/ is git-tracked knowledge. events.db is the experience record. They're both sources of truth — but events.db belongs in Mother's infrastructure, not pretending to be derived project state.
- `eventlog-is-truth`: The append-only eventlog is canonical. All projections (patina.db) are materialized views — derived, rebuildable.

## Goal

After this refactor:
- Per-project databases live at `~/.patina/mother/projects/{project_uid}/` — events.db, patina.db, runtime.db
- CLI resolves paths via project_uid and opens local SQLite directly — no daemon required for core protocol
- Mother daemon adds: child orchestration, boundary observation, cross-project DuckDB queries, project registry
- Project directories are plain text + working-copy cache
- One event-writing path — no duplication
- Child state is project-scoped — no collisions
- Foundation laid for multiproject-belief-share and eventual iroh-based federation

## Non-Goals

- Building the DuckDB cross-project federation query layer (future, builds on this foundation)
- Building belief propagation / adoption across projects (that's multiproject-belief-share, MVP 2)
- Multi-Mother federation or iroh integration (future)
- Changing scrape/oxidize/scry pipeline logic (only where data lives)
- Redesigning the event schema or eventlog table format
- Changing the child runtime model, WIT interfaces, or SDK
- Building elaborate data protection (single user, accept migration risk with awareness)
- Extracting shared crates — Mother owns database code, CLI resolves paths and opens files

## What You Will Lose During Migration

**User decision required before gmdp11 executes.**

- **events.db** (11MB): Moves from `.patina/local/data/` to `~/.patina/mother/projects/{uid}/`. Same file, new path. Original stays until confirmed.
- **patina.db** (170MB): Moves to Mother. Fully rebuildable via `patina scrape && patina oxidize`.
- **Dead databases deleted**: cortex.db (0 bytes), project-level graph.db (0 bytes). Nothing lost.
- **Direct path assumption**: Code that assumed `.patina/local/data/events.db` will resolve through project_uid instead. This is the change — not what's stored, but where it's found.

## Prior Art — What the Giants Teach

The federated sovereign-node pattern is proven across multiple mature systems:

| System | Storage unit | Replication unit | Coordination | Lesson for Patina |
|--------|-------------|-----------------|--------------|-------------------|
| **LiveStore** | SQLite (client-local) | Events | Sync provider | Events are source of truth; SQLite materializes locally |
| **Matrix** | SQLite (server-local) | Room events (signed) | Federation API | Events replicate, state materializes per server; signatures prove origin |
| **iroh** | Blobs (content-addressed) | Blobs + doc entries | QUIC p2p | Transport layer connects nodes; protocols compose on top |
| **DuckLake** | Parquet files | Catalog metadata | DuckDB catalog | Query across files without centralizing; catalog coordinates |
| **Git** | Objects (content-addressed) | Pack files | Push/pull | Each clone is sovereign; objects replicate, views materialize locally |

**Common pattern**: storage is local and sovereign. Replication syncs artifacts (events, objects, blobs), not databases. Each node materializes its own view. Coordination sits above, never replaces.

For Patina: events.db is per-machine autobiography (never federates). Beliefs federate via git (today) and iroh (future). patina.db is a local materialized view (rebuilt from scrape). Mother coordinates but doesn't centralize.

## Current State

### Database files on disk

```
.patina/local/data/
├── events.db        (11MB, irreplaceable)
├── patina.db        (170MB, rebuildable)
├── cortex.db        (0 bytes, dead)
├── graph.db         (0 bytes, dead)
├── embeddings/      (working-copy cache, rebuildable)
└── forge/           (forge cache)

~/.patina/mother/
├── runtime.db       (3.2MB, child state + sessions — NOT project-scoped)
└── graph.db         (569KB, cross-project belief graph)

~/.patina/lakes/*/lake.duckdb  (DuckLake, per-lake)
```

### Direct database consumers (will switch to Mother-resolved paths)

**events.db** (~56 call sites across 23 files):
- `src/eventlog.rs` (12) — canonical open/ensure/migrate
- `src/commands/scry/internal/logging.rs` (6) — query event logging
- `src/child/internal/host_support.rs` (4) — WASM child metrics/facts
- `src/commands/session/internal.rs` (4) — session events
- `src/mother/broker/mod.rs` (4) — broker routing
- `mother/src/registry.rs` (3) — boundary metrics (DUPLICATED)
- 17 other files (1-3 each)

**patina.db** (~50 call sites across 21 files):
- `src/commands/scrape/` (14) — write projections
- `src/commands/eval/` (8) — ATTACH cross-database queries
- `src/eventlog.rs` (7) — schema init
- `src/commands/measure/` (6) — ATTACH queries
- `src/commands/assay/` (5) — belief/code queries
- `src/commands/scry/` (6) — result enrichment
- 5 other files (1-2 each)

**runtime.db** (~24 call sites across 10 files):
- `src/session/internal/live.rs` (7) — session lifecycle
- `src/interface/internal/checkin.rs` (4) — interface management
- `src/child/toy_host/v2.rs` (3) — child state/offsets
- `mother/src/state.rs` (2) — schema/open
- 6 other files (1-2 each)

## Target State

### Directory layout

```
project/
├── .patina/
│   ├── config.toml              ← project identity + settings
│   ├── uid                      ← stable 8-hex project identity
│   ├── oxidize.yaml             ← embedding recipe
│   └── local/                   ← gitignored
│       └── cache/
│           └── embeddings/      ← .usearch + .safetensors (working-copy-specific)
├── layer/                       ← THE PRODUCT (git-tracked)
└── (code)

~/.patina/mother/
├── state.db                     ← Mother lifecycle (sessions, project registry,
│                                   belief/graph mutations, lake cursors)
├── graph.db                     ← cross-project belief graph (unchanged)
│
├── projects/{project_uid}/
│   ├── events.db                ← project autobiography (irreplaceable)
│   ├── patina.db                ← scrape projections (rebuildable)
│   └── runtime.db               ← child state for THIS project
│
├── persona/{persona_uid}/
│   ├── identity.age             ← keypair
│   └── beliefs.db               ← persona beliefs (cross-project)
│
└── lakes/{name}/
    ├── lake.toml
    └── lake.duckdb              ← (unchanged)
```

### How CLI tools find databases

```
1. CLI reads project/.patina/uid → gets project_uid (e.g., "2bdc808e")
2. CLI resolves: ~/.patina/mother/projects/2bdc808e/
3. CLI opens SQLite directly: events.db, patina.db, or runtime.db
4. No daemon needed. No HTTP. Just local file access.
```

Mother daemon, when running, uses the same paths. No conflict — SQLite WAL handles concurrent readers and serialized writers.

### How pipelines adapt

**scrape** reads working-copy code → writes to `mother/projects/{uid}/patina.db` and `events.db`
**oxidize** reads `mother/projects/{uid}/patina.db` → writes to `project/.patina/local/cache/embeddings/`
**scry** reads `mother/projects/{uid}/patina.db` + `project/.patina/local/cache/embeddings/`
**eval/measure** ATTACH `mother/projects/{uid}/events.db` ↔ `patina.db` (same directory, simple)
**children** (via host_support) read/write `mother/projects/{uid}/events.db` and `runtime.db`
**Mother registry** writes boundary metrics to `mother/projects/{uid}/events.db` (no more duplication)

### What Mother daemon adds (when running)

- Child orchestration — load WASM children, call handle(), manage lifecycle
- Boundary observation — wrap handle() with metrics, write to project events.db
- Cross-project queries — DuckDB ATTACH across project SQLite databases
- Session management — track active sessions in state.db
- Project registry — enumerate projects, resolve paths
- Secrets — vault, keychain, encryption

### What this enables for multiproject-belief-share (MVP 2)

Mother can enumerate projects, ATTACH their patina.db files, query beliefs by facet:

```sql
-- DuckDB: find beliefs relevant to a new Rust project
ATTACH 'projects/abc123/patina.db' AS p1 (TYPE SQLITE);
ATTACH 'projects/def456/patina.db' AS p2 (TYPE SQLITE);
SELECT 'abc123' as source, id, statement, facets FROM p1.beliefs WHERE facets LIKE '%rust%'
UNION ALL
SELECT 'def456', id, statement, facets FROM p2.beliefs WHERE facets LIKE '%rust%';
```

Persona beliefs from `persona/{uid}/beliefs.db` join the query. Adopted beliefs land as plain text in the new project's `layer/surface/epistemic/beliefs/` — git-tracked, defeatable through normal lifecycle.

### What federates across Mothers (future, via iroh)

| Federates | Mechanism | Why |
|-----------|-----------|-----|
| Beliefs | Git pull (today), iroh (future) | Knowledge declarations travel |
| Specs, sessions | Git pull | Contracts and history travel with code |
| Persona identity | Keypair exchange | Trust root for signing |
| Events | **NEVER** | Machine-local autobiography |
| Projections | **NEVER** | Rebuild locally from code |
| Child state | **NEVER** | Machine-local runtime |

## Execution Contract

1. **Scalpel over shotgun** — change only gate-targeted files. No opportunistic rewrites.
2. **Read before write/remove** — inspect current code paths and callsites before edits.
3. **One gate at a time** — do not start next gate until current gate's exit proofs pass.
4. **Cargo check between gates** — `cargo check --workspace -q` after every gate.
5. **No silent scope changes** — if a gate reveals unexpected entanglement, stop and update spec.
6. **No Co-Authored-By** in commits (per CLAUDE.md).
7. **Atomic commits** — one per logical change.

## Solution — Phase Gates

### GMDP-G1: Delete dead databases

Remove code that creates or references cortex.db, navigation.db, and project-level graph.db.

- **Entry**: Verify 0 bytes / unused via `rg` searches
- **Exit proof**: `rg "cortex\.db|navigation\.db" --type rust src/` returns zero. `cargo check --workspace -q`.

### GMDP-G2: Project path resolution module

Add `paths::mother::projects` sub-module to `src/paths.rs`. Given a `project_uid`, return `~/.patina/mother/projects/{uid}/` with `events.db`, `patina.db`, `runtime.db` paths. Create directory on first use. This is the single source of truth for where project databases live.

**Security boundary**: `project_dir(uid)` must validate that uid is exactly 8 lowercase hex characters before constructing the path. This is the one checkpoint where untrusted input (read from `.patina/uid`, which is git-tracked and could be adversarial in a cloned repo) becomes a filesystem path. Reject anything that isn't `[0-9a-f]{8}` — no path traversal possible with that character set.

- **Entry**: GMDP-G1 passes
- **Exit proof**: Unit tests verify path construction, directory creation, and uid validation (rejects `../`, empty, non-hex). `cargo check --workspace -q`.

### GMDP-G3: Project registration in Mother

Add `project_registry` table to `state.db`. Register project on first use (`patina init` or first command that resolves project_uid). Idempotent. Mother can enumerate registered projects.

- **Entry**: GMDP-G2 passes
- **Exit proof**: Registration round-trips. Duplicate registration is no-op. `cargo check --workspace -q`.

### GMDP-G4: Unified event writing path

Refactor `src/eventlog.rs` — `open_events_db()` and `open_events_db_at()` resolve to Mother-scoped path when project_uid is available. All consumers get the same connection with the same schema, PRAGMAs, and migrations. Remove the duplicated `open_events_db()` from `mother/src/registry.rs`.

- **Entry**: GMDP-G3 passes
- **Exit proof**: `rg "EVENTS_DB_REL_PATH" mother/src/registry.rs` returns zero. Event writes land in `~/.patina/mother/projects/{uid}/events.db`. `cargo test -q --lib`.

### GMDP-G5: Split runtime.db — per-project child state

Move child tables (mother_child_state, mother_child_offsets, mother_child_tasks, mother_child_checkpoints, mother_child_subscriptions, mother_child_runs) from global `runtime.db` to per-project `projects/{uid}/runtime.db`. Mother-level tables (sessions, lake cursors, belief/graph mutations) stay in `state.db` (renamed from runtime.db).

- **Entry**: GMDP-G4 passes
- **Exit proof**: Two test projects, same child name, independent state. `cargo test -q --lib`.

### GMDP-G6: Move patina.db to Mother path

Update scrape to write projections to `mother/projects/{uid}/patina.db`. Update oxidize, scry, eval, assay, measure to read from Mother-scoped path. ATTACH queries simplified — both databases now in same directory.

- **Entry**: GMDP-G5 passes
- **Exit proof**: `patina scrape` writes to Mother path. `patina scry "test"` returns results. `cargo test -q --lib`.

### GMDP-G7: Wire remaining consumers

Update all remaining direct database call sites: session events, interface checkin, broker, context, AI surface, doctor runtime, commands/events. All resolve through project_uid to Mother paths.

- **Entry**: GMDP-G6 passes
- **Exit proof**: `rg "\.patina/local/data" --type rust src/` returns zero (no more project-local database references). `cargo test -q --lib`.

### GMDP-G8: Clean project directory

Remove project-local database creation from init and eventlog. `.patina/local/` contains only `cache/embeddings/`. Remove stale references.

- **Entry**: GMDP-G7 passes
- **Exit proof**: Fresh `patina init` creates no SQLite in project directory. `cargo check --workspace -q`.

### GMDP-G9: Persona belief store structure

Create `~/.patina/mother/persona/{persona_uid}/` directory structure. Ensure beliefs.db (or beliefs directory) exists. This gate doesn't build belief propagation — it ensures the storage exists and doesn't block multiproject-belief-share.

- **Entry**: GMDP-G8 passes
- **Exit proof**: Persona directory created on Mother init. Path resolution works. `cargo check --workspace -q`.

### GMDP-G10: One-time migration

Move existing data for the current project. Explicit contract:

**Preflight checks** (fail migration if any fail):
1. Mother daemon is NOT running (`pgrep -f "patina mother"` returns nothing)
2. Acquire migration lockfile at `~/.patina/mother/projects/{uid}/.migration-lock` (fail if already held — another migration or CLI command is active). Lock is a PID file; stale locks detected by checking if PID is alive.
3. No `.db-wal` or `.db-shm` files exist alongside events.db (WAL must be checkpointed — if they exist, run `PRAGMA wal_checkpoint(TRUNCATE);` first)
4. `.patina/uid` is readable and valid (8 hex chars per INV-2)
5. Target directory `~/.patina/mother/projects/{uid}/` does not already contain events.db (idempotence: if target exists, skip that file and report)

**Steps**:
1. `cp` (not `mv`) events.db and patina.db to Mother project directory. Copy preserves the original as implicit backup.
2. Verify copied files:
   - `PRAGMA integrity_check;` — structural integrity
   - `PRAGMA quick_check;` — fast consistency check
   - `SELECT COUNT(*) FROM eventlog;` — row count matches source (for events.db)
   - Schema version matches expected (check `scrape_meta` for events.db migration version)
3. Delete originals from `.patina/local/data/` only after all verification passes.
4. Delete dead databases: cortex.db, project-level graph.db.
5. Write migration marker: `~/.patina/mother/projects/{uid}/.migrated` with timestamp and source row counts.
6. Release migration lockfile.
7. Print summary: files moved, sizes, row counts verified, files deleted.

**Idempotence**: if `.migrated` marker exists, skip already-migrated files. Report what was skipped.

**Recovery on failure**: originals are still in place (copy-then-delete, not move). User can re-run migration after fixing the issue.

- **Entry**: GMDP-G9 passes. **User approval required before execution.**
- **Exit proof**: Files at Mother path, integrity check passes, old database locations removed, `.migrated` marker exists. `patina scry "test"` returns results.

### GMDP-G11: Final verification

Full test suite. WASM integration. End-to-end smoke: scrape → oxidize → scry.

- **Entry**: GMDP-G10 passes
- **Exit proof**: `cargo check --workspace -q`. `cargo test -q --lib`. `cargo test -q --test wasm_integration folder_text_to_parquet_six_child_pipeline_composes_via_events`. `patina scrape && patina scry "how does handle work?"` returns results.

## Invariants

These must hold throughout and after the refactor. Build agents: if a gate would violate an invariant, stop and update the spec.

### INV-1: Projection scope is per-project, not per-working-copy

`patina.db` contains beliefs, patterns, commits, co-changes — all derived from `layer/` and git history, which are the same across worktrees. Therefore `patina.db` is project-scoped and lives in `mother/projects/{uid}/patina.db`.

Embeddings (`.usearch`, `.safetensors`) are derived from code file contents, which differ per branch. Therefore embeddings are working-copy-scoped and stay in `project/.patina/local/cache/embeddings/`.

**Semantic race rule**: WAL prevents corruption but not semantic drift. Two worktrees scraping the same `patina.db` can produce last-writer-wins results where projections reflect one branch's code symbols but another branch's scrape metadata. To prevent this:

- scrape writes a `head_commit` stamp into `scrape_meta` recording the git HEAD at scrape time.
- oxidize verifies that `head_commit` in patina.db matches (or is an ancestor of) the current working copy's HEAD before building embeddings. If mismatched, oxidize warns and re-scrapes.
- scry does NOT verify — it reads whatever projections exist. Stale projections degrade result quality but don't corrupt data.

This means: scrape from worktree A, then oxidize from worktree B, will trigger a re-scrape. That's correct — embeddings must match the projections they're built from, and projections should match the working copy's code.

### INV-2: project_uid identity lifecycle

- **Created**: once, during `patina init`, as 8 random lowercase hex characters. Written to `.patina/uid`.
- **Never rotates**: uid is permanent. Changing it would orphan Mother's databases for that project.
- **Git-tracked**: `.patina/uid` is committed to git. Clones and forks get the same uid. This is intentional — same project identity across machines enables federation.
- **Collision risk**: 8 hex chars = 4 billion values. Acceptable for single-user, personal-scale use. If collision matters at scale, uid generation can be changed later — but existing uids never rotate.
- **Path changes**: if a project moves on disk, the project_registry entry (uid → path) becomes stale. Mother re-registers on next use — registration is idempotent and updates the path. The databases don't move — they're keyed by uid in Mother's directory, independent of project path.
- **Forks**: a fork inherits the parent's uid via git. If both fork and upstream are registered with the same Mother, they **share databases** — events and projections co-mingle. This is a known semantic conflict.
  - **Detection**: on `patina init`, if `.patina/uid` already exists AND the git remote origin doesn't match the project_registry's recorded path's remote, warn: "This uid is already registered from a different remote. Run `patina init --new-uid` to generate a fresh identity."
  - **`patina init --new-uid`**: generates a new uid, writes to `.patina/uid`, registers as a new project. Old uid's databases are unaffected.
  - **Default behavior**: do not silently co-mingle. Warn and let the user decide.

### INV-3: state.db / runtime.db transaction boundary

After the split, `state.db` (Mother lifecycle) and `projects/{uid}/runtime.db` (child state) are separate SQLite databases. Operations that previously relied on a single transaction across session state and child state cannot do so anymore.

**Rule**: no operation may assume transactional consistency between state.db and runtime.db.

**Per-flow consistency guarantees:**

| Flow | Write order | Failure mode | Recovery |
|------|------------|--------------|----------|
| Session start | state.db (session record) → runtime.db (child state) | Session recorded, child state missing | Child re-initializes on next handle() — at-least-once |
| Child handle() | runtime.db (state/offsets) → events.db (metrics) | State updated, metric missing | Metric gap is acceptable — boundary observation is best-effort |
| Session end | runtime.db (child cleanup) → state.db (session archived) | Child state cleaned, session not archived | Session appears "active" until next cleanup pass — idempotent |
| Event drain | events.db (read) → runtime.db (ack offset) | Events read, offset not acked | Events re-delivered on next drain — at-least-once, dedup by content_hash |

**Design principle**: all cross-database flows are at-least-once safe. Duplicate events are caught by `content_hash` dedup index. Duplicate state writes are idempotent (keyvalue put is overwrite). No compensating transactions needed.

### INV-4: source_id taxonomy is the canonical vocabulary

All event producers use this taxonomy in the `source_id` field:

| Pattern | Producer | Stable? |
|---------|----------|---------|
| `core` | Compiled-in core tools | Yes |
| `child:{name}` | WASM child (replaces `plugin:{name}`) | **Changed** — align with project vocabulary |
| `child:{name}:measure:{metric}` | Child declared metrics | **Changed** |
| `mother:{child}:{metric}` | Mother boundary observation | Yes |
| `interface:{kind}:{session}` | Interface session events | Yes |
| `{tool}:{mode}` | Core measure events | Yes |

**Transition plan**:
- **Phase 1 (this spec)**: new events use `child:*`. Query code reads both `plugin:*` and `child:*` via `WHERE source_id LIKE 'plugin:%' OR source_id LIKE 'child:%'`. No rewriting of old events.
- **Phase 2 (future, not this spec)**: after one full scrape cycle on all active projects, all meaningful events have `child:*` duplicates. A cleanup pass can drop legacy `plugin:*` events or leave them as historical.
- **Cutoff**: `plugin:*` read support is removed no earlier than the next major version bump after all active projects have been re-scraped. Until then, dual-read is required.

### INV-5: path migration enforcement

After GMDP-G7, no Rust source file outside `src/paths.rs` may contain the literal string `.patina/local/data` for database access.

**Enforcement layers** (defense in depth):

1. **API enforcement**: all database opens must go through `paths::mother::projects::{events_db, patina_db, runtime_db}(uid)`. Direct `Connection::open` with a raw path string is a bug.
2. **Grep proof for G7**: `rg "\.patina/local/data" --type rust src/ mother/` returns zero hits (excluding `paths.rs`, test fixtures, and migration code).
3. **Compile-time test**: add a `#[test]` in `paths.rs` that scans source files for the raw literal and fails if found. This catches regressions in CI, not just at gate time.
4. **CI lint (future)**: promote the test to a CI step.

This prevents silent regression where a new call site bypasses Mother-resolved paths.

### INV-6: .patina/local/ complete contents

The spec claims "only embeddings" but `.patina/local/` currently contains more:

| Directory | Contents | Rebuildable? | Cleanup owner | Retention |
|-----------|----------|-------------|---------------|-----------|
| `data/embeddings/` | .usearch, .safetensors | Yes — `patina oxidize` | CLI (oxidize) | Rebuild on demand, delete freely |
| `data/forge/` | .forge-pr, .forge-issue files | Yes — `patina scrape` re-fetches from GitHub API | CLI (scrape/forge) | Rebuild on demand, delete freely |
| `data/` databases | events.db, patina.db, cortex.db, graph.db | **Moved/deleted by this spec** | Migration (G10) | N/A after migration |
| `logs/` | tmux events, MCP server logs | N/A — ephemeral | Runtime | Rotate or delete at will |
| `interface-sessions/` | claude.toml, opencode.toml | No — active interface state | Interface adapters | Survives across sessions, lost on delete |
| `backups/` | Interface backup snapshots | No — historical snapshots | CLI (ai setup) | User decides retention |
| `last-session.md` | Pointer to last session artifact | Yes — convenience pointer | Session scripts | Overwritten each session |
| `hook.log` | Hook execution log | N/A — ephemeral | Runtime | Delete at will |

**Corrected claim**: after migration, `.patina/local/` contains rebuildable cache (embeddings, forge), runtime logs, interface session state, backups, and convenience files. No SQLite databases. The spec exit criterion (gmdp8) is updated to reflect this.

**Irreplaceable items in .patina/local/**: `interface-sessions/` (active interface state) and `backups/` (historical snapshots). Everything else is ephemeral or rebuildable.

## Resolved Decisions

1. **Per-project databases in Mother (Option B).** Isolation, portability, no collisions. Each project's data can be moved atomically. Foundation for DuckDB ATTACH.

2. **CLI opens local files directly.** No daemon round-trip for core protocol operations. Mother is local — her databases are local files. Path resolution via project_uid, then direct `Connection::open()`. `core-verbs-standalone-mother-additive` preserved.

3. **Embeddings stay working-copy-local.** Branch-specific (different code = different vectors). Only derived state that's truly working-copy-dependent.

4. **Events never federate.** Machine-local autobiography. Beliefs federate (via git today, iroh future). Projections rebuild locally. This matches LiveStore, Matrix, and Git patterns.

5. **state.db splits from runtime.db.** Mother lifecycle state vs per-project child runtime state are different scopes with different access patterns. No cross-database transactions (INV-3).

6. **One-time migration, not compatibility shim.** Single user. Move files, update paths. No backwards compatibility needed.

7. **Persona belief store is structure, not system.** Create the directory and schema placeholder. multiproject-belief-share builds the propagation system on top.

8. **oxidize.yaml stays in .patina/, not layer/.** Build configuration, not knowledge. Like Cargo.toml — project config that travels with code but isn't the product.

9. **patina rebuild routes through Mother path.** Rebuild runs scrape (writes to Mother-scoped patina.db) then oxidize (rebuilds local embeddings). Same path resolution as any other command.

## Verification

```bash
patina spec check greenfield-mother-patina-data-platform --json
cargo check --workspace -q
cargo test -q --lib
```

## Build Readiness

Beliefs aligned. Architecture agreed in session 20260330-083255-177610000 through deep exploration of database ownership, multi-project scaling, federation models, and prior art (LiveStore, Matrix, iroh, DuckLake). Prior art: greenfield-mother-clean-continued (12 gates, completed), folder-text-to-parquet (9 gates, completed). This spec has 11 gates. Natural seam: G1–G5 (infrastructure: paths, registration, event unification, state split), G6–G8 (consumer wiring), G9–G11 (persona, migration, verification). Blocks multiproject-belief-share (MVP 2).
