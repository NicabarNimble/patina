---
type: feat
id: job-aligned-mct-backoffice
status: draft
created: 2026-04-11
related:
- layer/surface/build/feat/sdk-vision-lock/SPEC.md
- layer/surface/build/feat/ba-truths/SPEC.md
- layer/surface/build/feat/child-init-typed-default/SPEC.md
- layer/surface/build/feat/mother-grant-audit-coverage/SPEC.md
- sdk/patina-sdk
- src/mother
- src/commands
- children/
- wit/
beliefs:
- '[[sdk-is-mct-entry-point]]'
- '[[wasi-is-foundation-not-option]]'
- '[[children-have-agency-toys-are-capabilities]]'
exit_criteria:
- id: jamb1-scope-ratified
  text: Scope is locked to job-alignment back-office for Patina/MCT-fit roles (no generic spray-and-pray automation).
  checked: false
- id: jamb2-job-schema-locked
  text: A deterministic normalized job record schema is defined and checked (source URL, company, role, constraints, evidence links, confidence, observed timestamps).
  checked: false
- id: jamb3-alignment-profile-locked
  text: A user-owned alignment profile exists with fail-closed hard gates plus weighted dimensions for soft scoring.
  checked: false
- id: jamb4-typed-child-lane
  text: New scoring/gating capability is implemented as typed children (SDK-first), not ad-hoc prompt-only logic.
  checked: false
- id: jamb5-mother-authority
  text: Mother grant validation for job pipeline children is fail-closed and audit-logged per current governance specs.
  checked: false
- id: jamb6-deterministic-first
  text: Final recommend/reject decision path is deterministic and reproducible from recorded inputs; LLM output is advisory only.
  checked: false
- id: jamb7-dedup-and-history
  text: Re-ingesting same/canonicalized job updates history and emits change signals; duplicate postings are collapsed deterministically.
  checked: false
- id: jamb8-hitl-surface
  text: HITL command flow exists for ingest/review/decide and emits compact session summaries plus file links for auditability.
  checked: false
- id: jamb9-proof-e2e
  text: One end-to-end path (ingest → normalize → score → gate → record) passes with tests and reproducible fixture data.
  checked: false
- id: jamb10-vision-lock-alignment
  text: Spec is explicitly mapped to sdk-vision-lock criteria (svl3/svl5/svl6/svl10) with no contradictory guidance.
  checked: false
---
# feat: Job-Aligned MCT Backoffice

> Build a deterministic, Mother-governed job-alignment pipeline for landing roles aligned with Patina + MCT direction.

## Problem

Current job-search tooling optimizes throughput and convenience, but your goal is
alignment: roles where you can build Rust/Wasm/component-model systems and drive
MCT direction.

Without a deterministic backoffice, role selection drifts toward ad-hoc prompt
judgment, weak auditability, and noisy context churn.

## Goal

Create a typed, deterministic job-alignment pipeline in Patina that:

1. Normalizes role inputs into a strict record.
2. Applies hard alignment gates fail-closed.
3. Computes transparent weighted fit scoring.
4. Preserves evidence/history/change signal for each role.
5. Keeps Mother as capability authority and audit boundary.

## Non-Goals

- Auto-submitting applications.
- Mass outbound automation/spam.
- Replacing human judgment on final career decisions.
- Building a generic recruiting SaaS.

## Target Shape

### 1) Deterministic Job Record Canon

Define a canonical normalized job record (URL + extracted content + structured
fields + confidence + evidence links).

### 2) Alignment Profile as Policy

A user-owned policy file encodes:
- **Hard gates** (must-have / must-not-have).
- **Weighted fit dimensions** (soft scoring).

Hard gates fail closed.

### 3) Typed MCT Pipeline

Pipeline is implemented with typed children and WIT contracts (SDK-first),
reusing existing children where possible and adding a focused scorer/gate child
where needed.

### 4) Mother-Governed Capability Surface

Children declare `[needs].toys` (+ optional scopes), with Mother enforcing and
logging GRANT/DENY decisions.

### 5) Evidence + Change Tracking

Repeated ingest of same canonical role should reuse/compare prior records,
emitting deterministic change cues and preserving decision history.

### 6) Lean HITL Surface

Minimal commands for ingest/review/decide with compact summaries and artifact
links; no prompt flood.

## Solution

1. Add canonical job schema + fixtures.
2. Add alignment policy schema + starter profile.
3. Implement typed scorer/gate path.
4. Wire Mother grant/audit enforcement for job children.
5. Add deterministic dedup/change signal behavior.
6. Add thin HITL command surface for workflow execution.

## Implementation Order

1. **Schema first:** job record + alignment policy + fixtures.
2. **Pipeline core:** normalize + score + gate + record writer path.
3. **Governance:** Mother grant/audit checks and fail-closed behavior.
4. **HITL:** ingest/review/decide commands + compact summaries.
5. **Proof:** fixture-driven E2E and deterministic replay checks.

## Verification

```bash
patina spec show job-aligned-mct-backoffice
patina spec check job-aligned-mct-backoffice --json

# compile/tests
cargo check --workspace -q
cargo test --workspace -q

# targeted tests (to be added by implementation)
# cargo test --test job_alignment_pipeline
# cargo test --test mother_job_grants
```

## Exit Criteria

Frontmatter criteria `jamb1..jamb10` are the source of truth.

## Build Readiness

Medium. Architecture and governance constraints are clear; implementation needs
new typed job-scoring path plus command/runtime wiring.
