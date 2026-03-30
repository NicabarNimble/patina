# Design: Greenfield Mother Data Platform

## Why This Design

Patina's databases grew around a monolithic CLI. The system now has Mother (daemon), children (WASM), personas (crypto identity), and needs to scale to multiple projects and eventually multi-Mother federation. The database layout must match the architecture: Mother is local infrastructure, projects are sovereign islands, coordination sits above storage.

Derived in session 20260330-083255-177610000 through: full database audit (every file, every call site, every data flow), architectural alignment with four-role belief, prior art study (LiveStore, Matrix, iroh, DuckLake, Git), and the discovery that the federated sovereign-node pattern is the universal answer.

## Architecture

### The four scopes and their data

**Mother** — per-machine daemon. Owns: `~/.patina/mother/`. Her databases, her directory. She manages project databases as local infrastructure but doesn't centralize them into a single store. She's the coordinator, not the owner of project knowledge.

**Project** — per-repo, plain text. Owns: `layer/` (the knowledge product) + `.patina/` (identity and config). Databases live in Mother's directory scoped by project_uid — but the project's knowledge is in `layer/`, git-tracked, federates via git pull. A project at rest is complete.

**Child** — WASM worker inside Mother. Owns nothing persistent. State in Mother's per-project runtime.db. Events through Mother's per-project events.db. Children don't know where databases live — they use SDK APIs (granted::state, messaging, events-stream) and Mother resolves the storage.

**Working copy** — a checked-out version of a project (main branch, worktree, jj change). Owns: rebuildable cache only. Embeddings are branch-specific — different code produces different vectors. This is the one piece of derived state that varies per working copy.

### Database layout

```
~/.patina/mother/
│
├── state.db                         ← Mother's own lifecycle
│   Tables:
│   - mother_sessions                (project_uid scoped)
│   - mother_session_participants
│   - mother_session_handoffs
│   - project_registry               (NEW: uid, path, registered_at)
│   - mother_lake_cursors
│   - graph_mutation_log
│   - belief_mutation_log
│   - belief_verifications
│   - belief_evidence
│   - belief_relationships
│
├── graph.db                         ← cross-project belief graph (unchanged)
│
├── projects/
│   └── {project_uid}/
│       │
│       ├── events.db                ← project autobiography (IRREPLACEABLE)
│       │   PRAGMAs: WAL, synchronous=FULL, busy_timeout=5000
│       │   Migrations: v1→v2→v3
│       │   This machine's experience with this project.
│       │   Does NOT federate to other Mothers.
│       │
│       ├── patina.db                ← scrape projections (REBUILDABLE)
│       │   Materialized from code + layer/ + git.
│       │   Tables: eventlog copy, code_fts, beliefs, belief_fts,
│       │   patterns, pattern_fts, commits, commits_fts,
│       │   co_changes, moments, scrape_meta
│       │
│       └── runtime.db               ← child state for THIS project
│           Tables: mother_child_state, mother_child_offsets,
│           mother_child_tasks, mother_child_checkpoints,
│           mother_child_subscriptions, mother_child_runs
│           No project_uid column — the database IS the scope.
│
├── persona/
│   └── {persona_uid}/
│       ├── identity.age             ← keypair (signs beliefs, UCAN tokens)
│       └── beliefs.db               ← persona beliefs (cross-project)
│
└── lakes/{name}/
    ├── lake.toml
    └── lake.duckdb                  ← DuckLake storage (unchanged)


project/
├── .patina/
│   ├── config.toml                  ← project identity + settings
│   ├── uid                          ← stable 8-hex project identity
│   ├── oxidize.yaml                 ← embedding recipe
│   └── local/                       ← gitignored
│       └── cache/
│           └── embeddings/          ← .usearch + .safetensors
├── layer/                           ← THE PRODUCT (git-tracked)
└── (code)
```

### How CLI tools access data

Mother is local. Her databases are local files. Core CLI tools open them directly.

```
1. CLI reads project/.patina/uid → "2bdc808e"
2. CLI resolves: ~/.patina/mother/projects/2bdc808e/
3. CLI opens: events.db, patina.db, or runtime.db (direct Connection::open)
4. No daemon needed for core protocol operations.
```

Mother daemon, when running, uses the same paths. SQLite WAL handles concurrent access.

### Data flows

**scrape → oxidize → scry**

```
scrape (reads working copy code + layer/ + git)
  ├── writes projections → mother/projects/{uid}/patina.db
  └── writes events → mother/projects/{uid}/events.db

oxidize (reads projections, writes embeddings)
  ├── reads corpus → mother/projects/{uid}/patina.db
  └── writes embeddings → project/.patina/local/cache/embeddings/
                          (working-copy-local, branch-specific)

scry (reads projections + embeddings)
  ├── reads projections → mother/projects/{uid}/patina.db
  ├── reads embeddings → project/.patina/local/cache/embeddings/
  └── writes query events → mother/projects/{uid}/events.db
```

**child execution**

```
Mother calls child.handle(action, payload)
  │
  ├── Mother boundary observation (observe_handle)
  │   └── writes metrics → mother/projects/{uid}/events.db
  │
  ├── Child reads events (patina:events-stream subscribe)
  │   └── host_support reads → mother/projects/{uid}/events.db
  │
  ├── Child writes events (wasi:messaging/producer)
  │   └── host_support writes → mother/projects/{uid}/events.db
  │
  ├── Child reads/writes state (wasi:keyvalue)
  │   └── host_support → mother/projects/{uid}/runtime.db
  │
  └── Child emits metrics (patina:measure)
      └── host_support writes → mother/projects/{uid}/events.db
```

