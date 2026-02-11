---
type: feat
id: mother-architecture
status: design
created: 2026-02-10
sessions:
  origin: 20260210-061323
  resolved: 20260210-104448
related:
  - layer/surface/build/feat/mother/SPEC.md
  - layer/surface/build/feat/patina-platform/SPEC.md
beliefs:
  - mother-is-the-daemon
  - four-layer-architecture
  - unix-philosophy
  - simplicity-is-architecture
  - separate-worlds-for-isolation
---

# feat: Mother Architecture

> Projects vs Mother. Projects are git repos with a patina layer — sovereign,
> self-contained. Mother is the central system that connects knowledge between
> projects, holds ref repos, manages machine infrastructure, and carries
> user/persona knowledge.
>
> Mother's connection architecture is called Children. Children are future WASM
> WIT plugins, but initially they follow the [[dependable-rust]] design.

## Problem

Mother has real responsibilities (models, repos, secrets, cross-project knowledge)
but no defined architecture. The result is disorganization — code sprawls across
`src/mother/`, `src/commands/`, and `src/models/` with unclear ownership
boundaries. Previous attempts at structure (Mother v2) failed because they
designed scaffolding without first defining the architecture.

## Portable vs Rebuildable

Everything under `cache/` is machine-local and rebuildable from portable state.
Each child declares what state it owns and whether it's portable or cache. A
future `patina mother rebuild` asks each child to re-derive its cache.

## Children Are Plugins

Mother's responsibilities are organized as **children** — plugins that run in
Mother's daemon context. A child is a black box that owns some state and handles
requests. Mother iterates children, routes to them, doesn't know their internals.

Children are not a fixed list. Any module that implements the shape is a child.
Today, children are native Rust behind traits ([[dependable-rust]] pattern).
The trait maps to a WIT world — when [[patina-platform]] lands, children
become WASM plugins. Same shape at both stages.

### The Trait

```rust
/// [[dependable-rust]] pattern: small stable interface, private internals.
pub trait MotherChild: Send + Sync {
    /// Identity
    fn name(&self) -> &str;

    /// Lifecycle — called when Mother loads this child
    fn on_load(&mut self, host: &dyn MotherHost) -> Result<()>;

    /// Lifecycle — called when Mother shuts down
    fn on_unload(&mut self) {}

    /// Health check — Mother calls this on heartbeat
    fn health(&self) -> ChildHealth;

    /// Handle a request routed by Mother
    fn handle(&self, request: &ChildRequest) -> Result<ChildResponse>;

    /// Heartbeat tick — child checks its own state, may request toys.
    /// Default: no-op (children that don't need periodic checks skip this).
    fn tick(&mut self) -> Vec<Toy> { vec![] }
}
```

### Host Capabilities

Mother provides children with scoped capabilities:

- **Storage** — read/write within the child's owned paths
- **Config** — key-value configuration
- **Logging** — structured event logging
- **Work** — request Mother to spawn processes (toys)

A child only sees what the host exposes. It cannot reach into other children's
state or touch project internals. Exact API emerges from implementing the first
child.

### Toys

A child can ask Mother to run work — shell commands, agents, whatever.
The child decides *what*. Mother handles *how*. Children never spawn
processes directly.

## Mother's Self

Mother is the plugin host. Her responsibilities:

1. **Daemon lifecycle** — start, stop, PID file, socket
2. **Child registry** — load children, iterate them
3. **Heartbeat loop** — tick each child, run any toys
4. **Request routing** — receive requests via UDS/MCP, route to children
5. **Toy management** — spawn processes children request, monitor results

## Belief Mobility

Beliefs are born in projects — the user and LLM discuss, learn, conclude,
grounded by scry (semantic) and assay (structural). Some beliefs travel up
to Mother when the user decides "this is true about me, not just this project."
Mother holds promoted beliefs at `~/.patina/layer/surface/beliefs/`. Projects
are sovereign — they can always disagree.

Mother queries project beliefs **on demand** — read-only, at query time, no
copies, no sync. Projects stay the single source of truth for their own beliefs.

## Starting Children

These are the first children. They are not special — future children are added
the same way.

| Child | "Do X" | State |
|-------|--------|-------|
| models | Resolve embedding models | `cache/models/`, `models.lock` |
| repos | Maintain ref repo knowledge | `cache/repos/`, `registry.yaml` |
| secrets | Cache decrypted secrets | RAM (daemon) |
| persona | Hold user knowledge across projects | `layer/surface/beliefs/`, persona events |

Each child gets its own spec for internals. This spec only defines the frame.

## Current State

Today `src/mother/` has three files:
- `mod.rs` — public interface, daemon start
- `internal.rs` — UDS client for daemon communication
- `graph.rs` — cross-project graph (31 nodes, 3 edges)

The daemon runs as a background process, listens on a UDS socket, and today does
two things: proxies scry requests (dead path — MCP bypasses it) and caches
decrypted secrets (sole real consumer).

This architecture refactors the daemon from a monolithic handler into a
child-iterating loop.

## Evolution Path

```
Phase 1 (now):   Native Rust traits behind [[dependable-rust]] pattern.
Phase 2 (later): WIT interfaces. Same shape, WASM plugins via wasmtime.
                 See [[patina-platform]] for plugin infrastructure.
```

## Acceptance Criteria

1. [ ] `MotherChild` trait defined in `src/mother/`
2. [ ] Mother daemon loads and iterates registered children
3. [ ] At least one child implemented behind the trait
4. [ ] Mother routes requests to children by name
5. [ ] Children can request toys (Mother spawns and monitors)
6. [ ] `patina mother status` shows all children and their health

## Non-Goals

- WASM plugin runtime (that's [[patina-platform]], later)
- Fixed child list (children are plugins, not a hardcoded enum)
- Child-to-child communication (children talk through Mother, not directly)
- Hot-reloading children (restart Mother is fine)
- Belief sync/copy (Mother reads on demand, doesn't replicate)
- Agent protocol or agent coordination (future spec)
