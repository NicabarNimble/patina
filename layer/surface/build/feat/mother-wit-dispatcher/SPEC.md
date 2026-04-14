---
type: feat
id: mother-wit-dispatcher
status: active
created: 2026-04-13
sessions:
  origin: 20260413-160000-000000000
beliefs:
  - "[[wasi-is-foundation-not-option]]"
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[world-boundary-is-type-safety]]"
related:
  - layer/core/values/patina-identity.md
  - layer/core/values/spec-driven-design.md
  - layer/core/values/unix-philosophy.md
  - layer/core/values/safety-boundaries.md
  - layer/core/values/dependable-rust.md
  - layer/core/values/adapter-pattern.md
  - layer/core/values/session-capture.md
  - layer/core/values/oxidized-knowledge.md
  - wit/child/child.wit
  - src/child/internal/child.rs
  - src/child/internal/mod.rs
  - mother/src/runtime.rs
  - mother/src/registry.rs
  - mother/src/http_api.rs
  - src/main.rs
  - children/folder-watch-actor/wit-contract/watch.wit
  - layer/surface/build/feat/folder-watch-actor-child/SPEC.md
  - children/watch-null-sink/
  - layer/surface/build/feat/watch-null-sink-child/SPEC.md
exit_criteria:
  - id: mwd1-manifest-ingress-mode
    text: "child.toml supports explicit ingress mode (`handle`, `hybrid`, `wit-only`) for child kind `child`; default remains backward-compatible."
    checked: true
  - id: mwd2-runtime-wit-dispatch-surface
    text: "Mother runtime exposes a typed invocation path addressed by fully-qualified operation id (`<package>:<interface>.<function>`) through a generic invocation driver seam (fail-closed + bridge implementation)."
    checked: true
  - id: mwd3-cli-wit-call-command
    text: "CLI adds a WIT invocation command (`patina child call ...`) and routes through Mother typed dispatch instead of `handle(action,payload)`."
    checked: true
  - id: mwd4-wit-only-enforced
    text: "When ingress mode is `wit-only`, `handle` business operations are rejected at runtime with clear remediation; lifecycle functions (`on-load`, `tick`, `health`, `drain`) remain available."
    checked: true
  - id: mwd5-folder-watch-proof
    text: "folder-watch-actor business ops (`configure`, `status`, `scan-now`, `reset`) are invokable end-to-end through `patina child call` operation-id routing without watcher-specific Mother runtime branches."
    checked: true
  - id: mwd6-observability-parity
    text: "Mother emits invocation metrics for WIT dispatch with labels (`child`, `interface`, `function`, `outcome`) and keeps existing handle metrics during migration."
    checked: true
  - id: mwd7-wasi-first-contract-rule
    text: "Architecture docs and checks enforce: WASI for capabilities; custom WIT only for business-domain contracts not provided by WASI."
    checked: true
  - id: mwd8-compat-lane-preserved
    text: "Existing handle-based children continue to run unchanged under `handle`/`hybrid` ingress modes."
    checked: true
  - id: mwd9-tests-and-audit
    text: "Automated tests cover handle-only, hybrid, and wit-only lanes; audit output can list per-child ingress mode and exported business operations."
    checked: true
  - id: mwd10-core-values-anchored
    text: "Design and implementation slices explicitly map to layer/core values (identity, unix decomposition, dependable rust seams, adapter proof, safety boundaries, spec-driven test gates, session capture, oxidized knowledge storage)."
    checked: true
---
# feat: Mother WIT dispatcher (typed child ingress)

## Problem

Children can define business contracts in WIT, but Mother/CLI currently enter children through `handle(action: string, payload: string)` in `patina:child` runtime world.

That creates a split:
- business promise lives in specs/WIT,
- runtime ingress is still string+JSON.

Result: WIT is not the only way in, so boundary typing is weaker than intended.

## Goal

Make WIT the primary (and optionally only) ingress for child business operations, while preserving lifecycle orchestration and backward compatibility.

Concretely:
1. Mother can invoke child business exports by WIT operation identity.
2. CLI can invoke typed operations directly.
3. `wit-only` children can reject business `handle` calls.
4. Existing children continue to work during migration.

