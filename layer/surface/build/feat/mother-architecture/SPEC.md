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

From Anton Logvinov's "Large Arrays of Things" (LOATs): don't build deep
hierarchies of ownership. Keep things flat, same shape, simple relationships.
Mother is the array. Children are the things.

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
    fn handle(&self, request: &Request) -> Result<Response>;

    /// Heartbeat tick — child checks its own state, may launch toys
    fn tick(&mut self) -> Vec<Toy>;
}

pub enum ChildHealth {
    Healthy,
    Degraded(String),  // working but something's off
    Unavailable(String), // can't serve requests
}

/// A toy is a process a child wants Mother to launch
pub struct Toy {
    pub name: String,
    pub command: ToyCommand,
    pub on_complete: Box<dyn FnOnce(ToyResult) + Send>,
}

pub enum ToyCommand {
    /// Shell command (e.g., patina oxidize --repo gastown)
    Shell(Vec<String>),
    /// Agent in a yolo container
    Agent { adapter: String, context: String, task: String },
}
```

## Mother's Self

Mother's own responsibilities (not delegated to children):

1. **Daemon lifecycle** — start, stop, PID file, socket
2. **Child registry** — know all children, iterate them
3. **Heartbeat loop** — periodically tick each child, launch any toys
4. **Request routing** — receive requests via UDS/MCP, route to the right child
5. **Toy management** — spawn processes children request, monitor, collect results

Mother does NOT know about models, repos, beliefs, or secrets directly.
She just knows she has children and she iterates them.

## Toys

A toy is a process that a child asks Mother to launch. The child decides
*what* needs doing. Mother handles *how* to launch it.

Examples:
- Repos child detects gastown is 30 days stale → requests a toy:
  `Shell(["patina", "oxidize", "--repo", "gastown"])`
- Beliefs child detects drift between projects → requests a toy:
  `Agent { adapter: "codex", context: "layer/sessions/...", task: "investigate belief X" }`

Mother spawns the toy, monitors it, and calls `on_complete` when done.
Children never spawn processes directly.

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

## Acceptance Criteria

1. [ ] `MotherChild` trait defined in `src/mother/mod.rs`
2. [ ] Mother daemon iterates registered children on heartbeat
3. [ ] At least one child (models) implemented behind the trait
4. [ ] Mother routes requests to children by name
5. [ ] `Toy` struct defined — children can request process launches
6. [ ] Mother can spawn and monitor a toy (shell command)
7. [ ] `patina mother status` shows all children and their health

## Non-Goals

- WASM plugin runtime (that's [[patina-platform]], later)
- Agent protocol or agent coordination (future spec)
- Specific child implementations (separate specs per child)
- Hot-reloading children (restart Mother is fine)
- Child-to-child communication (children talk through Mother, not directly)
