---
type: feat
id: mother-pando-bindings-runtime
status: active
created: 2026-04-07
blocks:
- mother-duckdb-ducklake-federation
sessions:
  origin: 20260407-063612-748374000
related:
- src/commands/mother/daemon.rs
- src/commands/mother/mod.rs
- src/commands/mother/loader.rs
- src/mother/internal.rs
- mother/src/http_api.rs
- mother/src/http_routes.rs
- mother/src/registry.rs
- layer/surface/build/feat/mother-duckdb-ducklake-federation/SPEC.md
beliefs:
- '[[core-verbs-standalone-mother-additive]]'
- '[[children-have-agency-toys-are-capabilities]]'
- '[[adapter-pattern]]'
exit_criteria:
- id: mpbr1-two-phase-startup
  text: 'Mother startup is split into control-plane (Phase 1) and child activation (Phase 2). Phase 1 completes state_db_open, federation startup, transport bootstrap, and health/status availability. Phase 2 runs child_discovery and registry_load_all in a background task. The startup assertion: health endpoint returns 200 before any child `on_load` is called.'
  checked: true
- id: mpbr2-bindings-contract
  text: 'A `MotherRuntime` trait in `mother/src/runtime.rs` defines typed methods for pando-child-toy composition: `load_pando`, `refresh_pandos`, `reload_child`, `query_readiness`. Daemon internals call these methods, not CLI commands. CLI is a thin client over HTTP endpoints that invoke these same methods.'
  checked: true
- id: mpbr3-lifecycle-ops
  text: 'Three lifecycle operations exist: `load_pando(name)` registers/activates a pando composition, `refresh_pandos()` rescans and reconciles all pandos, `reload_child(name)` drains the old instance and loads the new one. Reload keeps the previous instance serving until the new one passes health check; on failure, the old instance stays active with its existing health status (not degraded — the reload attempt failed, not the running instance). All three are idempotent: calling with same state produces same result. Concurrent operations on the same child are rejected with 409, not queued.'
  checked: true
- id: mpbr4-startup-observability
  text: 'Per-stage startup metrics emitted as `measure.metric` events: `mother:startup:stage_latency_ms` (gauge, labels: scope=startup, action={stage}), `mother:startup:stage_failure` (counter). Per-child activation metrics: `mother:startup:child_activation_ms` (gauge, labels: scope=startup, action=child_activate, child={name}), `mother:startup:child_activation_failure` (counter). Lifecycle operation metrics: `mother:lifecycle:{op}_latency_ms` (gauge), `mother:lifecycle:{op}_failure` (counter).'
  checked: true
- id: mpbr5-readiness-surface
  text: 'Health endpoint response includes readiness fields with exact names: `control_plane_ready` (bool), `children_ready_count` (usize), `children_total` (usize), `children_degraded` (array of `{name, reason}` objects). Existing fields unchanged — additive only. State transitions: `control_plane_ready` false→true once, never reverts; `children_total` set after discovery, changes only on refresh; `children_ready_count` monotonically increases during warmup, can decrease on refresh (re-discovery); `children_degraded` entries added on activation failure, removed on successful reload. CLI `patina mother status` surfaces all readiness fields in the same phase as the API — no lag.'
  checked: true
- id: mpbr6-manifest-integrity
  text: 'Mother computes SHA-256 of all pando and child artifacts at install/seed time: pando.toml, child.toml, and .wasm binaries. Hashes written alongside (e.g. pando.toml.sha256, child.toml.sha256, slate-manager.wasm.sha256). At load time, Mother recomputes and compares all hashes before granting any bindings or instantiating any WASM module. Any mismatch refuses to load with diagnostic. First-party pandos recompute hashes from binary''s embedded copy during seeding.'
  checked: true
- id: mpbr7-proof
  text: 'Proof passes: `cargo check --workspace -q`, `cargo test -q --lib`, and a startup test asserting health returns 200 with `control_plane_ready: true` before child activation begins. No hard timing SLA — the structural assertion is that transport is listening before `on_load` runs.'
  checked: true
---

# feat: Mother Pando Bindings Runtime

## Problem

Mother startup pays full child discovery/load cost on the critical path. In
`daemon.rs:554`, the sequence is: `state_db_open` → `federation startup` →
`child_discovery` → `registry_load_all` → `router_build` → `transport_bootstrap`.
Transport only starts after all children are loaded. With WASM compilation and
multiple children, this creates visible hangs before health is available.

At the same time, runtime composition trends toward CLI-driven orchestration.
CLI is the right operator interface, but daemon internals should use typed
in-process calls for pando/child/toy composition on hot paths.

