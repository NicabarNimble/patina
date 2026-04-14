---
type: feat
id: mother-rivet-correlation-join
status: active
created: 2026-04-14
sessions:
  origin: 20260413-075041-892082000
beliefs:
  - "[[patina-identity]]"
  - "[[spec-driven-design]]"
  - "[[dependable-rust]]"
  - "[[adapter-pattern]]"
  - "[[safety-boundaries]]"
  - "[[unix-philosophy]]"
related:
  - layer/core/values/patina-identity.md
  - layer/core/values/spec-driven-design.md
  - layer/core/values/dependable-rust.md
  - layer/core/values/adapter-pattern.md
  - layer/core/values/safety-boundaries.md
  - layer/core/values/unix-philosophy.md
  - layer/surface/build/feat/mother-rivet-integration/SPEC.md
  - mother/src/runtime.rs
  - mother/src/registry.rs
  - mother/src/http_api.rs
  - src/commands/mother/daemon.rs
  - /Users/nicabar/Projects/Patina/rivet-deno-lab/main.ts
exit_criteria:
  - id: mrc1-child-call-correlation-input
    text: "Mother typed call HTTP payload accepts optional `correlation` object carrying Rivet identifiers without breaking existing payload shape."
    checked: false
    verify: "cargo test -p mother http_api::tests::child_call_route_dispatches_typed_operation -- --nocapture"
  - id: mrc2-observation-persists-correlation
    text: "Typed call observations persist optional correlation data (`rivet_run_id`, `rivet_actor_id`, `rivet_workflow_id`, `rivet_job_id`)."
    checked: false
    verify: "cargo test -p mother registry::tests::observed_typed_call_records_correlation_metadata -- --nocapture"
  - id: mrc3-inspector-correlation-filter
    text: "Inspector typed-call endpoint supports correlation filters and returns only matching calls with accurate count."
    checked: false
    verify: "cargo test -p mother http_api::tests::inspector_typed_calls_filters_by_rivet_run_id -- --nocapture"
  - id: mrc4-fail-open-backcompat
    text: "Calls without correlation metadata still execute and appear in inspector exactly as before (additive-only behavior)."
    checked: false
    verify: "cargo test -p mother http_api::tests::inspector_typed_calls_route_returns_history -- --nocapture"
  - id: mrc5-rivet-bridge-pass-through
    text: "Rivet lab bridge sends correlation metadata in typed call request payload so Mother can join Rivet execution IDs to typed observations."
    checked: false
    verify: "cd /Users/nicabar/Projects/Patina/rivet-deno-lab && deno task smoke"
  - id: mrc6-slice-tests
    text: "Deterministic tests cover success path and filtered inspector path for correlation join behavior."
    checked: false
    verify: "cargo test -p mother http_api::tests::inspector_typed_calls_filters_by_rivet_run_id registry::tests::observed_typed_call_records_correlation_metadata -- --nocapture"
---
# feat: Mother ↔ Rivet typed-call correlation join

> Join Rivet execution identifiers with Mother typed-call observations so operators can trace one Rivet run across the Mother boundary.

## Problem

Mother typed call history currently tracks child/operation/outcome, but Rivet execution identifiers are not persisted with each call. Operators can see what Mother did, but cannot reliably answer which calls belong to a specific Rivet run.

## Goal

Add an additive correlation seam:

1. Rivet adapter sends correlation identifiers in typed call payload.
2. Mother persists those identifiers in typed call observations.
3. Inspector endpoint supports filtering by those identifiers.

This closes the observability join for `mother-rivet-integration` criterion `mri5` without coupling child business contracts to Rivet.

## Non-Goals

- No change to child WIT contracts.
- No Rivet-specific behavior branches in child execution.
- No new orchestration abstraction layer.

## Architecture

- Keep `ChildCallRequest` as Mother invocation authority surface.
- Add optional `correlation` metadata field (additive).
- Store correlation metadata in `TypedCallObservation` (additive).
- Perform inspector filtering at API boundary while preserving current response shape (`count`, `calls`).

## Verification

```bash
cargo test -p mother registry::tests::observed_typed_call_records_correlation_metadata -- --nocapture
cargo test -p mother http_api::tests::inspector_typed_calls_filters_by_rivet_run_id -- --nocapture
cargo test -p mother http_api::tests::inspector_typed_calls_route_returns_history -- --nocapture
```
