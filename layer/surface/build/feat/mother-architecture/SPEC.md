---
type: feat
id: mother-architecture
status: design
created: 2026-02-10
sessions:
  origin: 20260210-061323
related:
  - layer/surface/build/feat/mother/SPEC.md
  - layer/surface/build/feat/mother-environment/SPEC.md
  - layer/surface/build/feat/mother-repos/SPEC.md
  - layer/surface/build/feat/mother-beliefs/SPEC.md
  - layer/surface/build/feat/mother-dashboard/SPEC.md
  - layer/surface/build/feat/patina-platform/SPEC.md
beliefs:
  - mother-is-the-daemon
  - four-layer-architecture
  - unix-philosophy
  - simplicity-is-architecture
---

# feat: Mother Architecture — Self, Children, and Toys

> Mother is a real mother. She has self (state, heartbeat, daemon) and children
> (trait-based modules that own parts of `~/.patina/`). Children are independent
> but report to Mother. Some children can launch toys (processes that do work).
> All children start as native Rust behind traits. They can evolve to WASM
> plugins behind WIT interfaces. Same shape at both stages.

## Problem

Mother keeps growing in scope. Every time we add a responsibility (models, repos,
beliefs, secrets, graph, dashboard, agents), the monolithic daemon concept gets
harder to reason about. We can't hold it all in our heads at once. Previous
attempts (Mother v2) built inert scaffolding because the spec was too big to
validate against reality.

The 4 decomposed specs (environment, repos, beliefs, dashboard) are better but
still lack an architectural frame. They don't describe how these pieces relate
to each other or to Mother herself.

## The Mental Model

Flat, same shape, simple relationships. Don't build deep hierarchies of
ownership. Mother is the array. Children are the things.

Like a real mother:
- She has **self** — her own state, her heartbeat, her presence
- She has **children** — independent beings she knows, checks on, coordinates
- Children can have **toys** — things they launch to do work
- Children are separate from Mother but part of the same family

## Architecture

```
Mother (self)
│
│  State:     daemon process, PID, socket, heartbeat
│  Job:       iterate children, route requests, detect problems
│  Interface: UDS socket, MCP tools (scry, context, assay)
│
├── child: models
│   Owns: ~/.patina/cache/models/, registry, EmbeddingSpec
│   Serves: resolve model path, embedding identity
│   Toys: none (passive — serves on request)
│
├── child: repos
│   Owns: ~/.patina/cache/repos/, registry.yaml
│   Serves: repo lifecycle (clone, pull, scrape, index)
│   Toys: can launch agents to re-index stale repos
│
├── child: beliefs
│   Owns: ~/.patina/layer/surface/beliefs/, beliefs.db
│   Serves: cross-project belief federation, user-level beliefs
│   Toys: can launch agents to investigate belief drift
│
├── child: secrets
│   Owns: decrypted secret cache (RAM only)
│   Serves: Touch ID avoidance, secret resolution
│   Toys: none (passive — serves on request)
│
├── child: graph
│   Owns: ~/.patina/cache/graph.db
│   Serves: cross-project relationships, edge weights
│   Toys: none (derived from what other children know)
│
└── child: dashboard
    Owns: nothing (observes siblings)
    Serves: CLI status, web dashboard, health summary
    Toys: none (reads, doesn't write)
```

## Current State

Today `src/mother/` has three files:
- `mod.rs` — public interface, daemon start
- `internal.rs` — UDS client for daemon communication
- `graph.rs` — cross-project graph (31 nodes, 3 edges)

The daemon runs as a background process (`patina mother start`), listens on a
UDS socket, and today does two things: proxies scry requests (dead path — MCP
bypasses it) and caches decrypted secrets (sole real consumer). The graph
exists but is manually maintained.

This architecture refactors the daemon from a monolithic handler into a
child-iterating loop. Existing functionality (secrets cache, graph) becomes
children behind the trait. New functionality (models, repos, beliefs) plugs
in as additional children.

## The Child Trait

All children implement the same trait. [[dependable-rust]] pattern: small stable
interface, private internals.

```rust
/// Every child implements this. Mother doesn't know their internals.
pub trait MotherChild: Send + Sync {
    /// Identity
    fn name(&self) -> &str;

    /// Health check — Mother calls this on heartbeat
    fn health(&self) -> ChildHealth;

    /// Handle a request routed by Mother
    /// Request/Response shapes are child-specific — defined by each child's
    /// spec. Mother routes by child name, doesn't inspect the payload.
    fn handle(&self, request: &ChildRequest) -> Result<ChildResponse>;

    /// Heartbeat tick — child checks its own state, may launch toys.
    /// Default: return empty (passive children don't need to override).
    fn tick(&mut self) -> Vec<Toy> { vec![] }
}

pub enum ChildHealth {
    Healthy,
    Degraded(String),  // working but something's off
    Unavailable(String), // can't serve requests
}
```

