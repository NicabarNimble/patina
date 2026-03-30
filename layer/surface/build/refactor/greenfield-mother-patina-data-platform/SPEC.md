---
type: refactor
id: greenfield-mother-patina-data-platform
status: draft
created: 2026-03-30
sessions:
  origin: 20260330-083255-177610000
related:
- mother/src/registry.rs
- mother/src/state.rs
- src/eventlog.rs
- src/child/internal/host_support.rs
- src/measure.rs
- src/commands/scrape/
- src/commands/oxidize/
- src/commands/scry/
- layer/surface/build/refactor/greenfield-mother-clean-continued/
- layer/surface/build/feat/folder-text-to-parquet/
beliefs:
- '[[standards-are-storage-coordination-sits-above]]'
- '[[observation-at-the-boundary]]'
- '[[children-have-agency-toys-are-capabilities]]'
- '[[four-roles-no-overlap]]'
exit_criteria:
- id: mdc1-dead-databases-removed
  text: cortex.db, navigation.db, and graph.db (project-level) deleted from codebase. Zero code references.
  checked: false
- id: mdc2-events-in-mother
  text: events.db lives at ~/.patina/mother/projects/{project_uid}/events.db. All event writers (eventlog module, host_support, measure, registry) write to the Mother-scoped path. Project .patina/local/data/events.db no longer created.
  checked: false
- id: mdc3-runtime-project-scoped
  text: runtime.db child state tables (mother_child_state, mother_child_offsets, mother_child_tasks) are scoped by project_uid. Two projects using the same child name do not collide.
  checked: false
- id: mdc4-patina-db-in-mother
  text: Scrape projections (patina.db) live at ~/.patina/mother/projects/{project_uid}/patina.db. Scrape reads code from the working copy, writes projections to Mother-scoped path.
  checked: false
- id: mdc5-working-copy-cache-only
  text: 'Project .patina/local/ contains only rebuildable working-copy-specific cache: embeddings/ (.usearch, .safetensors). No SQLite databases in project directory.'
  checked: false
- id: mdc6-registry-uses-canonical-path
  text: mother/src/registry.rs open_events_db() duplication eliminated. Registry writes boundary metrics through the same event-writing path as all other Mother components.
  checked: false
- id: mdc7-project-registers-with-mother
  text: patina init (or first command in a patina project) registers project_uid with Mother. Mother creates projects/{uid}/ directory structure. Project works offline with degraded capabilities if Mother is unavailable.
  checked: false
- id: mdc8-cli-routes-through-mother
  text: CLI commands that write events (scrape, measure, session events) route through Mother when available. Direct-write fallback for offline operation writes to project-local cache that Mother absorbs on next connection.
  checked: false
- id: mdc9-existing-data-migrated
  text: One-time migration moves existing .patina/local/data/events.db and patina.db to Mother's project directory. User informed of what moved and what was deleted.
  checked: false
- id: mdc10-workspace-compiles-tests-pass
  text: cargo check --workspace -q passes. cargo test -q --lib passes. WASM integration tests pass.
  checked: false
---
# refactor: Mother owns databases, projects stay plain text

> Consolidate database ownership in Mother. Projects keep only git-tracked `layer/` and minimal `.patina/` config. Mother holds per-project events, child runtime state, and scrape projections scoped by `project_uid`. Working-copy-specific caches (embeddings, FTS5 indexes) remain project-local as rebuildable derived state. SQLite is the portable storage unit per project; DuckDB is the future cross-project federation layer.

## Problem

Database ownership is scattered and contradictory:

1. **events.db lives in the project** at `.patina/local/data/events.db` — but it's irreplaceable runtime data pretending to be "derived local state" in a gitignored directory that `patina rebuild` claims to recreate.

2. **Mother's registry duplicates eventlog infrastructure.** `mother/src/registry.rs` (commit `2e4257ed`, ftp4) copies the events.db path constant and full schema DDL because the `mother` crate can't depend on `patina-ai`. This creates schema drift risk (missing `busy_timeout`, no migration path, no `content_hash` dedup index).

3. **Child state has no project dimension.** `runtime.db` tables (`mother_child_state`, `mother_child_offsets`) are keyed by `(plugin_name, key)` with no `project_uid`. If Mother manages children for two projects, state collides.

