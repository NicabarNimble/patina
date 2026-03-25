# Design: Greenfield Mother: separate internal services from child registry

## Why This Design

Mother's beliefs say she is infrastructure — daemon, sandbox, toys, grants. Children are WASM knowledge workers. But the code has native Rust structs masquerading as children through `MotherChild` and `StaticChild` types, while `builtin_dispatch.rs` bypasses the registry entirely. The services already exist; they just need to stop pretending to be children.

This design is primarily **deletion and promotion** — delete the fake child abstractions, promote existing capability code into a `MotherServices` struct that HTTP routes call directly.

## Build Target

A Mother daemon where:
- `MotherServices` owns internal capabilities (secrets, sessions, health, broker, lakes, specs)
- `ChildRegistry` holds only WASM `KnowledgeChild` instances
- One transport (HTTP over UDS/TCP), one heartbeat path (knowledge cycles only)
- No `MotherChild` trait, no `StaticChild`, no `--legacy-migration`

## Resolved Decisions

1. **Internal services, not children** — secrets caching, session state, health, specs, lakes are Mother's own modules. They don't go through a registry.
2. **KnowledgeChild stays** — the drain → tick → handle lifecycle is the right model for WASM guests. Only the legacy `MotherChild` trait (tick → Toys) is removed.
3. **HTTP is the sole transport** — `daemon.rs` (protocol v1 socket handler) is dead code and deleted.
4. **builtin_dispatch.rs absorbed** — its pattern-matching logic moves into service-backed HTTP routes.
5. **Session dual-bookkeeping preserved** — artifacts in `layer/sessions/` (Patina) + `mother_sessions` table (Mother state machine). Both are correct; the session-writer "child" in between is not.

## Commits

1. `refactor(mother): introduce MotherServices struct with SecretsService` — Create `mother/src/services/mod.rs` with `MotherServices` and `SecretsService`. Migrate TTL cache logic from `SecretsCacheChild` into `SecretsService`. Wire secrets HTTP routes to call service directly.

2. `refactor(mother): add SessionStateService and HealthService` — Add session state operations and health aggregation as internal services. Health now queries `MotherServices` health + `ChildRegistry` health separately.

3. `refactor(mother): wire HTTP routes to MotherServices` — Update `http_routes.rs` and `http_api.rs` to route `/health`, `/secrets/*`, builtin child actions through `MotherServices`. Absorb `builtin_dispatch.rs` logic into service routes.

4. `refactor(mother): delete MotherChild trait and legacy children` — Remove `MotherChild` trait from `runtime.rs`. Delete `secrets.rs` (SecretsCacheChild), `session_writer.rs` (SessionWriterChild), `static_child.rs` (StaticChild). Remove `legacy_children` from `ChildRegistry` and `register_legacy()`.

5. `refactor(mother): remove legacy heartbeat and migration flag` — Remove `--legacy-migration` flag from `DaemonBootstrapConfig`. Remove legacy branch from `daemon_heartbeat.rs`. Heartbeat now only runs `run_knowledge_cycles()`.

6. `refactor(mother): delete daemon.rs protocol v1 handler` — Remove the old Unix socket line-based protocol handler. Mother speaks HTTP only (over UDS or TCP).

7. `refactor(mother): update daemon_bootstrap to services + registry` — `daemon_bootstrap.rs` creates `MotherServices` and a clean `ChildRegistry` (WASM only, no builtins). Update `ServerState` to hold both.

8. `refactor(mother): update CLI integration and tests` — Update `src/commands/mother/` to use new service structure. Remove `builtin_dispatch.rs`. Update all tests to reflect removed types.

## Direct Code Targets

### Mother crate (`mother/src/`)
- `runtime.rs` — Delete `MotherChild` trait, `Toy` struct. Keep `KnowledgeChild`, `TaskIntent`, `PendingEvent`.
- `registry.rs` — Remove `legacy_children` vector, `register_legacy()`, `tick_legacy_all()`. Simplify to single `children` vector.
- `secrets.rs` — Delete entirely (SecretsCacheChild). Logic migrates to `services/secrets.rs`.
- `session_writer.rs` — Delete entirely.
- `static_child.rs` — Delete entirely.
- `toys.rs` — Evaluate: `GrantedToys` may still be needed for WASM child capability grants. `Toy` struct (shell command) is deleted.
- `daemon.rs` — Delete entirely (protocol v1 handler).
- `daemon_bootstrap.rs` — Remove `register_builtin_children()`. Create `MotherServices` instead.
- `daemon_heartbeat.rs` — Remove `legacy_migration` parameter and legacy branch.
- `daemon_bootstrap_config.rs` — Remove `legacy_migration` field.
- `http_api.rs` — Update `ApiRuntime` to take `MotherServices` + `ChildRegistry`.
- `http_routes.rs` — Add service routes, keep `/child/<name>/*` for WASM only.
- `lib.rs` — Update module exports: add `services`, remove `secrets`, `session_writer`, `static_child`.

### New files
- `mother/src/services/mod.rs` — `MotherServices` struct, aggregates all services
- `mother/src/services/secrets.rs` — `SecretsService` (TTL cache, wraps authority backend)
- `mother/src/services/sessions.rs` — `SessionStateService` (wraps mother_sessions table)
- `mother/src/services/health.rs` — `HealthService` (Mother + child health aggregation)

### CLI integration (`src/commands/mother/`)
- `builtin_dispatch.rs` — Delete entirely
- `daemon.rs` — Update `ServerState` to hold `MotherServices` + `ChildRegistry`

## Verification Plan

1. **Compile check**: `cargo build` with no references to `MotherChild`, `StaticChild`, `SecretsCacheChild`, `SessionWriterChild`
2. **Test suite**: `cargo test` — all mother crate tests pass or are updated
3. **Daemon smoke test**: `patina mother start` → `/health` returns service + child status
4. **Session smoke test**: `patina ai claude` → session starts, wrapper scripts work
5. **Secrets smoke test**: cache/get/lock operations through `SecretsService`
6. **Broker smoke test**: `patina mother run` sources still work
7. **Grep verification**: `rg "MotherChild|StaticChild|legacy_migration|legacy_children"` returns zero hits

## Build Readiness

All architecture decisions are resolved. The code surfaces are well-mapped. Each commit is scoped to a single concern. The risk is low — this is primarily deletion of unused/stubbed code and promotion of working code from child wrappers into direct service modules. The capabilities themselves don't change, only how they're called.

## Open Questions

- **`toys.rs` scope**: The `Toy` struct (shell command spawning) dies with `MotherChild`. But `GrantedToys` (capability declarations for WASM children) may still be needed. Evaluate during commit 4 — keep `GrantedToys` if referenced by `KnowledgeChild` init, delete `Toy`.
- **Scry backend**: Currently wired through `ApiRuntime`. Should it become a `ScryService` in `MotherServices`? Likely yes but not blocking — can be a follow-up.
- **`mother/src/broker/`**: Broker code is already service-shaped (not a child). It may just need to be re-exported under `MotherServices` for consistency. Evaluate during commit 3.
