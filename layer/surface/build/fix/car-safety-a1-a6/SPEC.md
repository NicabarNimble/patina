---
type: fix
id: car-safety-a1-a6
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
  - layer/surface/build/fix/car-architecture-a7-a25/SPEC.md
exit_criteria:
  - id: car-a1-utf8-panic
    text: "enrichment.rs byte-slice truncation replaced with char-boundary-safe truncation and regression test for multi-byte content."
    checked: false
  - id: car-a2-cwd-thread-safety
    text: "Retrieval path removes process-global CWD mutation; repo paths are parameterized through query/oracle pipeline."
    checked: false
  - id: car-a3-capability-divergence
    text: "Capability checks unified behind one canonical function; test proves identical behavior across both call paths."
    checked: false
  - id: car-a4-starting-commit-stub
    text: "Mother session starting commit is persisted and returned from real session data, not hardcoded 'none'."
    checked: false
  - id: car-a5-dimension-mismatch
    text: "Belief grounding reads index dimensions dynamically; both 256-d projected and 768-d raw indexes work."
    checked: false
  - id: car-a6-frontmatter-dedup
    text: "Duplicated SessionFrontmatter types are consolidated on canonical artifact types with schema drift removed."
    checked: false
  - id: car-safety-proof
    text: "`cargo check --workspace -q`, `cargo test -q --lib`, and targeted functional checks for scry/session/mother safety paths all pass."
    checked: false
---

# fix: Code Audit Remediation — Safety (A1-A6)

Safety and correctness only. No dead code sweeps or broad refactors are authorized here.

## Scope

- UTF-8 panic safety in enrichment truncation.
- Retrieval thread safety by removing global CWD mutation.
- Capability authorization consistency.
- Mother session starting commit correctness.
- Embedding index dimension correctness.
- Session frontmatter schema canonicalization.

## Constraints

- Keep diffs surgical and test-backed.
- Do not add new features.
- Do not mix with architecture inversion or cleanup gates.
