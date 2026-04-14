---
type: feat
id: mother-rivet-integration
status: active
created: 2026-04-14
sessions:
  origin: 20260414-090000-000000000
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
  - mother/src/runtime.rs
  - mother/src/registry.rs
  - src/commands/mother/daemon.rs
  - src/child/internal/child.rs
  - layer/surface/build/feat/mother-wit-dispatcher/SPEC.md
  - layer/surface/build/feat/mother-typed-invocation-driver/SPEC.md
  - layer/surface/build/feat/pando-delivery-policy/SPEC.md
  - layer/surface/build/feat/mother-rivet-integration/PORTABILITY-SEAM.md
  - sdk/patina-sdk/README.md
exit_criteria:
  - id: mri1-integration-profile
    text: "Mother has an explicit runtime profile for Rivet integration (enabled/disabled) with no behavior change when disabled."
    checked: true
  - id: mri2-wasi-authority-preserved
    text: "Rivet integration does not bypass Mother typed invocation + Wasmtime component execution authority for child business calls."
    checked: true
  - id: mri3-rivet-dispatch-adapter
    text: "Rivet queue/workflow inputs can be translated into Mother typed child calls (`ChildCallRequest`) through one adapter boundary."
    checked: true
  - id: mri4-delivery-policy-mapping
    text: "`required|best-effort|dead-letter` delivery semantics are mapped consistently onto Rivet-triggered execution paths."
    checked: true
  - id: mri5-observability-join
    text: "Mother typed call observations are correlated with Rivet execution identifiers for inspector/debugging workflows."
    checked: true
  - id: mri6-folder-watch-rivet-proof
    text: "folder-watch-actor flow runs through Rivet-triggered orchestration and preserves typed contract behavior end-to-end."
    checked: false
  - id: mri7-sdk-guidance
    text: "patina-sdk guidance remains backend-neutral: child authors target WIT + toys only, not Rivet-specific APIs."
    checked: true
  - id: mri8-portability-seam
    text: "A minimal orchestration seam is documented so future non-Rivet backends can be added without changing business contracts."
    checked: true
---
# feat: Mother Rivet integration

> Integrate Mother deeply with Rivet for orchestration now, while preserving strict WASI/component-model execution and a minimal future portability seam.

## Problem

We want to stop rebuilding orchestration primitives from scratch, but we do not want Mother or child business contracts locked to one external platform.

## Goal

Use Rivet for actor/workflow/queue/scheduling orchestration while keeping:

1. **Mother authoritative for policy + typed invocation semantics**
2. **WASI/component execution authoritative for child business code**
3. **WIT business contracts as portable source of truth**

## Scope

- Add a Rivet integration profile to Mother runtime.
- Add one adapter path from Rivet-triggered jobs/events into Mother typed child calls.
- Preserve existing non-Rivet daemon behavior.
- Join observability between Rivet run IDs and Mother typed-call observations.

## Non-goals

- Rewriting child contracts in Rivet-native type systems.
- Moving child execution into a non-WASI runtime lane.
- Building a fully generic orchestration abstraction upfront.

## Architecture lock

- **Business contracts**: WIT (`patina:*`) and child manifests.
- **Capabilities**: WASI/toys only.
- **Execution**: Mother typed invocation into Wasmtime component children.
- **Orchestration substrate**: Rivet-first integration for now.

Rivet is used as orchestration infrastructure, not business-contract authority.

## Phased delivery

### Phase A — profile + adapter skeleton
- Add Mother runtime profile flag for Rivet mode.
- Add adapter ingress that translates Rivet payload -> `ChildCallRequest`.

### Phase B — policy + observability parity
- Ensure delivery policy mapping remains exact.
- Correlate Mother typed call observations with Rivet run/job identifiers.

### Phase C — proof flow
- Run folder-watch typed operations via Rivet-triggered path.
- Prove no watcher-specific Mother branches are required.

### Phase D — portability seam documentation
- Freeze minimal seam for alternate backends.
- Keep implementation Rivet-deep until a second backend is real.

## Verification (target commands)

```bash
patina spec check mother-rivet-integration --json

# existing typed call + policy baselines must stay green
patina spec check mother-wit-dispatcher --json
patina spec check mother-typed-invocation-driver --json
patina spec check pando-delivery-policy --json
```

## Completeness gates (hard)

To avoid MVP theater, each criterion is only marked checked when code + tests + runtime proof are all present.

- **mri1** requires: `--rivet` profile flag, `/health.rivet_integration`, CLI status rendering, and one live start probe in each mode.
- **mri2** requires: Rivet path invokes `ChildCallRequest` through Mother registry typed-call path (no direct child bypass), with unit test proof.
- **mri3** requires: one explicit Rivet adapter ingress (`/api/rivet/dispatch`) translating payload -> typed call request, with route + handler tests.
- **mri4** requires: `required|best-effort|dead-letter` behavior implemented on Rivet dispatch path, including dead-letter reroute semantics, with targeted tests.
- **mri5** requires: correlation metadata persisted on typed call observations and queriable via inspector filters.
- **mri6** requires: live folder-watch flow dispatched via Rivet adapter (`/api/rivet/dispatch`) with successful typed response and inspector correlation evidence.
- **mri7** requires: SDK guidance explicitly states backend-neutral authoring (WIT + toys), and rejects orchestrator-specific child APIs.
- **mri8** requires: minimal orchestration seam doc listing the only backend-specific boundary objects/routes so a second backend can be added without contract edits.

## Completeness audit snapshot (2026-04-14)

- **mri1**: complete — CLI flag + health/status surface + tests.
- **mri2**: complete — Rivet ingress uses `ChildCallRequest` through registry typed-call path.
- **mri3**: complete — single Rivet ingress adapter route/handler wired and tested.
- **mri4**: complete — adapter implements `required|best-effort|dead-letter` mapping with targeted tests.
- **mri5**: complete — correlation metadata persisted + inspector filter support.
- **mri6**: **open** — live `folder-watch-actor` flow through Rivet adapter still pending.
- **mri7**: complete — SDK docs now explicitly lock backend-neutral authoring model.
- **mri8**: complete — portability seam frozen in `PORTABILITY-SEAM.md`.

## Notes

This spec intentionally favors **deep integration now** and **late extraction** of seams, aligned to adapter-pattern discipline: extract only after at least two real implementations exist.
