---
type: fix
id: car-dedup-a9-a21
status: draft
created: 2026-03-31
sessions:
  origin: 20260331-224232-852361000
references:
  - layer/core/patina-identity.md
  - layer/core/dependable-rust.md
  - layer/core/unix-philosophy.md
related:
  - layer/surface/build/fix/code-audit-remediation/SPEC.md
  - layer/surface/build/fix/car-deadcode-a8-a24/SPEC.md
exit_criteria:
  - id: car-a9-path-truth
    text: "Highest-traffic path duplication sites are migrated to crate::paths and no new hardcoded .patina path literals are introduced."
    checked: false
  - id: car-a17-safe-identifier-dedup
    text: "Duplicate safe identifier helpers are unified into one shared implementation."
    checked: false
  - id: car-a18-strip-frontmatter-dedup
    text: "Duplicate strip_frontmatter implementations are unified behind one canonical function."
    checked: false
  - id: car-a19-extract-section-dedup
    text: "Duplicate extract_section_items implementations are unified with consistent number parsing behavior."
    checked: false
  - id: car-a20-semver-dedup
    text: "Semver bump logic is consolidated to one source used by release/dev paths."
    checked: false
  - id: car-a21-test-helper-dedup
    text: "Temp patina-home test helper is extracted and reused across test modules."
    checked: false
  - id: car-dedup-proof
    text: "Compile and lib tests pass; behavior-equivalence checks for touched utilities pass."
    checked: false
---

# fix: Code Audit Remediation — Dedup and Path Truth (A9, A17-A21)

Consolidation-only spec. Keep behavior stable while reducing divergence.

## Constraints

- Canonicalize toward existing module truth (`paths`, release internals, shared scrape helpers).
- No broad refactors beyond the listed duplicate families.
