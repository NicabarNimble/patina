---
type: refactor
id: greenfield-mother-clean-continued
status: draft
created: 2026-03-25
sessions:
  origin: 20260325-064204-876122000
exit_criteria:
  - MotherChild trait deleted from codebase
  - StaticChild type deleted from codebase
  - ChildRegistry holds only KnowledgeChild (WASM guests)
  - legacy_children vector removed from ChildRegistry
  - --legacy-migration flag removed from daemon startup
  - Legacy heartbeat branch (toy subprocess spawning) removed
  - daemon.rs (old protocol v1 socket handler) removed
  - session_writer.rs (native placeholder child) removed
  - secrets.rs (MotherChild version) removed
  - MotherServices struct (or equivalent) owns secrets, sessions, health, broker, lakes, specs
  - HTTP routes call MotherServices directly, not through child registry
  - builtin_dispatch.rs eliminated — its logic absorbed into MotherServices routes
  - All existing tests pass or are updated to reflect new structure
  - patina mother start still works (UDS + TCP)
  - patina ai claude still launches and creates sessions
  - Secrets caching still works (authority backend preserved)
---
# refactor: Greenfield Mother: separate internal services from child registry

> Mother's code conflates internal services (secrets, sessions, health, specs, lakes) with the child abstraction. Native Rust structs masquerade as children via MotherChild trait and StaticChild markers, while the actual child registry should hold only WASM guests. This refactor draws a clean line: Mother owns internal services directly, ChildRegistry holds only KnowledgeChild WASM instances.

## Problem

Mother's four-role belief (`[[four-roles-no-overlap]]`) says:

- **Patina** = belief layer (knowledge protocol)
- **Mother** = infrastructure (daemon, sandbox, toys, grants)
- **Children** = knowledge workers (WASM, sandboxed, use SDK)
- **Projects** = development zone

But the code doesn't honor this boundary. The `ChildRegistry` in `mother/src/registry.rs` holds two vectors:
- `knowledge_children: Vec<KnowledgeChild>` — intended for WASM guests
- `legacy_children: Vec<MotherChild>` — native Rust structs pretending to be children

Six "children" are registered at daemon startup that are not children at all:
1. `secrets` (SecretsCacheChild) — in-memory TTL cache, native Rust
2. `session-writer` — placeholder that returns "healthy" and nothing else
3. `spec-manager` — StaticChild marker, no-op
4. `doctor` — StaticChild marker, no-op
5. `lake-manager` — StaticChild marker, no-op
6. `secrets-authority` — StaticChild marker, no-op

Meanwhile, `builtin_dispatch.rs` in the CLI doesn't even use the registry for these — it pattern-matches on child name and routes directly to CLI code. They are **already internal services pretending to be children**.

Additionally, `daemon.rs` implements a pre-v1 Unix socket protocol that is mostly stubbed ("not yet implemented") while `http_daemon.rs` is the actual working transport. Two parallel transports exist for no reason.

The heartbeat thread has a `--legacy-migration` branch that spawns `Toy` structs as shell subprocesses — a model superseded by the WASM capability grant design.

## Goal

Draw a clean, permanent line between Mother's internal services and the child registry. After this refactor:
- Mother owns and calls her internal services directly
- ChildRegistry is exclusively for WASM KnowledgeChild guests
- One transport (HTTP over UDS/TCP)
- One heartbeat path (knowledge child cycles only)
- The codebase reflects the four-role architecture from our beliefs

## Status

Draft. Architecture alignment confirmed in session 20260325-064204-876122000. This continues the greenfield Mother work from the `mother-child-toy-beliefs-layout` refactor spec (March 13, 2026) and the toybox framework design from session 20260324-101606-299953000.

## Non-Goals

