---
type: feat
id: child-registry-control-plane-remaining
status: active
created: 2026-04-29
updated: 2026-04-29
sessions:
  origin: 20260424-063230-539313000
related:
- layer/surface/build/feat/child-registry-control-plane/SPEC.md
- layer/surface/build/feat/child-registry-control-plane/DESIGN.md
- mother/src/state/children_registry.rs
- mother/src/child_registry/sync.rs
- mother/src/child_registry/github.rs
- src/commands/mother/children.rs
- src/commands/mother/mod.rs
beliefs:
- '[[spec-driven-design]]'
- '[[safety-boundaries]]'
- '[[dependable-rust]]'
- '[[adapter-pattern]]'
references:
- layer/core/values/spec-driven-design.md
- layer/core/values/safety-boundaries.md
- layer/core/values/dependable-rust.md
- layer/core/values/adapter-pattern.md
exit_criteria:
- id: crc-r1-approval-lockdown
  text: Mother enforces explicit approval lifecycle transitions (`candidate|approved|blocked|deprecated`) and denies non-approved install/assignment by default with auditable override semantics.
  checked: false
- id: crc-r2-pin-verify-install
  text: Install flow is pin-first (`name@version`/entry id), verifies staged artifact+manifest hashes before atomic placement, records install provenance, and fails closed on mismatch.
  checked: false
- id: crc-r3-assignment-audit
  text: Project child assignment/revoke commands persist authoritative assignment rows and emit deterministic audit events for grant/deny/revoke transitions.
  checked: false
- id: crc-r4-operator-surface-parity
  text: Operator surface under `patina mother children` includes remaining lifecycle operations required by control-plane spec (`show/search/approve/block/deprecate/install/assign/unassign/status`) with JSON outputs and explicit failure reasons.
  checked: false
- id: crc-r5-external-slate-proof
  text: External Slate proof is executed end-to-end (source sync -> approval -> install -> assign -> routed usage verification) with reproducible verification steps captured in spec artifacts.
  checked: false
validated_against_commit: e0c87f8aaecaa5c79693676ca623e910c1bdc630
last_freshness_check: 2026-04-29T02:12:02Z
freshness_scope:
- mother/src/state/children_registry.rs
- mother/src/child_registry/sync.rs
- mother/src/child_registry/github.rs
- src/commands/mother/children.rs
- src/commands/mother/mod.rs
---
# feat: Child registry control plane completion (remaining criteria)

> Carry forward and finish the criteria intentionally skipped during forced completion of `child-registry-control-plane`.

## Problem

`child-registry-control-plane` was force-completed with 5 unchecked criteria. Without an explicit follow-on contract, those obligations can drift out of active planning.

## Goal

Finish the remaining control-plane criteria with fail-closed behavior and auditable operator flows, while preserving the seams already established in Mother state and provider adapters.

## Status

- Prior spec delivered: seam refactor, schema/state APIs, provider abstraction, GitHub sync ingestion, source management commands.
- This spec scope starts at remaining slices equivalent to prior C/D/E/F obligations.
- Implementation progress landed in `fda637be` (state transitions + audit store) and `a8c2f090` (operator lifecycle commands + install/assignment flows).
- Exit criteria remain unchecked until verification gate + external Slate proof evidence are completed and linked.

## Non-Goals

- Rework already-landed seam architecture unless defects are discovered.
- Introduce marketplace/billing semantics.
- Replace existing local child loading path.

## Target Shape

1. Approval workflow is first-class and enforced by policy.
2. Install pipeline is pin-and-verify with atomic writes and provenance records.
3. Assignment workflow is authoritative + auditable.
4. Operator commands cover full lifecycle contract for Mother child control plane.
5. External Slate workflow is proven and documented.

## Solution

Implement remaining slices in order:

- **Approval**: command/API transitions with explicit state machine.
- **Install**: verified download/stage/atomic swap with status recording.
- **Assignment**: project binding commands + audit event emission.
- **Operator surface**: complete command set with JSON/text parity.
- **External proof**: scripted/manual reproducible Slate onboarding run.

## Implementation Order

1. Approval lifecycle command/state transition hardening.
2. Pin+verify install flow and local artifact materialization.
3. Assignment/revoke flows with audit event emission.
4. Remaining operator commands + status surfaces.
5. External Slate end-to-end proof and fixture capture.

## Resolved Decisions

- Keep `ChildRegistryStore` seam as the state authority boundary.
- Keep provider adapters data-only; no policy in adapters.
- Keep fail-closed behavior as default for unresolved/unimplemented paths.

## Verification

```bash
patina spec check child-registry-control-plane-remaining --json
cargo fmt --all
cargo check -q
cargo test -p mother state::tests --quiet
cargo test -p mother child_registry::sync::tests --quiet
cargo test -p patina-ai commands::mother::children::tests --quiet
```

Plus scenario verification for each exit criterion (approval, install verify failure, assignment audit, full CLI surface, Slate proof).

## Exit Criteria

Frontmatter `crc-r1..crc-r5` is the source of truth.

## Build Readiness

- Existing seam and sync baseline is green.
- Remaining work is now explicitly queued and trackable via this spec.
