---
type: refactor
id: patina-code-to-vision-seam-closure
status: draft
created: 2026-03-22
related:
  - layer/surface/build/refactor/patina-code-to-vision/SPEC.md
  - layer/surface/build/refactor/greenfield-mother-patina-rebuild/SPEC.md
exit_criteria:
  - id: CV1
    text: Mother is a standalone daemon in the mother/ crate — Mother-owned runtime infrastructure (state, broker infrastructure, registry, events, tasks, lifecycle, socket, protocol) is centralized there, not split across three locations
    checked: false
  - id: CV2
    text: CLI binary has zero Mother infrastructure runtime code — it talks to Mother over Unix socket or runs core verbs standalone; thin adapter bridges to core product domains are explicit and allowed
    checked: false
  - id: CV11
    text: Scrape strategy boundary is explicit and enforceable — layer/beliefs remain core, and non-core scrape strategy lanes are extraction-ready and independently pluggable without breaking current core scrape behavior. Child extraction happens only after 1:1 parity proof
    checked: false
---
# refactor: Close remaining code-to-vision seams

> Close CV1/CV2/CV11 with explicit parity gates, or prove and lock permanent seam ownership.

## Problem

`patina-code-to-vision` intentionally retained adapter-backed seams to avoid risky,
low-value refactors during migration. Those criteria still need dedicated closure work.

## Goal

Resolve CV1, CV2, and CV11 in a focused lane with explicit parity and rollback discipline.

## Non-Goals

- Do not regress already-green criteria from `patina-code-to-vision`.
- Do not force architectural purity without migration-safe evidence.

## Scope

- Mother ownership seams and CLI runtime boundaries.
- Scrape strategy extraction contract and parity gates.

## Verification

- `cargo check -q`
- `cargo test -q`
- targeted command proofs for Mother on/off runtime policy
- explicit seam map update proving closure or permanent acceptance with rationale

## Build Readiness

Ready when promoted to active after review against greenfield blueprint.