- Wiring actual WASM child loading (that's Phase 3 — unblocked by this work but not part of it)
- Changing the session artifact format or `patina ai` entry point (these already work)
- Redesigning the secrets authority backend (it's solid, just needs to stop being a "child")
- Implementing the stubbed daemon actions (scry, context, measure, spec, lake)
- Multi-Mother federation
- Token management or context window aware sessions (separate concern)

## Current State

### Mother crate (`mother/src/`)
- `runtime.rs` defines two traits: `KnowledgeChild` (drain/tick/handle) and `MotherChild` (tick→Toys)
- `registry.rs` holds both in parallel vectors with dual-path orchestration
- `secrets.rs` implements `MotherChild` for in-memory cache
- `session_writer.rs` implements `MotherChild` as a no-op placeholder
- `static_child.rs` creates named healthy markers
- `daemon_bootstrap.rs` registers all six fake children before accepting connections
- `daemon_heartbeat.rs` branches on `legacy_migration` flag
- `daemon.rs` implements a second transport (Unix socket, line-based protocol) that is mostly stubbed
- `http_daemon.rs` + `http_routes.rs` + `http_api.rs` implement the actual working HTTP transport

### CLI integration (`src/commands/mother/`)
- `builtin_dispatch.rs` routes "child" requests by pattern-matching names, bypassing the registry entirely
- `daemon.rs` wires up `ServerState` with registry + scry backend
- Internal HTTP client talks to the HTTP transport

### What works today
- `patina ai <interface>` entry point (claude, opencode, gemini)
- Session lifecycle (start/update/note/end) via wrapper scripts
- Interface bundle deployment with tarball-style reconciliation
- Secrets authority backend (vault, keychain, encryption)
- HTTP daemon (UDS + TCP) with health, scry, secrets, child dispatch routes
- Broker source configuration and cursor management
- Session dual-bookkeeping (artifacts in layer/ + state in mother_sessions table)

## Target State

```
Mother (daemon process)
├── MotherServices (internal, native Rust)
│   ├── SecretsService      — wraps secrets_authority_backend (cache + vault)
│   ├── SessionStateService — wraps mother_sessions table operations
│   ├── HealthService       — aggregates Mother health + child health from registry
│   ├── BrokerService       — wraps broker/ (source routing, cursors)
│   ├── LakeService         — wraps lake dispatch
│   └── SpecService         — wraps spec dispatch
│
├── HTTP Transport (single, http_daemon.rs)
│   ├── /health             → HealthService
│   ├── /secrets/*          → SecretsService
│   ├── /api/scry           → ScryBackend
│   ├── /api/spec/*         → SpecService
│   ├── /api/lake/*         → LakeService
│   └── /child/<name>/*     → ChildRegistry (WASM only)
│
└── ChildRegistry (WASM guests only)
    ├── KnowledgeChild instances loaded from ~/.patina/children/
    └── Heartbeat: drain → tick → handle (no legacy branch)
```

## Solution

### Phase 1: Create MotherServices and migrate internal capabilities

Introduce a `MotherServices` struct in the mother crate that owns the internal capabilities. Move secrets caching from `SecretsCacheChild` into `SecretsService`. Move session state operations into `SessionStateService`. Wire HTTP routes to call services directly.

### Phase 2: Purge legacy child abstractions

Delete `MotherChild` trait, `StaticChild`, `SecretsCacheChild`, `SessionWriterChild`. Remove `legacy_children` from `ChildRegistry`. Remove `--legacy-migration` flag and heartbeat legacy branch. Remove `daemon.rs` (old protocol v1 handler). Remove `builtin_dispatch.rs` (absorbed into service routes).

### Phase 3: Simplify registry and heartbeat

`ChildRegistry` becomes a single vector of `KnowledgeChild`. Heartbeat runs only `run_knowledge_cycles()`. `ApiRuntime` trait updated to reflect services + registry separation.

## Implementation Order

1. Create `MotherServices` struct with service modules
2. Migrate secrets cache logic into `SecretsService` (preserve TTL behavior)
3. Migrate session state ops into `SessionStateService`
4. Wire HTTP routes to `MotherServices` for secrets, health, builtin dispatch
5. Remove `builtin_dispatch.rs` — logic now lives in service-backed routes
6. Delete `MotherChild` trait from `runtime.rs`
7. Delete `SecretsCacheChild` (`secrets.rs`)
8. Delete `SessionWriterChild` (`session_writer.rs`)
9. Delete `StaticChild` (`static_child.rs`)
10. Remove `legacy_children` from `ChildRegistry`, remove `register_legacy()`
11. Remove `--legacy-migration` flag from `DaemonBootstrapConfig`
12. Remove legacy heartbeat branch from `daemon_heartbeat.rs`
13. Delete `daemon.rs` (old protocol v1 socket handler)
14. Update `daemon_bootstrap.rs` to register only WASM children (no builtins)
15. Update `ApiRuntime` trait to use `MotherServices` + `ChildRegistry`
16. Update all tests

## Resolved Decisions

- **Mother has internal services + external children** (not "everything is a child"). Confirmed in session 20260325-064204-876122000. Aligns with `[[four-roles-no-overlap]]`, `[[children-have-agency-toys-are-capabilities]]`, and `[[core-verbs-standalone-mother-additive]]`.
- **`KnowledgeChild` lifecycle stays** (drain → tick → handle). This is the right model for WASM guests. Only `MotherChild` (tick → Toys) goes away.
- **HTTP is the single transport**. The old `daemon.rs` line-based protocol is dead code.
- **Session dual-bookkeeping is correct** — artifacts in `layer/sessions/` (Patina knowledge) + state in `mother_sessions` (Mother state machine). The session-writer "child" in the middle adds nothing and is removed.

## Verification

- `cargo build` succeeds with no references to deleted types
- `cargo test` passes (mother crate + integration tests)
- `patina mother start` launches daemon, `/health` returns OK
- `patina mother status` reports services + any loaded children
- `patina ai claude` launches, creates session, session-start returns JSON
- Secrets: cache/get/lock operations work through SecretsService
- No regression in `patina mother run` (broker sources)

## Exit Criteria

- [ ] `MotherChild` trait deleted from codebase
- [ ] `StaticChild` type deleted from codebase
- [ ] `ChildRegistry` holds only `KnowledgeChild` (WASM guests)
- [ ] `legacy_children` vector removed from `ChildRegistry`
- [ ] `--legacy-migration` flag removed from daemon startup
- [ ] Legacy heartbeat branch (toy subprocess spawning) removed
- [ ] `daemon.rs` (old protocol v1 socket handler) removed
- [ ] `session_writer.rs` (native placeholder child) removed
- [ ] `secrets.rs` (MotherChild version) removed
- [ ] `MotherServices` struct (or equivalent) owns secrets, sessions, health, broker, lakes, specs
- [ ] HTTP routes call `MotherServices` directly, not through child registry
- [ ] `builtin_dispatch.rs` eliminated — its logic absorbed into `MotherServices` routes
- [ ] All existing tests pass or are updated to reflect new structure
- [ ] `patina mother start` still works (UDS + TCP)
- [ ] `patina ai claude` still launches and creates sessions
- [ ] Secrets caching still works (authority backend preserved)

## Build Readiness

Beliefs are aligned. Architecture is agreed. The code surfaces to change are well-understood. This is primarily deletion and reorganization — the capabilities already work, they just need to stop routing through the child registry. Ready for DESIGN.md and implementation.
