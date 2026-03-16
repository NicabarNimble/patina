# Design: boundary shim removal and proof

## Why This Design

Compatibility shims were the right migration safety tool, but they are now
technical debt. A slice-per-shim approach preserves rollback safety while
proving the new boundary ownership in production code.

## Build Target

Eliminate `src/mother/*` compatibility shim modules that proxy to
`src/beliefs/*` and `src/child/*`, without regressing runtime behavior.

## Slice Order

1. `src/mother/graph_host.rs` + `mod graph_host` (completed).
2. `src/mother/belief_host.rs` + `mod belief_host`.
3. `src/mother/graph.rs` + `mod graph` + export path updates.
4. `src/mother/child.rs` + `mod child` + export path updates.

## Verification Plan

- No direct callsites for removed shim path (`rg`).
- `cargo check --workspace`.
- Boundary guards:
  - `bash resources/scripts/check-runtime-boundaries.sh`
  - `bash resources/scripts/check-layer-output-contract.sh`
- Focused tests around plugin host routing and mother command surfaces.

## Rollback Strategy

If a slice breaks behavior, reintroduce only that shim in a follow-up commit;
do not revert unrelated boundary work.
