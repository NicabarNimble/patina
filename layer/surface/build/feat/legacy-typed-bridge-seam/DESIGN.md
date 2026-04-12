# Design: Legacy Typed Bridge Seam

## Architecture

### 1) Mother policy seam (authority plane)

Create a Mother-managed internal module responsible for:

- Legacy toy alias allowlist mapping.
- Unknown toy denial (fail closed).
- Deterministic translation result model used by API/runtime surfaces.

The module is policy-only: no side effects, no mutable global state.

### 2) Typed bridge child (execution plane)

Add a typed WIT child that accepts normalized bridge requests and returns typed translation outputs.

- WIT package: `patina:legacy-typed-bridge@0.1.0`
- world: `legacy-typed-bridge`
- import: `wasi:logging/logging@0.1.0`
- export: bridge interface with request/response records and `translate` function.

## Mapping contract

Initial canonical legacy aliases:

- `log` -> `logging`
- `state` -> `keyvalue`
- `store` -> `keyvalue`
- `fs` -> `filesystem`

Any other alias is denied.

## Data model

- `BridgeRequest`: legacy action + legacy toy aliases + opaque payload.
- `BridgeToyDecision`: per-toy decision (`mapped` or `denied`) with optional typed target.
- `BridgeResponse`: translated toy list + denied list + policy verdict.

## Safety and fail-closed rules

1. Unknown alias => denied.
2. Partial mapping => overall denied unless caller requests `allow_partial=true` (not enabled in first slice).
3. No automatic dynamic toy discovery.
4. Bridge outputs are read-only policy artifacts in this slice.

## Atlas visibility hook

Atlas can consume the Mother bridge policy model to show:

- children requiring bridge lane
- unknown/denied toy aliases
- typed-only compliance status

Implementation of Atlas policy rendering is a follow-up within the same spec.

## Initial code targets

- `mother/src/bridge.rs`
- `mother/src/lib.rs`
- `children/legacy-typed-bridge/child.toml`
- `children/legacy-typed-bridge/Cargo.toml`
- `children/legacy-typed-bridge/wit/world.wit`
- `children/legacy-typed-bridge/src/lib.rs`
- `Cargo.toml` (workspace member)