**cross-database queries (eval, measure)**

Both databases in the same directory — ATTACH is simple:

```sql
-- eval opens events.db, ATTACHes patina.db
-- Both at ~/.patina/mother/projects/{uid}/
ATTACH DATABASE 'patina.db' AS patina;  -- relative, same dir
SELECT ... FROM eventlog e JOIN patina.commits c ON ...;
```

### Event source_id taxonomy

All events flow through the same path. source_id identifies the producer.

**New events** use `child:*` vocabulary (aligned with project doctrine):

| Pattern | Producer |
|---------|----------|
| `core` | Compiled-in core tools (scrape, measure) |
| `child:{name}` | WASM child |
| `child:{name}:measure:{metric}` | Child declared metrics |
| `mother:{child}:{metric}` | Mother boundary observation |
| `interface:{kind}:{session}` | Interface session events |
| `{tool}:{mode}` | Core measure events (eval:feedback, scrape:code) |

**Legacy compatibility**: existing events use `plugin:*`. These are not rewritten. Query code must handle both: `WHERE source_id LIKE 'plugin:%' OR source_id LIKE 'child:%'`.

### The layered model

```
Layer 1: Plain text (git-tracked, THE PRODUCT, federates via git)
  project/layer/ — beliefs, specs, sessions, core values

Layer 2: SQLite (Mother-local, per-project, sovereign)
  mother/projects/{uid}/ — events, projections, child state
  CLI opens directly. Daemon adds coordination.
  Storage unit for DuckDB (Layer 4).
  Never leaves this machine.

Layer 3: Working-copy cache (project-local, rebuildable)
  project/.patina/local/cache/ — embeddings
  Branch-specific. Rebuild via oxidize.

Layer 4: DuckDB (Mother-level, future cross-project coordination)
  ATTACHes Layer 2 SQLite databases.
  Cross-project scry, belief discovery, federation queries.

Layer 5: iroh (future, p2p transport between Mothers)
  Syncs Layer 1 artifacts (beliefs signed by persona).
  Never syncs Layer 2 databases.
```

### What Mother daemon adds (when running)

| Capability | Requires daemon? |
|-----------|-----------------|
| scrape, oxidize, scry, assay | No — CLI opens local files |
| Child orchestration (handle, tick, drain) | Yes |
| Boundary observation (ftp4 metrics) | Yes |
| Cross-project DuckDB queries | Yes |
| Session management | Yes |
| Secrets management | Yes |
| Project registry enumeration | No — state.db is a local file |

### What enables multiproject-belief-share (MVP 2)

1. **Project registry** — Mother enumerates projects from state.db
2. **Per-project patina.db** — DuckDB ATTACHes to query beliefs across projects
3. **Persona belief store** — persona-level beliefs queryable alongside project beliefs
4. **Ref repo databases** — already at `~/.patina/cache/repos/*/` with indexed beliefs

Mother discovers beliefs relevant to a new project via DuckDB ATTACH across project + persona + ref repo databases, matched by facets and semantic similarity. Adopted beliefs land as plain text in the project's `layer/surface/epistemic/beliefs/`.

### What federates across Mothers (future, via iroh)

| Data | Federates? | Mechanism |
|------|-----------|-----------|
| Beliefs | Yes | Git pull (today), iroh-blobs (future), signed by persona keypair |
| Specs, sessions | Yes | Git pull |
| Persona identity | Yes | Keypair exchange |
| Events | Never | Machine-local autobiography |
| Projections (patina.db) | Never | Rebuild locally from code |
| Child state (runtime.db) | Never | Machine-local runtime |

Events are like LiveStore's event log — they're the source of truth for this machine's experience. Beliefs are the distilled knowledge that travels — like Matrix room events that federate between servers.

## Key Design Decisions

1. **Per-project databases (Option B), not one big database.** Isolation, portability, corruption containment. DuckDB ATTACHes across them for cross-project queries.

2. **CLI opens local files, daemon adds coordination.** core-verbs-standalone-mother-additive. No HTTP for scrape/scry/assay. Mother is local infrastructure, not a remote service.

3. **Embeddings stay working-copy-local.** Branch-specific. Only derived state that varies per worktree.

4. **Events never federate.** Beliefs federate (git, iroh). Projections rebuild. Matches every mature federated system studied.

5. **state.db splits from runtime.db.** Mother lifecycle vs project child state are different scopes.

6. **Persona store is structure, not system.** Directory + schema placeholder. multiproject-belief-share builds propagation on top.

7. **patina rebuild routes through Mother path.** Regenerates patina.db and embeddings. Mother owns the projection path.

## Open Questions (resolved)

1. **How does patina rebuild work?** → Rebuild runs scrape (writes to Mother-scoped patina.db) then oxidize (rebuilds local embeddings). Mother path resolution, same as any other command.

2. **Should oxidize.yaml move to layer/?** → No. Build config, not knowledge. Stays in `.patina/`.

3. **How do ATTACH queries work?** → Both databases in same Mother directory. ATTACH by relative path or absolute path resolved from project_uid. CLI opens the primary, ATTACHes the other. No service layer needed — just path resolution.
