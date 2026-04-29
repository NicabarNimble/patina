# Design: Child registry control plane completion (remaining criteria)

## Why This Design

The previous spec intentionally forced completion before all criteria were delivered. This design isolates the remainder as explicit, ordered control-plane obligations so delivery stays measurable and fail-closed.

## Build Target

- Complete policy lifecycle for approval/install/assignment.
- Complete operator lifecycle command set with machine-readable outputs.
- Prove external Slate onboarding path end-to-end.

## Resolved Decisions

- `ChildRegistryStore` remains the state seam.
- Provider adapters remain normalization-only boundaries.
- Default posture remains fail-closed.
- JSON output support is required for all mutating and status commands in this scope.

## Commits
1. `bc13705e` — added `sources add github` for child registry seed path.
2. `93961d64` — added source enable/disable operations.
3. `8190a1fe` — added JSON output mode for children sources/sync.
4. `c66b78b9` — provider seam + GitHub adapter + sync engine baseline.

## Direct Code Targets
- `mother/src/state/children_registry.rs` — approval/install/assignment state operations.
- `mother/src/child_registry/sync.rs` — sync compatibility with state lifecycle transitions.
- `mother/src/child_registry/github.rs` — release discovery/hash normalization refinements.
- `src/commands/mother/children.rs` — remaining command surface (`show/search/approve/block/deprecate/install/assign/unassign/status`).
- `src/commands/mother/mod.rs` — CLI enum wiring and routing.
- `src/commands/mother/audit.rs` (or equivalent) — assignment/approval/install audit emission.

## Verification Plan

1. Deterministic tests for approval transition rules and fail-closed enforcement.
2. Deterministic install tests covering hash mismatch rejection and atomic success path.
3. Deterministic assignment tests verifying approved-only and audit event emission.
4. CLI tests for JSON shape contracts for new commands.
5. External Slate proof runbook with exact commands and expected outputs.

## Build Readiness

- Baseline schema + provider sync + source operations are already in place.
- Remaining work can proceed as additive slices without seam churn.

## Open Questions

1. Should deprecated entries allow assignment only with `--force` or be fully denied?
2. Should signature metadata become strict-required by default or remain opt-in strict mode?
3. Should install command support dry-run hash verification before download materialization?