## Goal

1. Control plane available quickly (health returns 200 before child warmup).
2. Child and pando composition uses typed in-process methods.
3. Lifecycle operations are explicit, observable, and safe.
4. Strong observability at every startup stage and lifecycle boundary.

## Status

Draft. Blocks `mother-duckdb-ducklake-federation` until the runtime composition
model is landed, so federation query surface and lifecycle align with the same
readiness boundaries.

## Non-Goals

- Replacing Patina CLI as user interface.
- Cross-machine orchestration.
- Redesigning toy semantics or child manifest schema.
- Speculative abstractions without 2+ concrete call sites.
- Hard timing SLA (e.g. "ready in 50ms") — the assertion is structural, not temporal.

## Resolved Decisions

### Two-phase startup is structural, not temporal

Phase 1 (control plane): `state_db_open`, federation startup, transport bootstrap,
health/status endpoints. Phase 2 (child activation): `child_discovery`,
`registry_load_all`, pando registration. The startup assertion: **health endpoint
returns 200 before any child `on_load` is called.** This is a structural ordering
guarantee, not a timing target.

### CLI is operator UX, not internal execution boundary

Lifecycle operations (`load_pando`, `refresh_pandos`, `reload_child`) are methods
on a `MotherRuntime` trait in `mother/src/runtime.rs`. HTTP endpoints call these
methods. CLI calls HTTP endpoints. The daemon never shells out to CLI for hot-path
execution.

### Child reload is by canonical name

`reload_child(name)` uses the child's canonical name from its manifest. Pando-level
aliases are resolved by the pando layer before calling reload. No alias selectors
at the runtime API level.

### HTTP first, CLI as thin client

Lifecycle endpoints land as HTTP routes first. CLI commands are thin clients that
call those endpoints (same pattern as federation). Both land in the same spec but
HTTP is the primary contract.

### Reload keeps previous instance until new passes health

`reload_child(name)`:
1. Load new WASM module
2. Call `on_load` on new instance
3. If `on_load` succeeds: drain old instance, swap in new, mark healthy
4. If `on_load` fails: discard new instance, keep old running
5. Child state in `runtime.db` is unaffected — it belongs to the child name, not
   the instance

**Key lock:** On reload failure, the old instance stays active with its
**existing health status** — it is not marked degraded. The reload attempt
failed, not the running instance. The failure is logged, emitted as a metric,
and returned as a 200 response with `status: "reload_failed"` (not 500 — the
operation completed deterministically). The child's operational status doesn't change.
Degraded status only applies when a child's *own* `on_load` fails during initial
activation (warmup) — not when a reload of a replacement fails.

### Concurrency: reject, don't queue

Background warmup and manual reload/refresh share a per-child mutex. The
client-visible behavior when contention occurs:

| Scenario | Behavior |
|----------|----------|
| Two reload requests for same child | Second returns 409 `operation_in_progress` |
| Reload request during warmup activation of same child | Returns 409 `operation_in_progress` |
| Reload of child A while child B is reloading | Both proceed concurrently |
| `refresh_pandos` while any child reload is in progress | Returns 409 `operation_in_progress` |
| Two concurrent `refresh_pandos` calls | Second returns 409 `operation_in_progress` |

No hidden queues, no coalescing. The caller sees a deterministic response and
can retry. This is simpler than a queue and sufficient for a single-daemon model.

### Error envelope for lifecycle operations

One error shape for all lifecycle HTTP responses:

```json
{
  "error": "child_not_found",
  "code": 404,
  "detail": "no child named 'lakehouse-catalog'"
}
```

HTTP status matrix:

| Status | `error` code | When |
|--------|-------------|------|
| 400 | `invalid_request` | Missing or malformed parameters |
| 404 | `child_not_found` | Unknown child name in `reload_child` |
| 404 | `pando_not_found` | Unknown pando name in `load_pando` |
| 409 | `operation_in_progress` | Concurrent operation on same target |
| 500 | `internal_error` | Unexpected failure |
| 200 | (success) | Operation completed, result in body |

The `error` field is a stable machine-readable code. `detail` is human-readable
and may vary. `code` mirrors the HTTP status for consumers that parse the body.

### Lifecycle HTTP endpoints and success schemas

Three lifecycle routes, all `POST`, all require auth (Bearer on TCP, file
permissions on UDS):

**POST /api/lifecycle/load-pando**

Request:
```json
{ "name": "slate" }
```
Success (200):
```json
{ "pando": "slate", "status": "loaded", "children_activated": 1 }
```

