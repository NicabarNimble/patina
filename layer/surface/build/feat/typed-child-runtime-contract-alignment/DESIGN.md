# Design: Typed child runtime contract alignment for Mother

## Why This Design

Slate already proves control-plane lifecycle correctness. Remaining failure is contract alignment between typed operation declarations, component exports, and Mother routing expectations. This design isolates that boundary and fixes it without fail-open behavior.

## Build Target

- Canonical versioned typed operation IDs for Slate.
- Loader/runtime acceptance of the intended typed contract shape.
- Reproducible routed execute success with external Slate release artifacts.

## Resolved Decisions

- Contract IDs are versioned (`patina:slate/control@0.1.0.*`).
- Validation remains strict and fail-closed.
- Keep this slice focused on contract alignment needed for routed execute proof.

## Commits
1. `f08f8478` — versioned Slate contract operation IDs in in-repo `children/slate-manager/child.toml`.
2. `2ff1195` (external `patina-child-slate`) — versioned contract IDs for published Slate child manifest.

## Direct Code Targets
- `children/slate-manager/child.toml` — contract operation ID canonicalization.
- `src/commands/mother/daemon/dispatch.rs` — Slate typed operation mapping IDs.
- `src/child/internal/child.rs` — typed operation contract validation/load semantics.
- `src/commands/mother/daemon/tests/mod.rs` — mapping/contract regression assertions.

## Verification Plan

1. Unit coverage for operation-ID mapping + export validation.
2. Loader/runtime activation check shows Slate load success under strict contract checks.
3. Routed execute command proof:
   - `PATINA_SPEC_BACKEND=execute cargo run -q -- spec next --json`
4. End-to-end control-plane + routing confirmation for external Slate entry.

## Build Readiness

- External Slate release path is active and reproducible.
- Failure mode is narrow and observable in daemon logs.
- Success criteria are concrete and testable.

## Open Questions

1. Do we keep Slate as `hybrid` ingress short-term or flip to `wit-only` once runtime path is fully typed-first?
2. Should typed operation IDs be normalized in one shared helper to eliminate drift between manifest and dispatch surfaces?
