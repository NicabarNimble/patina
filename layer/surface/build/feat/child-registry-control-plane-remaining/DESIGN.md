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
- Deprecated entries remain install/assignment-ineligible; only an explicit lifecycle override (`deprecated -> approved` with force) may reopen eligibility.
- Signature metadata remains compatibility/telemetry in this slice; hash verification is the strict install gate.

## Commits
Baseline (already landed before this follow-on):
1. `bc13705e` — added `sources add github` for child registry seed path.
2. `93961d64` — added source enable/disable operations.
3. `8190a1fe` — added JSON output mode for children sources/sync.
4. `c66b78b9` — provider seam + GitHub adapter + sync engine baseline.

Follow-on implementation progress (`child-registry-control-plane-remaining`):
5. `fda637be` — state layer: guarded entry transitions, lookup-by-id, assignment status mutation, child-registry audit store APIs.
6. `a8c2f090` — operator surface: `show/search/approve/block/deprecate/install/assign/unassign/status`, install verification + atomic placement path, assignment and install audit emission.

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

Verification gate used for this spec:

```bash
cargo fmt --all
cargo check -q
cargo test -p mother state::tests --quiet
cargo test -p mother child_registry::sync::tests --quiet
cargo test -p patina-ai commands::mother::children::tests --quiet
patina spec check child-registry-control-plane-remaining --json
```

## Build Readiness

- Baseline schema + provider sync + source operations are already in place.
- State seam + operator command surface for lifecycle/install/assignment are now implemented.
- Remaining work is evidence closure: complete deterministic verification artifacts, execute external Slate proof, and then check `crc-r1..crc-r5` with objective evidence.

## Open Questions

1. Should install command support a dedicated dry-run verification mode before materialization (post-scope enhancement)?
2. Do we want stricter JSON schema snapshots for `status`/`search` outputs in command tests to reduce future drift?
