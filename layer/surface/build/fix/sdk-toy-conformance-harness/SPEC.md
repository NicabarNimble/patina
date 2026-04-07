---
type: fix
id: sdk-toy-conformance-harness
status: abandoned
created: 2026-04-07
related:
  - layer/surface/build/fix/sdk-upstream-toy-sync/SPEC.md
  - src/commands/mother/toys.rs
exit_criteria:
  - id: uts7-wasi-testsuite-tracked
    text: "`wasi-testsuite` repo is integrated into Patina workflow with reproducible runner setup and result reporting."
    checked: false
  - id: uts8-testsuite-passes
    text: "WASI Preview 2 conformance passes for implemented interfaces, with explicit documentation for expected unsupported areas."
    checked: false
  - id: uts9-patina-toy-tests
    text: "Patina toy contract conformance suites exist and run in CI."
    checked: false
---
# fix: SDK Toy Conformance Harness

## Status

Abandoned for now while core runtime and product stabilization continues.

## Reason

Conformance harness integration (WASI testsuite and Patina toy suite) is
valuable but currently deprioritized relative to core Mother/child runtime
stability and binding model migration.

## Reopen Conditions

- Binding-oriented resource model is stabilized.
- Runtime capability naming cleanup is complete.
- CI/runtime dependency strategy for conformance tooling is approved.
