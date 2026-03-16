---
type: feat
id: spec-shadow-knowledge-child
status: abandoned
created: 2026-03-13
blocked_by:
- doctrine-boundary-reorg-no-core-tools
related:
- src/commands/spec/mod.rs
- sdk/patina-sdk/src/knowledge_child.rs
- src/plugin/internal/knowledge_child.rs
exit_criteria:
- id: shadow-spec-child-builds-and-loads
  text: New spec knowledge-child artifact builds and loads as a parallel system
  checked: false
- id: legacy-spec-and-shadow-spec-run-in-parallel
  text: Existing spec command path remains operational while shadow child path is testable behind explicit routing
  checked: false
- id: parity-tests-cover-core-spec-flows
  text: create/list/show/check/next parity tests exist between legacy and shadow spec paths
  checked: false
- id: cutover-criteria-defined-separately
  text: Legacy spec removal and full cutover are deferred to a dedicated follow-up spec
  checked: false
---
# feat: shadow spec system as knowledge-child (parallel to legacy spec)

> Build a new spec system path as a wasm knowledge-child under patina-sdk while keeping legacy spec in place.

## Problem

Spec-as-child experimentation is valuable, but replacing current spec flow immediately creates avoidable product risk.

## Goal

Ship a parallel shadow implementation first, prove parity and performance characteristics, then decide cutover via follow-up spec.

## Status

Abandoned.

Rationale: not critical for current execution lanes and intentionally deferred
until the full reorg is complete. Concepts can be re-explored post-reorg with
fresh scope and slice-first implementation planning.

## Non-Goals

- Do not remove legacy spec flow in this spec.
- Do not couple this spec to broad folder reorg.

## Verification

- `cargo check --workspace`
- `patina spec check spec-shadow-knowledge-child --json`
