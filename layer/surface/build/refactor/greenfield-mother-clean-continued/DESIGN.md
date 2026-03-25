# Design: Greenfield Mother — separate internal services from child registry

## Why This Design

Mother's beliefs say she is infrastructure — daemon, sandbox, toys, grants. Children are WASM knowledge workers. But the code has native Rust structs masquerading as children through `MotherChild` and `StaticChild` types, while `builtin_dispatch.rs` bypasses the registry entirely. The services already exist; they just need to stop pretending to be children.

This design is primarily **deletion and promotion** — delete the fake child abstractions, promote existing capability code into a `MotherServices` struct that HTTP routes call directly.

Prior art: `greenfield-mother-patina-rebuild` (v0.43.11, 2026-03-24) completed 12 gates in a single session. This spec is 7 gates, smaller scope.

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
6. **`GrantedToys` survives if referenced** — evaluated during GFC-G4.

## Gate Execution Plan

### GFC-G1: Introduce MotherServices with SecretsService

**Entry**: Spec promoted to active, start tag created.

**Commits**:
1. `refactor(mother): scaffold MotherServices with SecretsService` — Create `mother/src/services/mod.rs` and `mother/src/services/secrets.rs`. Migrate TTL cache logic from `SecretsCacheChild` (in-memory `Mutex<Option<CacheEntry>>`, 600s default TTL, get/cache/lock actions).

**Direct code targets**:
- Create `mother/src/services/mod.rs` — `MotherServices` struct aggregating service modules
- Create `mother/src/services/secrets.rs` — `SecretsService` with cache get/set/lock, TTL expiration
- `mother/src/lib.rs` — add `pub mod services`
- `mother/src/http_routes.rs` — wire `/secrets/cache` GET/POST and `/secrets/lock` POST to `SecretsService`

**Exit proofs**:
```
cargo build 2>&1 | tail -5    # expect: no errors
```

**GFC truth map updates**: GFC4 → verified-false (SecretsCacheChild still exists, but SecretsService now parallels it)

---

### GFC-G2: Add SessionStateService and HealthService

**Entry**: GFC-G1 exit proofs pass.

**Commits**:
1. `refactor(mother): add SessionStateService wrapping mother_sessions` — Thin wrapper over `state.rs` session table operations (create, get, list_active, update).
2. `refactor(mother): add HealthService aggregating services + registry` — Reports Mother service health separately from child registry health.

**Direct code targets**:
- Create `mother/src/services/sessions.rs` — wraps `KnowledgeRuntimeStore` session methods
- Create `mother/src/services/health.rs` — aggregates service health + `ChildRegistry::health_all()`
- `mother/src/services/mod.rs` — add fields to `MotherServices`

**Exit proofs**:
```
cargo build 2>&1 | tail -5    # expect: no errors
```

**GFC truth map updates**: GFC8 → verified-partial (MotherServices exists but not yet wired to HTTP routes)

---

### GFC-G3: Wire HTTP routes to MotherServices

**Entry**: GFC-G2 exit proofs pass.

**Commits**:
1. `refactor(mother): route HTTP endpoints through MotherServices` — Update `http_api.rs` `ApiRuntime` to hold `MotherServices`. Route `/health` through `HealthService`, `/secrets/*` through `SecretsService`. Absorb `builtin_dispatch.rs` spec/lake/doctor pattern-matching into service routes.
2. `refactor(mother): update ServerState to hold MotherServices` — `daemon_bootstrap.rs` and CLI `daemon.rs` create `MotherServices` and pass to `ServerState`.

**Direct code targets**:
- `mother/src/http_api.rs` — `ApiRuntime` gains `MotherServices` field, handlers dispatch to services
- `mother/src/http_routes.rs` — routes call service handlers instead of child registry for builtins
- `mother/src/daemon_bootstrap.rs` — construct `MotherServices` alongside `ChildRegistry`
- `src/commands/mother/daemon.rs` — `ServerState` holds `MotherServices`

**Exit proofs**:
```
cargo build 2>&1 | tail -5    # expect: no errors
patina mother start
curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/health | jq .
# parity checks (valid + invalid payloads):
# /child/spec-manager/dispatch
# /child/lake-manager/dispatch
# /child/doctor/run
patina mother stop
```

**GFC truth map updates**: GFC8 → verified-true, GFC9 → verified-partial (logic migrated, file not yet deleted)

---

### GFC-G4: Delete legacy child types

**Entry**: GFC-G3 exit proofs pass. Nothing references deleted types through old paths.

**Commits**:
1. `refactor(mother): remove legacy type usage paths` — remove references/callers first, keep files compiling but unreachable.
2. `refactor(mother): delete MotherChild trait and legacy child implementations` — Remove `MotherChild` from `runtime.rs`, delete `secrets.rs`, `session_writer.rs`, `static_child.rs`. Remove `Toy` struct. Evaluate `GrantedToys` — keep if referenced by `KnowledgeChild` init, delete if orphaned.
3. `refactor(mother): remove dead module exports` — Clean `lib.rs`: remove `secrets`, `session_writer`, `static_child` modules.