## Non-Goals

- Replacing `patina:child` lifecycle (`on-load`, `tick`, `health`, `drain`) in this spec.
- Defining a universal watcher/file schema in Mother.
- Removing handle lane globally in one cut.
- Introducing new non-WASI host capabilities.

## Core Value Anchors (normative)

- **Patina Identity** (`layer/core/values/patina-identity.md`): this is protocol infrastructure hardening for child invocation contracts, not a feature detour.
- **Spec-Driven Design** (`layer/core/values/spec-driven-design.md`): all non-trivial slices require explicit tests and proof commands before criteria are checked.
- **Unix Philosophy** (`layer/core/values/unix-philosophy.md`): keep lifecycle orchestration and business invocation as separate seams; avoid one giant dispatcher doing both concerns implicitly.
- **Dependable Rust** (`layer/core/values/dependable-rust.md`): introduce minimal public dispatch surface, keep ABI/details in private internals.
- **Adapter Pattern** (`layer/core/values/adapter-pattern.md`): add abstraction only where 2+ real implementations exist (`handle` lane and WIT lane).
- **Safety Boundaries** (`layer/core/values/safety-boundaries.md`): deny-by-default operation allowlists, no capability broadening, clear runtime rejection paths.
- **Session Capture** (`layer/core/values/session-capture.md`): each slice ships with reproducible command transcripts for operators.
- **Oxidized Knowledge** (`layer/core/values/oxidized-knowledge.md`): decision and policy live in repo (`layer/`), not local-only notes.

## Current Code Truth (read-first baseline)

The following behavior is established in code and constrains this spec:

1. `src/child/internal/child.rs` (`WasmChild::handle`) serializes JSON payload and calls `instance.call_handle(...)`.
2. `mother/src/runtime.rs` `Child` trait now includes both `handle(&ChildRequest)` and `call(&ChildCallRequest)`; default `call` is fail-closed.
3. `src/child/internal/child.rs` `WasmChild::call` is currently fail-closed generic (no watcher-specific typed binding branch in Mother runtime).
4. `mother/src/registry.rs` enforces ingress policy for both `handle` and `call`, and emits both handle and WIT-call metrics.
5. `mother/src/http_api.rs` serves typed calls through `/child/{name}/call` (special action in existing child route).
6. `src/main.rs` provides both `ChildCommands::Run` and `ChildCommands::Call`.
7. `src/child/internal/mod.rs` `ChildManifest` now parses ingress mode and operation allowlist.

All implementation slices must preserve compatibility with this baseline unless a slice explicitly migrates one seam.

## Findings Update (2026-04-13)

- Removed watcher-specific typed binding from Mother runtime (`watch_call_bindings` and explicit `patina:watch/control.*` branch).
- `WasmChild::call` is intentionally fail-closed generic again.
- This keeps Mother from becoming domain-child logic and preserves strict separation: children own domain contracts, Mother owns orchestration/policy.
- Added `watch-null-sink` child as an ephemeral typed event sink so connection testing can proceed without pushing watch-domain behavior into Mother.

## Observability Direction Update (2026-04-13)

- Primary reference model: **Rivet / agent-os observability patterns** (structured metrics, lifecycle timing, queue/schedule visibility, inspector-style runtime surfaces).
- Secondary exploration: **whamm** as a deep Wasm instrumentation candidate for targeted experiments.
- Scope lock: `mother-wit-dispatcher` delivery should not depend on whamm integration; first complete Rivet-style Mother-native telemetry and inspection surfaces.

## Runtime Update (2026-04-13, late)

- Added a generic `InvocationDriver` seam in WASM child runtime with two implementations:
  - `FailClosedInvocationDriver`
  - `HandleBridgeInvocationDriver`
- Operation ids are now resolved and validated generically (`<package>:<interface>.<function>`), with strict error taxonomy.
- Mother now records typed call outcomes (`success/error/denied`), deny reasons, and policy/invoke timing metrics.
- Added inspector route `POST /api/inspector/typed-calls` for recent typed-call visibility.
- `folder-watch-actor` now supports typed-args array payload shape for `configure`, enabling operation-id call path proof without watcher-specific Mother binding code.