**POST /api/lifecycle/refresh**

Request: `{}` or empty body

Success (200):
```json
{
  "pandos_loaded": 3,
  "pandos_failed": 0,
  "children_activated": 7,
  "children_failed": 1,
  "degraded": [{ "name": "lakehouse-catalog", "reason": "on_load failed" }]
}
```

**POST /api/lifecycle/reload-child**

Request:
```json
{ "name": "slate-manager" }
```
Success (200):
```json
{ "child": "slate-manager", "status": "reloaded", "previous_instance": "drained" }
```
Failure (200, reload failed but old instance still serving):
```json
{ "child": "slate-manager", "status": "reload_failed", "previous_instance": "active", "reason": "on_load failed: missing schema" }
```

Note: reload failure returns 200, not 500 — the operation completed
deterministically (old instance kept active). The `status` field distinguishes
success from failure. 500 is reserved for unexpected errors.

### CLI surfaces readiness in same phase as API

`patina mother status` renders all readiness fields returned by the health
endpoint in the same phase they are added to the API. No lag between API
availability and CLI visibility. If the API has `control_plane_ready`, the
CLI shows it in the same commit.

### Readiness state transition rules

| Field | Allowed transitions |
|-------|-------------------|
| `control_plane_ready` | `false` → `true` once. Never reverts. |
| `children_total` | Set after discovery. Changes only on `refresh_pandos` (re-discovery). |
| `children_ready_count` | Monotonically increases during warmup. Can decrease on refresh (fewer children discovered) but not on reload (old instance stays active). |
| `children_degraded` | Entries added on initial activation failure. Removed on successful reload. Not added on reload failure (old instance stays healthy). |

### Manifest integrity via SHA-256

Binding declarations in `pando.toml` define resource access scopes. Tampering
with a manifest could grant a child wider access than intended. Mother verifies
integrity at load time:

1. At install/seed: compute SHA-256 of `pando.toml`, each `child.toml`, and
   each `.wasm` file. Write hashes alongside originals.
2. At load: recompute all hashes, compare to stored hashes
3. Any mismatch → refuse to load entire pando, emit diagnostic:
   `"pando '{name}' integrity check failed on {file} — reinstall or run patina pando verify"`
4. Missing hash file → first load after upgrade, compute and write hash
   (bootstrap case — no hash yet means no prior integrity baseline)

The integrity chain is: `pando.toml` (what bindings are declared) → `child.toml`
(what capabilities the child claims) → `.wasm` (what code actually runs). All
three must pass before Mother grants any resources.

First-party pandos (embedded in binary) recompute from the binary's copy during
seeding — the binary is the authority. Third-party pandos compute at install time.

### Edge cases resolve during build

Exact behavior for 50-project ATTACH latency, partial warmup interruption, and
shutdown-during-reload will be discovered and documented during implementation.
The spec defines the structural model; the DESIGN.md captures resolved reality.

## Current Startup Sequence (what changes)

Before (blocking):
```
state_db_open → federation → child_discovery → registry_load_all → router_build → transport
```

After (two-phase):
```
Phase 1: state_db_open → federation → router_build → transport (health returns 200)
Phase 2: child_discovery → registry_load_all (background, reports progress)
```

## Readiness Surface

Health endpoint (`GET /health`) response adds these fields (additive, no breaking
changes to existing fields):

```json
{
  "status": "ok",
  "version": "0.47.1",
  "uptime_secs": 12,
  "control_plane_ready": true,
  "children_ready_count": 4,
  "children_total": 7,
  "children_degraded": [
    { "name": "lakehouse-catalog", "reason": "on_load failed: missing schema" }
  ],
  "child_count": 7,
  "children": [...],
  "federation_available": true,
  ...
}
```

Field contract (see Readiness State Transition Rules in Resolved Decisions for
full transition semantics):
- `control_plane_ready` — `true` after Phase 1 completes. Never reverts to `false`.
- `children_ready_count` — increments as Phase 2 activates children. Starts at 0.
- `children_total` — set after child discovery. 0 during Phase 1 if discovery hasn't run.
- `children_degraded` — array of `{name, reason}`. Empty when all healthy. Entries
  added on initial activation failure, removed on successful reload.

## Bindings Contract

```rust
// mother/src/runtime.rs
pub trait MotherRuntime: Send + Sync {
    fn load_pando(&self, name: &str) -> Result<PandoLoadResult>;
    fn refresh_pandos(&self) -> Result<PandoRefreshResult>;
    fn reload_child(&self, name: &str) -> Result<ChildReloadResult>;
    fn query_readiness(&self) -> ReadinessState;
}
```

