---
type: refactor
id: durable-rust-unix-realignment-program
status: active
created: 2026-04-14
beliefs:
  - "[[dependable-rust]]"
  - "[[unix-philosophy]]"
  - "[[spec-driven-design]]"
  - "[[adapter-pattern]]"
  - "[[safety-boundaries]]"
related:
  - layer/core/values/dependable-rust.md
  - layer/core/values/unix-philosophy.md
  - layer/core/values/spec-driven-design.md
  - layer/core/values/adapter-pattern.md
  - layer/core/values/safety-boundaries.md
  - layer/surface/reports/audit/2026-04-14-durable-rust-unix-realignment-audit.md
  - layer/surface/build/fix/spec-archive-read-path/SPEC.md
  - layer/surface/build/refactor/main-command-router-split/SPEC.md
  - layer/surface/build/refactor/mother-daemon-module-split/SPEC.md
  - layer/surface/build/refactor/mother-http-api-module-split/SPEC.md
  - layer/surface/build/refactor/child-typed-conversion-boundary/SPEC.md
  - layer/surface/build/refactor/typed-operation-identity-hardening/SPEC.md
  - layer/surface/build/fix/spec-create-transaction-boundary/SPEC.md
exit_criteria:
  - id: drup1-archive-fix-landed
    text: "`fix/spec-archive-read-path` is complete and archived-spec show/check behavior is restored with deterministic tests."
    checked: true
  - id: drup2-router-spec-reviewed
    text: "`refactor/main-command-router-split` spec is reviewed and approved in HITL before implementation."
    checked: true
  - id: drup3-daemon-spec-reviewed
    text: "`refactor/mother-daemon-module-split` spec is reviewed and approved in HITL before implementation."
    checked: true
  - id: drup4-http-api-spec-reviewed
    text: "`refactor/mother-http-api-module-split` spec is reviewed and approved in HITL before implementation."
    checked: true
  - id: drup5-conversion-spec-reviewed
    text: "`refactor/child-typed-conversion-boundary` spec is reviewed and approved in HITL with explicit conversion invariants."
    checked: true
  - id: drup6-typed-identity-spec-reviewed
    text: "`refactor/typed-operation-identity-hardening` spec is reviewed and approved in HITL before implementation."
    checked: false
  - id: drup7-spec-create-rewrite-explicitly-deferred
    text: "Spec-create flow rewrite is explicitly deferred to and implemented via dedicated spec `fix/spec-create-transaction-boundary`; no incidental churn lands under this program."
    checked: true
---

# refactor: Durable Rust + Unix realignment program

Program spec for architecture realignment after rapid feature delivery.

## Why

Recent slices landed functional behavior, but code concentration drift increased in core command/runtime modules. This program restores tight module boundaries and one-job-per-module posture without rewrite theater.

## Scope

- Land one immediate correctness fix (`spec-archive-read-path`).
- Stage review-first refactor specs for command/runtime concentration hotspots.
- Keep each repair slice independently verifiable.

## Non-goals

- No broad rewrite of the whole CLI/runtime in one spec.
- No speculative abstraction expansion.
- No partial cleanup of spec-create path; defer to full rewrite spec.

## Execution model (HITL)

- Draft spec
- Review with operator
- Explicit approval
- Implement
- Verify with deterministic tests and CLI proof

No implementation starts without explicit per-slice approval.
