---
type: fix
id: car-architecture-a7-a25
status: draft
created: 2026-03-31
sessions:
  origin: 20260331-224232-852361000
references:
  - layer/core/patina-identity.md
  - layer/core/unix-philosophy.md
  - layer/core/spec-driven-design.md
  - layer/core/dependable-rust.md
related:
  - layer/surface/build/fix/code-audit-remediation/SPEC.md
  - layer/surface/build/fix/car-safety-a1-a6/SPEC.md
  - layer/surface/build/fix/car-deadcode-a8-a24/SPEC.md
exit_criteria:
  - id: car-a7-retrieval-inversion
    text: "retrieval/oracles/semantic no longer imports from commands::scry internals; dependency direction is retrieval -> retrieval/shared only."
    checked: false
  - id: car-a25-spec-lib-inversion
    text: "spec library no longer imports from commands::spec::internal; dispatch ownership is moved so library is upstream of CLI."
    checked: false
  - id: car-architecture-proof
    text: "`cargo check --workspace -q`, `cargo test -q --lib`, `patina scry` sanity checks, and core `patina spec` command surface checks pass after inversion changes."
    checked: false
---

# fix: Code Audit Remediation — Architecture (A7, A25)

Dependency direction remediation only.

## Scope

- Remove retrieval -> commands inversion.
- Remove spec library -> commands inversion.

## Constraints

- No behavior expansion.
- No dead code sweeps in this spec.
- Keep boundary moves minimal and test-visible.
