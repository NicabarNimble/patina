---
type: fix
id: car-safety-a1-a6
status: draft
created: 2026-03-31
sessions:
  origin: 20260331-224232-852361000
references:
  - layer/core/patina-identity.md
  - layer/core/dependable-rust.md
  - layer/core/safety-boundaries.md
  - layer/surface/build/feat/child-construction-canon/SPEC.md
beliefs:
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[audit-before-refactor]]"
related:
  - layer/surface/build/fix/code-audit-remediation/SPEC.md
  - layer/surface/build/fix/car-architecture-a7-a25/SPEC.md
  - src/commands/scry/internal/enrichment.rs
  - src/retrieval/engine.rs
  - src/retrieval/oracles/semantic.rs
  - src/commands/scry/internal/search.rs
  - src/child/internal/mod.rs
  - src/child/internal/knowledge_child.rs
  - src/child/internal/tests.rs
  - mother/src/state.rs
  - src/commands/belief/mod.rs
  - src/commands/session/internal.rs
  - src/session/internal/artifact.rs
exit_criteria:
  - id: car-a1-utf8-panic
    text: "enrichment.rs byte-slice truncation replaced with char-boundary-safe truncation and regression test for multi-byte content."
    checked: false
  - id: car-a2-cwd-thread-safety
    text: "Retrieval path no longer mutates process-global CWD in any concurrent or batch execution path. Panic-safe restoration applies only to legacy serialized fallback paths. Cross-repo scry works without CWD corruption."
    checked: false
  - id: car-a3-capability-divergence
    text: "Capability check divergence resolved. If both check points are needed (manifest-time vs instantiation-time), auto_granted lists are proven identical via test. If only one is needed, the duplicate is removed."
    checked: false
  - id: car-a4-starting-commit-stub
    text: "Mother session starting commit is persisted and returned from real session data, not hardcoded 'none'."
    checked: false
  - id: car-a5-dimension-mismatch
    text: "Belief grounding reads index dimensions dynamically; both 256-d projected and 768-d raw indexes work."
    checked: false
  - id: car-a6-frontmatter-dedup
    text: "Single canonical SessionFrontmatter type used by both library and CLI. project_uid is Option<String> to handle pre-UID sessions. Duplicated struct and parser deleted."
    checked: false
  - id: car-safety-proof
    text: "`cargo check --workspace -q`, `cargo test -q --lib`, and targeted functional checks for scry/session/mother safety paths all pass."
    checked: false
---

# fix: Code Audit Remediation — Safety (A1-A6)

Safety and correctness only. No dead code sweeps or broad refactors are authorized here.

## Context

Patina's architecture is: protocol core (native CLI verbs), Mother (daemon hosting children), children (WASM compute legos), toys (sandbox capability grants). Per [[child-construction-canon]], the capability check (A3) is the security boundary for the entire child/toy system — every child's toy access flows through it. Per [[patina-identity]], scry is protocol core and must work both as native CLI and served by Mother (A2). Sessions are the evolve verb — Mother must preserve their data correctly (A4, A6).

## Scope

- A1: UTF-8 panic safety in enrichment truncation.
- A2: Retrieval CWD safety — panic-safe restore now, daemon-compatible path resolution direction.
- A3: Capability check consistency — investigate whether two check points serve different purposes before unifying. The toy grant boundary is security-critical per [[children-have-agency-toys-are-capabilities]].
- A4: Mother session starting commit correctness.
- A5: Embedding index dimension correctness for belief grounding.
- A6: Session frontmatter canonical type — adopt Option<String> for backward compat with 538 pre-UID sessions.

## Constraints

- Keep diffs surgical and test-backed.
- Do not add new features.
- Do not mix with architecture inversion or cleanup gates.
- A3: read the instantiation path before deciding whether to delete or unify. Per [[audit-before-refactor]], understand first.