**Request/Response:** Each child defines what requests it accepts. Mother routes
by child name without inspecting payloads. The exact serialization (enum
dispatch, serde, etc.) is an implementation decision for when we build the
first child.

## Toys

A toy is a process a child wants Mother to launch. The child decides *what*
needs doing. Mother handles *how* to launch it.

```rust
/// A toy is work a child wants Mother to run
pub struct Toy {
    pub name: String,
    pub command: ToyCommand,
}

pub enum ToyCommand {
    /// Shell command (e.g., patina oxidize --repo gastown)
    Shell(Vec<String>),
    /// Agent in a yolo container
    Agent { adapter: String, context: String, task: String },
}
```

How Mother returns results to the child (callback, polling, channel) is an
implementation detail to resolve when a child first needs toys. The concept:
child requests, Mother runs, child gets result.

## Mother's Self

Mother's own responsibilities (not delegated to children):

1. **Daemon lifecycle** — start, stop, PID file, socket
2. **Child registry** — know all children, iterate them
3. **Heartbeat loop** — periodically tick each child, launch any toys
4. **Request routing** — receive requests via UDS/MCP, route to the right child
5. **Toy management** — spawn processes children request, monitor, collect results

Mother does NOT know about models, repos, beliefs, or secrets directly.
She just knows she has children and she iterates them.

Examples:
- Repos child detects gastown is 30 days stale → requests a toy:
  `Shell(["patina", "oxidize", "--repo", "gastown"])`
- Beliefs child detects drift between projects → requests a toy:
  `Agent { adapter: "codex", context: "layer/sessions/...", task: "investigate belief X" }`

Children never spawn processes directly. Mother manages all toys.

## Evolution Path

```
Phase 1 (now):  Children are native Rust modules behind MotherChild trait
Phase 2 (later): Children can be WASM plugins behind WIT interfaces
                  Same trait shape. MotherChild maps to WIT world.
                  patina-platform spec defines the plugin infrastructure.
```

The trait doesn't change. A native child and a WASM child look the same to
Mother. The WIT capability grant system (from patina-platform) controls which
children can launch toys.

## What This Spec Does NOT Cover

Each child's internals are specified separately:

- **models child** → see [[mother-environment]] (registry, EmbeddingSpec, cache)
- **repos child** → see [[mother-repos]] (lifecycle, staleness, indexing)
- **beliefs child** → see [[mother-beliefs]] (federation, user beliefs, drift)
- **secrets child** → already works, no spec needed
- **graph child** → see existing graph implementation
- **dashboard child** → see [[mother-dashboard]] (CLI, web, observability)

This spec defines the **frame** — how Mother and children relate. The child
specs define the **content** — what each child actually does.

## Implementation Order

The **models child** is the most concrete and should be built first. It has:
- Real code to refactor (`resolve_model_path()`, `create_embedder()`)
- A clear EmbeddingSpec/EmbeddingBackend abstraction already designed
- Zero dependencies on other children
- An existing consumer (every `patina scry` and `patina oxidize` call)

Building models first proves the `MotherChild` trait works before adding
children with more complex needs (repos with toys, beliefs with federation).

## Acceptance Criteria

1. [ ] `MotherChild` trait defined in `src/mother/mod.rs`
2. [ ] Mother daemon iterates registered children on heartbeat
3. [ ] At least one child (models) implemented behind the trait
4. [ ] Mother routes requests to children by name
5. [ ] `Toy` struct defined — children can request process launches
6. [ ] Mother can spawn and monitor a toy (shell command)
7. [ ] `patina mother status` shows all children and their health

## Open Design Questions (Session 20260210-061323)

These emerged during brainstorming and need resolution before implementation.

### 1. Three Levels: Project, User, Machine

The entry point into patina is always from within a project. Projects are
sovereign — they own their `layer/`, their adapter choice, their sessions,
their beliefs. Mother doesn't touch project state.

Mother lives at the user + machine level. But user and machine are different:

```
Project   = this codebase, right here, right now
            layer/, adapter choice, sessions, project beliefs
            Sovereign. Mother doesn't touch this.

User      = me, across all machines (user = persona, interchangeable)
            My beliefs, my persona knowledge, my preferences, my repo list
            Portable. Should survive a machine wipe.

Machine   = this Mac, right here
            Downloaded model files, cloned repos, daemon PID, secrets cache
            Rebuilt from user data on a new machine. Not portable.
```

What lives where today at `~/.patina/`:

```
User-portable (small, sync-able):
  layer/surface/beliefs/    ← user/persona beliefs
  personas/default/events/  ← persona event log (source of truth)
  personas/default/persona.db ← persona SQLite
  registry.yaml             ← which repos I track (names + URLs)
  models.lock               ← which models I've chosen (provenance)
  mother/graph.db           ← how my projects relate
  adapters/                 ← my adapter preferences/templates

Machine-local (large, rebuildable from user data):
  cache/models/             ← ONNX files (re-download from models.lock)
  cache/repos/              ← 20 repo clones, 3.9GB (re-clone from registry.yaml)
  cache/personas/           ← materialized index (re-materialize from events)
  run/                      ← daemon PID, socket (ephemeral)
```

