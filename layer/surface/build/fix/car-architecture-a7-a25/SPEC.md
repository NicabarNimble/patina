---
type: fix
id: car-architecture-a7-a25
status: draft
created: 2026-03-31
sessions:
  origin: 20260331-224232-852361000
references:
  - layer/core/patina-identity.md
  - layer/core/dependable-rust.md
  - layer/surface/build/feat/child-construction-canon/SPEC.md
beliefs:
  - "[[children-have-agency-toys-are-capabilities]]"
related:
  - layer/surface/build/fix/code-audit-remediation/SPEC.md
  - layer/surface/build/fix/car-safety-a1-a6/SPEC.md
  - layer/surface/build/fix/car-deadcode-a8-a24/SPEC.md
  - src/retrieval/oracles/semantic.rs
  - src/commands/scry/internal/enrichment.rs
exit_criteria:
  - id: car-a7-retrieval-inversion
    text: "retrieval/oracles/semantic no longer imports from commands::scry internals; enrichment logic lives in retrieval/ so Mother can serve scry without CLI dependencies."
    checked: true
  - id: car-a25-intentional-dispatch-proof
    text: "Daemon-first spec dispatch architecture is validated as intentional: dependency graph and call ownership are documented with explicit non-regression checks showing no accidental CLI->library inversion debt."
    checked: true
  - id: car-architecture-proof
    text: "`cargo check --workspace -q`, `cargo test -q --lib`, and `patina scry` sanity checks pass after enrichment move."
    checked: true
---

# fix: Code Audit Remediation — Architecture (A7)

Dependency direction remediation. One gate: retrieval must stand alone.

## Context

Mother serves scry via the daemon. The retrieval engine must be a clean library without CLI command dependencies. Currently `retrieval/oracles/semantic.rs` imports from `commands::scry::internal::enrichment` — the lower layer depends on the higher layer. This blocks Mother from using retrieval standalone.

## A25 Disposition

A25 (spec library → CLI inversion) is **dropped from this spec**. The "inversion" in `spec.rs` is intentional daemon-first dispatch: `spec.rs` provides unified dispatch for both CLI and Mother's spec-manager child. This is the correct architecture per [[child-construction-canon]] where spec-manager is a Mother-hosted child. The import direction serves the daemon pattern, not an accident.

## Scope

- Move enrichment utilities from `commands::scry::internal` to `retrieval/`.
- Verify retrieval module has no remaining imports from `commands::`.

## Constraints

- No behavior expansion.
- No dead code sweeps in this spec.
- Keep the move minimal — same functions, new location.