**Direct code targets**:
- `mother/src/runtime.rs` — delete `pub trait MotherChild`, `pub struct Toy`
- `mother/src/secrets.rs` — delete file
- `mother/src/session_writer.rs` — delete file
- `mother/src/static_child.rs` — delete file
- `mother/src/lib.rs` — remove module declarations

**Exit proofs**:
```
cargo build 2>&1 | tail -5
rg "MotherChild|StaticChild|SecretsCacheChild|SessionWriterChild" --type rust  # expect: zero hits
```

**GFC truth map updates**: GFC1 → verified-true, GFC2 → verified-true, GFC4 → verified-true, GFC5 → verified-true

---

### GFC-G5: Purge ChildRegistry legacy path

**Entry**: GFC-G4 exit proofs pass.

**Commits**:
1. `refactor(mother): simplify ChildRegistry to KnowledgeChild only` — Remove `legacy_children` vector, `register_legacy()`, `tick_legacy_all()`. Single `children` vector.
2. `refactor(mother): remove legacy_migration flag and heartbeat branch` — Remove flag from `DaemonBootstrapConfig`. Remove legacy branch from `daemon_heartbeat.rs`.

**Direct code targets**:
- `mother/src/registry.rs` — delete `legacy_children`, `register_legacy()`, `tick_legacy_all()`
- `mother/src/daemon_heartbeat.rs` — remove `legacy_migration` parameter and toy-subprocess branch
- `mother/src/daemon_bootstrap_config.rs` — remove `legacy_migration` field

**Exit proofs**:
```
cargo build 2>&1 | tail -5
rg "legacy_children|legacy_migration|tick_legacy" --type rust  # expect: zero hits
patina mother start
curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/health | jq .
patina mother stop
```

**GFC truth map updates**: GFC3 → verified-true, GFC6 → verified-true

---

### GFC-G6: Delete daemon.rs protocol v1

**Entry**: GFC-G5 exit proofs pass.

**Commits**:
1. `refactor(mother): delete protocol v1 socket handler` — Remove `mother/src/daemon.rs`. Remove from `lib.rs`. Mother speaks HTTP only.

**Direct code targets**:
- `mother/src/daemon.rs` — delete file
- `mother/src/lib.rs` — remove `pub mod daemon`
- Any references to `DaemonState` — remove or redirect

**Exit proofs**:
```
cargo build 2>&1 | tail -5
rg "DaemonState|route_request|handle_action" --type rust mother/  # expect: zero hits
```

**GFC truth map updates**: GFC7 → verified-true

---

### GFC-G7: Final cleanup and verify

**Entry**: GFC-G6 exit proofs pass.

**Commits**:
1. `refactor(mother): delete builtin_dispatch.rs` — Remove from CLI. Logic already absorbed in GFC-G3.
2. `refactor(mother): update daemon_bootstrap to services-only` — Remove `register_builtin_children()`. Bootstrap creates `MotherServices` + empty `ChildRegistry` (populated only by WASM loading).
3. `refactor(mother): fix tests` — Update or remove tests that referenced deleted types.

**Direct code targets**:
- `src/commands/mother/builtin_dispatch.rs` — delete file
- `src/commands/mother/mod.rs` — remove `builtin_dispatch` module
- `mother/src/daemon_bootstrap.rs` — remove `register_builtin_children()` fn
- `mother/src/builtin_children.rs` — delete if exists (registration logic)
- Test files referencing `MotherChild`, `StaticChild`, etc.

**Exit proofs**:
```
cargo build 2>&1 | tail -5                    # expect: no errors
cargo test 2>&1 | tail -10                    # expect: all pass
rg "MotherChild|StaticChild|legacy_migration|legacy_children|builtin_dispatch" --type rust  # expect: zero
```

**GFC truth map updates**: GFC9 → verified-true, GFC10 → verified-true, GFC11 → unverified (needs daemon smoke test), GFC12 → verified-true

**Final smoke test** (manual or scripted):
```
patina mother start &
curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/health | jq .
# expect: status: "ok", services listed, children: [] (no builtins)
patina mother stop
```

GFC11 → verified-true after smoke test passes.

---

## Rollback Protocol

Each gate is independently revertable via `git revert`. If a gate introduces compilation failures that can't be resolved within the gate:
1. Revert the gate's commits
2. Update the truth map with what was learned
3. Revise the gate's plan before retrying

## Open Questions

- **`toys.rs` scope**: The `Toy` struct (shell command spawning) dies with `MotherChild`. But `GrantedToys` (capability declarations for WASM children) may still be needed. Resolved during GFC-G4 based on grep evidence.
- **Scry backend**: Currently wired through `ApiRuntime`. Should it become a `ScryService` in `MotherServices`? Likely yes but not blocking — follow-up spec.
- **`mother/src/broker/`**: Already service-shaped. May be re-exported under `MotherServices` for consistency during GFC-G3 if natural, otherwise follow-up.

## Edge Validation Notes

- Auth/permission checks for HTTP are in `mother/src/http_routes.rs` (`check_auth` + 401 guards), not in `src/commands/mother/builtin_dispatch.rs`.
- `legacy_migration` currently spans CLI/mother launch and heartbeat wiring; removal must cover all callsites, not just heartbeat internals.