**Open question:** Should the spec formally separate user-portable from
machine-local? This would enable carrying user data to a new machine and
rebuilding. All machine-local state is derived from user-portable state.

### 2. Who Are the Children?

The 4 existing child specs (environment, repos, beliefs, dashboard) were
written before this architecture. They may not map cleanly to children.

What actually exists as user-level state today (each could be a child):

| CLI command | State it owns | Notes |
|-------------|--------------|-------|
| `patina model *` | `cache/models/`, `models.lock` | Model files + provenance |
| `patina repo *` | `cache/repos/`, `registry.yaml` | Repo clones + registry |
| `patina persona *` | `personas/`, `cache/personas/` | Events + materialized index |
| `patina belief audit` | `layer/surface/beliefs/` | User-level beliefs |
| `patina mother graph *` | `mother/graph.db` | Cross-project relationships |
| `patina secrets *` | RAM (daemon) | Decrypted secret cache |
| `patina adapter *` | `adapters/` | Templates per adapter |

**Open question:** Do we need 7 children, or do some of these naturally merge?
User/persona beliefs and persona knowledge seem related. Graph might be derived
from what other children know rather than its own child. Adapters might be
project-level, not Mother-level. Don't split or merge prematurely — let
implementation reveal the natural boundaries.

### 3. Belief Ownership and Access

Beliefs are the shareable layer of knowledge. Clear ownership:
- **Mother holds user/persona beliefs** — things the user believes across all projects
- **Projects hold project beliefs** — sovereign, supersede Mother's when they conflict
- **Projects can ask Mother** about her beliefs and choose to adopt them

**Open question:** How does Mother relate to project beliefs?

**Option A: Mother indexes all beliefs.** She has a `beliefs.db` with copies
from all projects. Pro: fast cross-project search, detect patterns. Con: sync
problem — when does she re-scrape? How does she know a project belief changed?

**Option B: Mother only holds hers, queries projects on demand.** Someone asks
"what do projects think about X?" — Mother reads each project's beliefs live.
Pro: no sync, projects stay sovereign, single source of truth. Con: slow,
projects must be accessible on disk.

**Option C: Mother holds a catalog, not copies.** She knows WHERE beliefs are
(which project, which file) but doesn't copy content. Like a librarian — knows
what exists, points you to it. Pro: lightweight, no content sync. Con: still
needs to detect when beliefs are added/removed.

This is a critical design decision. It affects whether Mother is a cache, an
index, or a query router for beliefs. The answer likely influences how other
children work too (repos knowledge, graph edges from shared beliefs, etc.).

### 4. The Embedding Backend Abstraction

Session research uncovered a prior design for making model swaps painless:

- `EmbeddingSpec` struct: `id`, `dim`, `normalize`, `query_prefix`, `passage_prefix`
- `EmbeddingBackend` trait: `spec()`, `embed_query()`, `embed_passage()`
- Every `.usearch` index tagged with `meta.json` containing `embedding_id` + `dim`
- `scry` validates `meta.embedding_id == backend.spec.id` before querying

Today's `EmbeddingEngine` trait already has most of these fields scattered
across `OnnxEmbedder` struct members. The gap is grouping them into
`EmbeddingSpec` and writing/reading `meta.json` alongside indexes.

This matters at scale: `patina scry --all-repos` already merges scores across
20 repos with zero validation that vectors came from the same embedding space.
The `EmbeddingSpec` + `meta.json` is the safety net for that existing feature.

The models child spec ([[mother-environment]]) should incorporate this. The
current mother-environment spec partially covers it (AC 4, 5) but doesn't
reference the full `EmbeddingBackend` abstraction or the `include_str!()`
compile-time registry that needs to become runtime.

### 5. Existing Child Specs Need Revision

The 4 child specs were written before this architecture. Known issues:

**mother-environment (models child):**
- Claims "553MB in git tree" — actually gitignored, only registry.toml tracked
- Registry is `include_str!()` compiled into binary — spec doesn't address this
- AC 3 ("init ensures model") contradicts non-goal ("no auto-download")
- AC 6 overlaps with mother-repos (both claim `oxidize_for_repo()`)
- Daemon warm cache (Solution §5) has no acceptance criterion
- Missing: `models.lock` integration, migration story for existing users

**mother-repos, mother-beliefs, mother-dashboard:** Not yet reviewed against
this architecture. Need same treatment — read spec, trace code, validate ACs.

## Non-Goals

- WASM plugin runtime (that's [[patina-platform]], later)
- Agent protocol or agent coordination (future spec)
- Specific child implementations (separate specs per child)
- Hot-reloading children (restart Mother is fine)
- Child-to-child communication (children talk through Mother, not directly)