This trait is the internal execution boundary. `ServerState` implements it.
HTTP handlers and CLI both go through this trait. The trait lives in
`mother/src/runtime.rs` — the `mother` crate owns the contract, `src/commands/`
provides the implementation.

Result types are directional — exact fields will emerge from implementation.
The trait method signatures are the locked contract.

## Telemetry Contract

All metrics follow the existing `measure.metric` event pattern.

### Startup metrics (existing, extended)

| Metric | Kind | source_id | Labels | When |
|--------|------|-----------|--------|------|
| `stage_latency_ms` | gauge | `mother:startup:stage_latency_ms` | `scope=startup, action={stage}` | After each startup stage |
| `stage_failure` | counter | `mother:startup:stage_failure` | `scope=startup, action={stage}` | On stage failure |
| `child_activation_ms` | gauge | `mother:startup:child_activation_ms` | `scope=startup, action=child_activate, child={name}` | After each child `on_load` |
| `child_activation_failure` | counter | `mother:startup:child_activation_failure` | `scope=startup, action=child_activate, child={name}` | On child `on_load` failure |

### Lifecycle metrics (new)

| Metric | Kind | source_id | Labels | When |
|--------|------|-----------|--------|------|
| `load_pando_latency_ms` | gauge | `mother:lifecycle:load_pando_latency_ms` | `scope=lifecycle, action=load_pando, pando={name}` | After pando load |
| `load_pando_failure` | counter | `mother:lifecycle:load_pando_failure` | `scope=lifecycle, action=load_pando, pando={name}` | On pando load failure |
| `refresh_latency_ms` | gauge | `mother:lifecycle:refresh_latency_ms` | `scope=lifecycle, action=refresh` | After full refresh |
| `reload_child_latency_ms` | gauge | `mother:lifecycle:reload_child_latency_ms` | `scope=lifecycle, action=reload_child, child={name}` | After child reload |
| `reload_child_failure` | counter | `mother:lifecycle:reload_child_failure` | `scope=lifecycle, action=reload_child, child={name}` | On reload failure |

## Federation Handoff Contract

When this spec is complete, `mother-duckdb-ducklake-federation` can assume:

1. **Health returns 200 before federation query routes are called.** Federation
   startup (DB open, DuckLake load, ATTACH) runs in Phase 1. By the time
   federation HTTP routes are registered, the attach registry is populated.
2. **`MotherRuntime` trait exists.** Federation query execution can be wired
   as a method on the runtime trait alongside lifecycle operations.
3. **Readiness surface is stable.** Federation fields already in health
   (`federation_available`, etc.) coexist with the new readiness fields.
4. **Lifecycle operations are available.** `refresh_pandos()` can trigger
   federation re-attach as a side effect.

## Phases

### Phase 1 — Startup split and readiness

- Reorder `daemon.rs:554` to move `child_discovery` and `registry_load_all`
  after transport bootstrap
- Run Phase 2 in a background thread with progress reporting
- Add readiness fields to `HealthDetails` and `HealthResponse`
- Add per-child activation metrics
- Test: health returns 200 before any child `on_load`

### Phase 2 — Bindings contract and lifecycle

- Add `MotherRuntime` trait to `mother/src/runtime.rs`
- Implement `load_pando`, `refresh_pandos`, `reload_child`, `query_readiness`
- Add per-child mutex for serialized lifecycle operations
- Reload: load new → on_load → swap if success, keep old if failure (see Resolved Decisions)
- Add lifecycle metrics
- Add HTTP endpoints: `/api/lifecycle/load-pando`, `/api/lifecycle/refresh`,
  `/api/lifecycle/reload-child` (schemas in Resolved Decisions)
- Add thin CLI commands: `patina mother lifecycle load-pando`, `refresh`, `reload-child`

### Phase 3 — Proof and federation unblock

- Full test coverage for startup ordering assertion
- Lifecycle operation tests (idempotency, reload rollback, degradation)
- Update `mother-duckdb-ducklake-federation` `blocked_by` to clear
- DESIGN.md updated with resolved reality

## Verification

```bash
cargo check --workspace -q
cargo test -q --lib

# Startup ordering assertion
patina mother start
# health returns 200 immediately, children_ready_count increases over time
patina mother status
```

## Build Readiness

Ready for implementation. The startup reordering (Phase 1) is a mechanical
change to `daemon.rs` with a clear structural assertion. The bindings trait
(Phase 2) has a defined shape but result types resolve during build.
