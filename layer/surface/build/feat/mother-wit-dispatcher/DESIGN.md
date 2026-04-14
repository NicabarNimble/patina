# Design: Mother WIT dispatcher

## Intent

Close the gap between declared business contracts and runtime invocation by adding a typed WIT dispatcher in Mother.

Today:
- lifecycle ingress: typed (`patina:child`)
- business ingress: stringly (`handle(action,payload)`)

Target:
- lifecycle ingress: keep `patina:child`
- business ingress: WIT operation dispatch

## Value alignment

- `patina-identity`: typed dispatcher is protocol-contract infrastructure, not product feature creep.
- `unix-philosophy`: separate lifecycle control path from business call path.
- `dependable-rust`: keep public runtime seam small; hide WIT ABI plumbing internally.
- `adapter-pattern`: only add adapter seam where both lanes are real (`handle`, typed WIT).
- `safety-boundaries`: allowlist operations and fail closed.
- `spec-driven-design`: every slice ships with deterministic success + failure proof.

## Read-first code anchors

- `mother/src/runtime.rs`: `Child` trait exposes `handle` and `call`; default `call` is fail-closed.
- `src/child/internal/child.rs`: `WasmChild::handle` bridges action+JSON payload into `instance.call_handle`.
- `src/child/internal/child.rs`: `WasmChild::call` is fail-closed generic (no watcher-specific contract branch).
- `mother/src/registry.rs`: enforces ingress policy on both handle and call and emits both metric families.
- `mother/src/http_api.rs`: child endpoint shape remains `/child/{name}/{action}`, with `action == call` for typed lane.
- `src/main.rs`: includes `ChildCommands::Run` and `ChildCommands::Call`.

These anchors define the compatibility surface we must preserve while adding typed ingress.

## Scope

- Mother runtime dispatch extension
- daemon/CLI invocation surface for typed calls
- manifest-level ingress policy (`handle`, `hybrid`, `wit-only`)
- migration compatibility for existing handle children

## Scalpel diff policy

- Prefer additive seams over in-place rewrites.
- One slice = one narrow boundary.
- Keep old path operational until typed path has deterministic parity tests.
- Do not mix manifest parsing, runtime dispatch, CLI routing, and watcher policy in one change.

## Dispatch shape

### Request

- child name
- operation id (`<package>:<interface>.<function>`)
- args JSON (array positional)

### Response

- typed success value (JSON-encoded for transport)
- typed error value/string
- invocation metadata (interface/function labels)

## Enforcement

1. Read ingress mode from manifest.
2. Read operation allowlist (if present).
3. For `wit-only`:
   - deny business handle calls,
   - allow lifecycle (`health`, `tick`, `drain`, `on-load`, `on-unload`).
4. Emit explicit deny reason and remediation command.

## Observability

Primary model: follow **Rivet / agent-os style** operational visibility.

Add WIT-call metrics in Mother eventlog:
- `mother_wit_call_latency_ms` (gauge)
- `mother_wit_call_throughput` (counter)
- `mother_wit_call_success` / `mother_wit_call_error` (counter)

Required labels:
- child
- interface
- function
- outcome

Incremental expansion (Rivet-inspired):
- startup/lifecycle timing buckets for typed dispatcher path
- per-operation deny-reason counters
- inspector-friendly query surface for recent typed calls and policy outcomes

`whamm` remains exploratory for deep low-level Wasm instrumentation and is not a prerequisite for this spec completion.

Keep handle metrics during migration.

## Proposed seam (minimal)

At runtime abstraction level, add one typed call seam beside handle:

- `handle(request)` (existing)
- `call(operation_id, args_json)` (new)

`call(...)` is the only new public behavior required for this spec; all WIT ABI resolution remains internal/private.

## Locked defaults

- `wit-only` denies all business `handle` calls.
- Operation allowlist required for `wit-only`.
- Child-level operation addressing in first cut.

## Rollout

1. Implement hybrid first.
2. Keep runtime typed call fail-closed generic until generic dispatcher exists.
3. Use `watch-null-sink` to validate typed event wiring without persistence.
4. Enable `wit-only` for selected children once generic dispatch exists.
5. Keep handle lane for service children until contracts stabilize.

## File touchpoints

- `mother/src/runtime.rs`
- `mother/src/registry.rs`
- `src/child/internal/child.rs`
- `src/commands/mother/daemon.rs`
- `src/main.rs`
- `children/folder-watch-actor/child.toml` (ingress policy after runtime support)

## Notes

WASI remains capability source of truth.
Custom WIT is only for business-domain contracts not covered by WASI.

Explicit anti-drift rule: Mother must not add per-domain typed binding branches (e.g. watcher-specific runtime fields). Domain behavior stays in children.