4. **Dead databases accumulate.** `cortex.db` (0 bytes), `navigation.db` (0 bytes), `graph.db` (project-level, 0 bytes) were never implemented or are superseded. They create confusion about what's active.

5. **The scrape/oxidize/scry pipeline writes to project-local databases** that are working-copy-specific (FTS5 indexes reflect the current branch's code) but don't distinguish between working-copy-specific state (embeddings) and project-level state (events).

This refactor was surfaced during the folder-text-to-parquet build (session 20260330-083255-177610000) when the build agent had to duplicate `open_events_db()` in Mother's registry because the dependency graph prevents Mother from accessing the canonical eventlog module.

## Core Value Anchors

- `patina-identity`: "The binary is the pipeline. The layer is the product." — `layer/` is git-tracked knowledge; databases are infrastructure Mother owns. Projects declare, Mother manages.
- `unix-philosophy`: Each component has one job. Mother manages state. Projects manage knowledge. Children do work. Databases don't cross these boundaries.
- `dependable-rust`: One path to open events.db, one schema, one migration system. No duplication.
- `spec-driven-design`: This spec authorizes the database move. Nothing outside this scope.
- `standards-are-storage-coordination-sits-above`: SQLite per-project is the portable storage unit. DuckDB (future) federates across them. Neither replaces the other.

## Goal

After this refactor:
- Mother owns all per-project databases at `~/.patina/mother/projects/{project_uid}/`
- Projects are plain text: `layer/` (git-tracked) + `.patina/config.toml` + `.patina/uid` + rebuildable cache
- One code path for event writing — no duplication between Mother and patina-ai
- Child state is project-scoped — no collisions when Mother serves multiple projects
- Dead databases deleted

## Non-Goals

- Building the DuckDB cross-project federation layer (that's the `standards-are-storage-coordination-sits-above` belief applied later, not this spec)
- Multi-Mother federation or persona-scoped databases (future spec)
- Changing the scrape/oxidize/scry pipeline logic (only where its output lives)
- Redesigning the event schema or eventlog format
- Building elaborate data protection or rollback systems (single user, we accept migration risk with awareness)
- Changing the child runtime model or WIT interfaces
- Extracting `patina-core` as a shared crate (dependency question resolved: Mother owns database code, CLI routes through Mother)

## What You Will Lose During Migration

**User decision required before mdc9 executes:**

- **events.db move**: Your 11MB `events.db` moves from `.patina/local/data/` to `~/.patina/mother/projects/{uid}/`. The file is the same — just relocated. If the move fails or is interrupted, the original is still in place. No data loss if migration completes.

- **patina.db move**: Your 170MB `patina.db` (scrape projections) moves to Mother. This is fully rebuildable via `patina scrape && patina oxidize` if anything goes wrong. Worst case: you re-scrape.

- **Dead databases deleted**: `cortex.db` (0 bytes), `navigation.db` (0 bytes), project-level `graph.db` (0 bytes). Nothing is lost — these are empty files.

- **Mother-level graph.db** (569KB at `~/.patina/mother/graph.db`): Kept as-is. Not touched by this spec.

- **Lake databases** (`~/.patina/lakes/*/lake.duckdb`): Kept as-is. Not touched by this spec.

- **Offline degradation**: After this refactor, commands that write events require Mother to be running (or use a local fallback that Mother absorbs later). Currently they write directly. This is a capability trade — you gain clean ownership, you lose always-works-without-Mother for write paths.

## Current State

### Database files on disk

```
.patina/local/data/
├── events.db        (11MB, irreplaceable, WAL+FULL)
├── patina.db        (170MB, rebuildable from scrape)
├── cortex.db        (0 bytes, dead)
├── navigation.db    (does not exist in code, dead reference)
├── graph.db         (0 bytes, dead)
├── embeddings/      (working-copy-specific, rebuildable)
└── forge/           (forge-specific cache)

~/.patina/mother/
├── runtime.db       (3.2MB, child state + tasks + sessions)
└── graph.db         (569KB, cross-project belief graph)

~/.patina/lakes/*/lake.duckdb  (DuckLake storage, per-lake)
```

### Code paths that open events.db

1. `src/eventlog.rs` — canonical: `open_events_db()`, `open_events_db_at()`, `ensure_events_db()`. Used by 40+ call sites across patina-ai.
2. `src/child/internal/host_support.rs` — WASM child metrics/facts. Calls `crate::eventlog::open_events_db()`.
3. `mother/src/registry.rs` — Mother boundary metrics (ftp4). **Duplicated** `open_events_db()` with inline schema DDL.

### Code paths that open runtime.db

1. `mother/src/state.rs` — `KnowledgeRuntimeStore::open()`. Absolute path via `patina_home().join("mother/runtime.db")`.
2. Used by: daemon bootstrap, child state (keyvalue), event offsets, task queue, session records, lake cursors.
3. **No project_uid scoping** on child tables — `mother_child_state` keyed by `(plugin_name, key)`.

### Code paths that open patina.db

1. `src/eventlog.rs` — `initialize()` creates schema. Used by scrape pipeline.
2. `src/commands/scrape/` — writes code_fts, beliefs, patterns, commits, co_changes.
3. `src/commands/oxidize/` — reads patina.db to generate training data and build indexes.
4. `src/commands/scry/` — reads patina.db for result enrichment (beliefs, patterns, commits).
5. `src/commands/eval/` — ATTACHes patina.db to events.db for cross-database queries.
6. `src/commands/measure/` — ATTACHes events.db to patina.db.

## Target State

```
project/
├── .patina/
│   ├── config.toml              ← project identity + settings
│   ├── uid                      ← stable 8-hex project identity
│   ├── oxidize.yaml             ← embedding recipe
│   └── local/                   ← gitignored, working-copy-specific
│       └── cache/
│           └── embeddings/      ← .usearch + .safetensors (rebuildable)
├── layer/                       ← THE PRODUCT (git-tracked)
└── (code)

~/.patina/mother/
├── projects/{project_uid}/
│   ├── events.db                ← project autobiography (irreplaceable)
│   ├── patina.db                ← scrape projections (rebuildable)
│   └── runtime.db               ← child state, offsets, tasks for THIS project
├── state.db                     ← Mother's own lifecycle (sessions, registry)
├── graph.db                     ← cross-project belief graph (unchanged)
└── (persona/, lakes/ — untouched by this spec)
```

### Key design decisions

1. **Per-project runtime.db** instead of adding `project_uid` column to the shared one. Isolation: one project's corruption doesn't touch others. Portability: you can move a project's database directory atomically.

2. **Scrape projections in Mother** because the principle is "Mother owns databases, projects are plain text." Working-copy-specific *embeddings* stay in the project because they reflect the current code, not project identity. But patina.db (which scrape writes and scry reads) is project-scoped, not working-copy-scoped — beliefs, patterns, and commits don't change per worktree; they come from `layer/` and git which are the same across worktrees.

3. **CLI routes through Mother for writes.** When Mother is running, event writes go through Mother (single code path, no duplication). When Mother is offline, a local fallback writes to a temporary journal that Mother absorbs on next startup. This is not an elaborate escape hatch — it's a simple append-only file that gets replayed.

4. **One-time migration**, not a compatibility shim. Move the files, update the paths, delete the dead databases. Single user, no backwards compatibility needed.

## Execution Contract

1. **Scalpel over shotgun** — change only gate-targeted files. No opportunistic rewrites.
2. **Read before write/remove** — inspect current code paths and callsites before edits.
3. **One gate at a time** — do not start the next gate until current gate's exit proofs pass.
4. **Cargo check between gates** — `cargo check --workspace -q` must succeed after every gate.
5. **No silent scope changes** — if a gate reveals unexpected entanglement, stop and update the spec.
6. **No Co-Authored-By** in commits (per CLAUDE.md).
7. **Atomic commits** — one commit per logical change. `feat:` for new capability, `refactor:` for moves, `fix:` for corrections, `spec:` for spec updates.

## Solution — Phase Gates

### MDC-G1: Delete dead databases

Remove code that creates or references `cortex.db`, `navigation.db`, and project-level `graph.db`.

- **Entry**: Verify these files are 0 bytes / unused with `rg` searches
- **Exit proof**: `rg "cortex\.db|navigation\.db" --type rust src/` returns zero. `cargo check --workspace -q` passes.

### MDC-G2: Add project_uid scoping to Mother paths

Create `mother/src/project_paths.rs` — given a `project_uid`, returns paths for `events.db`, `patina.db`, `runtime.db` under `~/.patina/mother/projects/{uid}/`. Ensures directory creation.

- **Entry**: MDC-G1 exit proofs pass
- **Exit proof**: Unit tests verify path construction and directory creation. `cargo check --workspace -q` passes.

### MDC-G3: Move events.db to Mother

Update `src/eventlog.rs` to accept a project-root-resolved or Mother-resolved path. All callers that currently use the relative `.patina/local/data/events.db` switch to the Mother-scoped path when a `project_uid` is available. Remove the duplicated `open_events_db()` from `mother/src/registry.rs` — registry uses the same path module.

- **Entry**: MDC-G2 exit proofs pass
- **Exit proof**: `rg "EVENTS_DB_REL_PATH" mother/src/registry.rs` returns zero. Event writes land in `~/.patina/mother/projects/{uid}/events.db`. `cargo test -q --lib` passes.

### MDC-G4: Move patina.db to Mother

Update scrape, oxidize, scry, eval, measure, and assay to read/write `patina.db` from Mother-scoped path. Oxidize and scry continue to use working-copy-local embeddings for vector search.

- **Entry**: MDC-G3 exit proofs pass
- **Exit proof**: Scrape writes to Mother path. Scry reads from Mother path. `cargo test -q --lib` passes.

### MDC-G5: Split runtime.db per project

Create per-project `runtime.db` at `~/.patina/mother/projects/{uid}/runtime.db`. Move child state tables (mother_child_state, mother_child_offsets, mother_child_tasks, mother_child_checkpoints, mother_child_subscriptions) to per-project databases. Mother-level state (mother_sessions, mother_session_participants, graph/belief mutation logs, lake cursors, belief tables) stays in Mother's own `state.db`.

- **Entry**: MDC-G4 exit proofs pass
- **Exit proof**: Two test projects with same child name have independent state. `cargo test -q --lib` passes.

### MDC-G6: Clean up project .patina/local/

Remove project-local database creation from init and eventlog. Project `.patina/local/` only contains `cache/embeddings/`. Remove stale references to `.patina/local/data/` for databases.

- **Entry**: MDC-G5 exit proofs pass
- **Exit proof**: Fresh `patina init` creates no SQLite databases in project directory. `cargo check --workspace -q` passes.

### MDC-G7: Project registration with Mother

First CLI command in a patina project (or explicit `patina init`) registers `project_uid` with Mother. Mother creates `projects/{uid}/` directory structure. Graceful offline fallback: write to project-local temporary journal, Mother absorbs on next connection.

- **Entry**: MDC-G6 exit proofs pass
- **Exit proof**: `patina init` on a fresh project creates Mother directory. Commands write events to Mother path. Offline fallback writes to local journal.

### MDC-G8: One-time migration

Move existing data files for the current project:
- `.patina/local/data/events.db` → `~/.patina/mother/projects/{uid}/events.db`
- `.patina/local/data/patina.db` → `~/.patina/mother/projects/{uid}/patina.db`
- Delete `.patina/local/data/cortex.db`, `graph.db` (if not already gone from G1)
- Print summary of what moved and what was deleted

- **Entry**: MDC-G7 exit proofs pass. **User approval required** before execution.
- **Exit proof**: Files exist at Mother path. Old locations empty or removed. `patina scry "test query"` returns results from Mother-scoped database.

### MDC-G9: Final verification

Full test suite. Workspace compile. WASM integration tests. Manual smoke test of scrape → oxidize → scry pipeline.

- **Entry**: MDC-G8 exit proofs pass
- **Exit proof**: `cargo check --workspace -q`. `cargo test -q --lib`. `cargo test -q --test wasm_integration folder_text_to_parquet_six_child_pipeline_composes_via_events`. `patina scrape && patina scry "how does handle work?"` returns results.

## Verification

```bash
patina spec check mother-database-consolidation --json
cargo check --workspace -q
cargo test -q --lib
```

## Build Readiness

Beliefs aligned. Architecture agreed in session 20260330-083255-177610000. Prior art: greenfield-mother-clean-continued (12 gates, completed), folder-text-to-parquet (9 gates, completed). This spec has 9 gates. The first two (dead database removal, path module) are low-risk warmup. The core work is G3–G5 (move events, move patina.db, split runtime). G7–G8 (registration, migration) are the integration layer. Executable across multiple sessions.