## Architecture Rule (locked)

Two planes, two rules:

1. **Capability plane** (sandbox openings):
   - Use WASI (and approved Patina toys) only.
   - Examples: filesystem, keyvalue, logging, messaging.

2. **Business contract plane** (domain promises):
   - Use custom WIT packages/interfaces.
   - Examples: `patina:watch/control.configure`, `status`, `scan-now`, `reset`.

No new stringly business API is introduced.

## Proposed Runtime Model

### Ingress modes

Per child (manifest-driven):
- `handle` — legacy lane only.
- `hybrid` — handle + WIT dispatch enabled.
- `wit-only` — business calls allowed only through WIT dispatcher.

Lifecycle calls remain via `patina:child` runtime world for all modes.

### Operation identity

Mother resolves business calls via fully qualified operation id:

`<package>:<interface>.<function>`

Example:
- `patina:watch/control.status`
- `patina:watch/control.scan-now`

### Dispatch contract

Mother typed dispatcher receives:
- child name
- operation id
- args payload (canonical JSON form for CLI transport)

Dispatcher is responsible for:
- resolving target export,
- ABI-safe conversion,
- invocation,
- conversion of typed result/error back to structured response.

## CLI Surface (proposed)

New command:

```bash
patina child call <child> <operation-id> '<json-args>'
```

Examples:

```bash
patina child call folder-watch-actor patina:watch/control.status '[]'
patina child call folder-watch-actor patina:watch/control.scan-now '[]'
patina child call folder-watch-actor patina:watch/control.configure '[{"watch-path":"/input","stream-name":"watch.folder","recursive":true,"include-hidden":false,"emit-existing-on-start":true,"extensions":[]}, false]'
```

Notes:
- `run` command remains for compatibility.
- In `wit-only`, `run <action>` for business ops returns explicit error and points to `child call`.

## Child Manifest Additions (proposed)

Add optional section:

```toml
[child.ingress]
mode = "hybrid" # handle | hybrid | wit-only

[child.contract]
default = "patina:watch/control.status"
allow = [
  "patina:watch/control.configure",
  "patina:watch/control.status",
  "patina:watch/control.scan-now",
  "patina:watch/control.reset",
]
```

If omitted, behavior remains backward-compatible (`handle`).

## Mother Changes (high level)

1. Extend runtime abstraction to support WIT dispatch path in addition to `handle`.
2. Track per-child ingress mode and allowed operations from manifest.
3. Add dispatcher metrics (`mother_wit_call_latency_ms`, throughput, success/error).
4. Update daemon protocol to expose typed child call endpoint.
5. Keep existing handle route during migration.

## Migration Plan

### Phase 1 — Hybrid lane
- Add manifest ingress mode parsing + defaults.
- Add WIT dispatch plumbing and CLI command.
- Keep handle unchanged.

### Phase 2 — First production child on WIT path
- Enable hybrid for `folder-watch-actor`.
- Keep runtime fail-closed until generic typed dispatcher lands.
- Use `watch-null-sink` to validate child-to-child typed event wiring without persistence side effects.

### Phase 3 — Strict mode
- Switch selected children to `wit-only`.
- Add guardrails and audit output.
- Record an explicit removal milestone for legacy business-handle ingress on converted children (no indefinite hybrid drift).

## Implementation Slices (scalpel plan)

Each slice is intentionally narrow, reviewable, and test-gated.

### Slice A — Manifest policy only
Files:
- `src/child/internal/mod.rs`
- `src/child/internal/tests.rs`

Scope:
- Parse `[child.ingress]` and `[child.contract]`.
- Keep default behavior as `handle` when absent.

Required proof:
- one parse-success test,
- one fail-closed test for invalid ingress mode,
- one compatibility test confirming manifests without new keys still load.

### Slice B — Runtime abstraction seam
Files:
- `mother/src/runtime.rs`
- `src/child/internal/child.rs`
- unit tests nearest seam

Scope:
- Add a minimal typed invocation method alongside handle.
- Keep existing `handle` code path unchanged.

