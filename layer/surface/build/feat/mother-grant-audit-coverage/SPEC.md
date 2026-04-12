---
type: feat
id: mother-grant-audit-coverage
status: draft
created: 2026-04-11
sessions:
  origin: 20260410-220235-028265000
related:
- src/commands/mother/daemon.rs
- src/commands/mother/loader.rs
- src/child/internal/child.rs
- src/child/internal/mod.rs
- src/child/internal/host_support.rs
- tests/pando_parity.rs
- tests/wasm_integration.rs
- layer/surface/build/feat/sdk-vision-lock/SPEC.md
- layer/surface/build/refactor/child-typed-composition/SPEC.md
beliefs:
- '[[children-have-agency-toys-are-capabilities]]'
- '[[world-boundary-is-type-safety]]'
exit_criteria:
- id: mgac1-outside-grant-events
  text: "Mother emits structured grant events for outside toy/capability decisions at child load (GRANT and DENY with child, toy/capability, reason)."
  checked: true
- id: mgac2-typed-wiring-events
  text: "Typed composition wiring decisions emit structured grant/audit events for each inside-toy link attempt (from, to, toy, outcome, reason)."
  checked: true
- id: mgac3-fail-closed-deterministic
  text: "Unauthorized or invalid grants/wiring fail closed with deterministic error messages and non-zero load outcome; no silent skips."
  checked: true
- id: mgac4-audit-surface
  text: "Audit events are queryable through an existing Mother event/measure/log surface (documented retrieval path for operators)."
  checked: true
- id: mgac5-test-coverage
  text: "Tests cover positive and negative cases: denied toy grant, denied typed wiring, and expected audit event presence/shape."
  checked: true
- id: mgac6-backward-safe
  text: "Handle-based service children continue to load with equivalent fail-closed behavior; audit coverage adds observability without widening authority."
  checked: true
---
# feat: Mother grant audit coverage for fail-closed composition

> Implement structured grant audit logging for outside toy grants and typed inside-toy wiring decisions, including explicit DENY reasons and deterministic load failures.

## Problem

Mother already enforces many fail-closed checks (manifest capability validation,
call-time toy gating, typed wiring failure). But grant decisions are not yet
reported as a complete, structured audit trail.

This leaves a gap between actual safety behavior and operator visibility.

## Goal

Provide full grant-audit coverage so operators can answer:

1. What was requested?
2. What was granted or denied?
3. Why?
4. Which composition link failed (if any)?

## Non-Goals

- Redesigning grant policy model itself.
- Replacing existing runtime checks.
- Building a new storage system for audit data.

## Current State

- Fail-closed checks exist in manifest and call paths.
- Typed composition wiring fails fast in `compose_typed_component`.
- Audit output is partial/inconsistent across paths.

## Target Shape

- Unified structured event shape for grant decisions.
- Coverage for both lanes:
  - outside toy/capability grants at child load time
  - inside typed wiring grants at pando composition time
- Deterministic deny reasons included in event payload.

## Solution

1. Define canonical grant-audit event schema.
2. Emit events during load-time capability checks.
3. Emit events during typed composition wiring attempts.
4. Ensure denied operations emit DENY then fail closed.
5. Add tests for event presence + deterministic failures.

## Implementation Order

1. Event schema + helper utilities.
2. Integrate with child load/capability validation path.
3. Integrate with typed composition wiring path.
4. Add tests and operator retrieval documentation.

## Resolved Decisions

- Fail-closed remains authority mechanism; audit is observability contract.
- No silent denials: every denial must be visible and attributable.

## Verification

```bash
patina spec check mother-grant-audit-coverage --json
cargo test --test wasm_integration
cargo test --test pando_parity
cargo check --workspace -q
```

## Operator Retrieval Path (mgac4)

`mother.grant` events are emitted into `eventlog` and retrievable through existing event surfaces.

```bash
# JSONL replica surface
patina events export
rg '"event_type":"mother.grant"' layer/events.jsonl

# Direct sqlite surface (local runtime db)
sqlite3 .patina/local/data/events.db \
  "select seq,timestamp,data from eventlog where event_type='mother.grant' order by seq desc limit 20;"
```

## Exit Criteria

Frontmatter `mgac1..mgac6` are source of truth.

## Build Readiness

Medium-High. Core enforcement exists; primary work is coverage and consistency.
