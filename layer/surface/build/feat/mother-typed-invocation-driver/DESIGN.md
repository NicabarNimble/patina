# design: mother typed invocation driver

## Core values anchors

- **Patina identity**: this is protocol ingress infrastructure, not product feature drift.
- **Dependable Rust**: small public seam (`Child::call`), private runtime details (`InvocationDriver`).
- **Adapter pattern**: two real implementations (fail-closed + handle bridge) prove seam legitimacy.
- **Safety boundaries**: deny-by-default validation, typed error codes, and policy gates stay fail-closed.
- **Unix philosophy**: policy/observability in Mother, domain behavior in children.

## Runtime seam

In `src/child/internal/child.rs`:

- `InvocationDriver` trait
- `FailClosedInvocationDriver`
- `TypedComponentInvocationDriver` (default)
- `HandleBridgeInvocationDriver` (compat lane)

`WasmChild::call` delegates to the driver.

## Generic resolution + codec

- Resolve operation id via `<package>:<interface>.<function>` parser.
- Validate function token characters.
- Resolve exported interface/function candidates generically (version + separator tolerance).
- Lower JSON args to canonical component values using reflected parameter types.
- Lift canonical component results back to JSON.

Compatibility lane (`handle-bridge`) keeps legacy payload rules:
- args must be JSON array
- 0 args => `{}` payload
- 1 arg => payload is first value
- N>1 => payload is full args array

This keeps operation addressing contract-agnostic while moving default invocation onto component ABI.

## Observability model (Rivet-inspired)

In `mother/src/registry.rs`:

- Existing call metrics kept (`latency`, `throughput`, `success/error`).
- Added:
  - `mother_wit_call_denied` with `deny_reason`
  - `mother_wit_call_policy_ms`
  - `mother_wit_call_invoke_ms`
- Added in-memory bounded recent history (`TypedCallObservation`) for inspector surfaces.

In `mother/src/http_api.rs` + `mother/src/http_routes.rs`:

- New endpoint: `POST /api/inspector/typed-calls` (with optional `{limit}`)

## mwd5 contract detail

`folder-watch-actor` business operations are invoked through typed `patina:watch/control` exports.
The child runs in `wit-only` ingress mode for business calls; Mother remains contract-agnostic and watcher-specific handle bridging is not required.

## Non-goals in this slice

- whamm integration (kept exploratory).
- full support for resource/future/stream/error-context value lowering/lifting in JSON bridge.