Required proof:
- deterministic success path for handle lane untouched,
- deterministic failure path for unknown operation id in typed lane.

### Slice C — Mother registry + HTTP API
Files:
- `mother/src/registry.rs`
- `mother/src/http_api.rs`
- `src/commands/mother/daemon.rs`

Scope:
- Add typed child call endpoint and observability.
- Preserve `/child/{name}/{action}` semantics.

Required proof:
- deterministic route test for typed endpoint,
- fail-closed deny test for `wit-only` + handle business request,
- metric label conformance test.

### Slice D — CLI surface
Files:
- `src/main.rs`

Scope:
- Add `patina child call <child> <operation-id> <json-args>`.
- Keep `patina child run` behavior for compatibility.

Required proof:
- deterministic argument parsing test,
- deterministic error message test for malformed args JSON.

### Slice E — Folder-watch proof
Files:
- `children/folder-watch-actor/child.toml`
- optional focused test harness path

Scope:
- Set ingress mode and operation allowlist for watcher.
- Prove operation-id typed calls for watcher ops through generic invocation driver path on real components.

Required proof:
- deterministic typed-call success proof for `status`, `configure`, `scan-now`, and `reset`,
- deny proof for disallowed operation.

## Verification

```bash
# Build + compile proof
cargo check -q
cargo test -q --lib ingress
cargo test -q -p mother typed_call_defaults_fail_closed_for_unknown_operation
cargo test -q -p mother observed_typed_call_emits_success_metrics
cargo test -q -p mother observed_typed_call_emits_error_metrics
cargo test -q -p mother wit_only_denies_business_handle_calls
cargo test -q -p mother handle_mode_denies_typed_call
cargo test -q -p mother child_call_route_dispatches_typed_operation
cargo test -q -p mother child_call_route_rejects_missing_operation_id
cargo test -q -p mother inspector_typed_calls_route_returns_history
cargo test -q -p patina-ai resolve_typed_operation_
cargo test -q -p patina-ai encode_typed_args_for_handle_
cargo test -q -p patina-ai folder_watch_actor_typed_call_contracts_end_to_end

# Build and install watcher child
cargo build --manifest-path children/folder-watch-actor/Cargo.toml --target wasm32-wasip2
cp children/folder-watch-actor/target/wasm32-wasip2/debug/patina_ai_child_folder_watch_actor.wasm ~/.patina/plugins/folder-watch-actor.wasm
cp children/folder-watch-actor/child.toml ~/.patina/plugins/folder-watch-actor.toml

# Build and inspect null sink child (typed event sink, no persistence)
cargo build --manifest-path children/watch-null-sink/Cargo.toml --target wasm32-wasip2
wasm-tools component wit children/watch-null-sink/target/wasm32-wasip2/debug/patina_ai_child_watch_null_sink.wasm
# expect: export patina:watch/events@0.1.0

# Audit view: ingress mode + operations
cargo run -q -- child list

# Typed call succeeds through operation-id invocation driver lane
cargo run -q -- child call folder-watch-actor patina:watch/control.status '[]'

# configure via typed args array
cargo run -q -- child call folder-watch-actor patina:watch/control.configure '[{"watch_path":"/tmp","stream_name":"watch.folder","recursive":true,"include_hidden":false,"emit_existing_on_start":false,"extensions":["txt"]},true]'

# Compatibility lane still works where allowed (hybrid)
cargo run -q -- child run folder-watch-actor -- status

# Spec gate
patina spec check mother-wit-dispatcher --json
```

## Risks

1. Dynamic WIT invocation complexity in host runtime.
2. CLI JSON transport may hide argument shape errors without clear diagnostics.
3. Mixed lane period can confuse operators unless audit output is explicit.

## Resolved Defaults (review baseline)

1. `wit-only` hard-denies business `handle` calls. Lifecycle remains available through existing runtime exports.
2. Operation allowlist is mandatory for `wit-only` and optional for `hybrid` (recommended).
3. First cut is child-level operation addressing only; pando-level aliases are deferred.

## Open Question

- Should we require operation allowlist for `hybrid` in Phase 2, or keep optional until broader migration completes?
