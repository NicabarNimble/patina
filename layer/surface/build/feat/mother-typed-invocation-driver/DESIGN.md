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
- `HandleBridgeInvocationDriver`

`WasmChild::call` now delegates to the driver.

## Generic resolution + codec

- Resolve operation id via `<package>:<interface>.<function>` parser.
- Validate function token characters.
- Derive legacy action name from function (`_` -> `-`).
- Encode args with strict shape rules:
  - must be JSON array
  - 0 args => `{}` payload
  - 1 arg => payload is first value
  - N>1 => payload is full args array

This supports migration while keeping operation addressing contract-agnostic.

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

## mwd5 compatibility detail

`children/folder-watch-actor/src/lib.rs` configure handle path now accepts either:
- object patch payload (existing)
- typed args array payload `[config, reset_snapshot?]`

This keeps Mother generic and places domain-specific argument interpretation in the child.

## Non-goals in this slice

- Full dynamic canonical ABI invocation of arbitrary WIT signatures from host-side reflection.
- whamm integration (kept exploratory).
