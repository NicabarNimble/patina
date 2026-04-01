---
type: fix
id: car-dedup-a9-a21
status: ready
created: 2026-03-31
sessions:
  origin: 20260331-224232-852361000
related:
- layer/surface/build/fix/code-audit-remediation/SPEC.md
- layer/surface/build/fix/car-deadcode-a8-a24/SPEC.md
- src/paths.rs
- src/project/internal.rs
- src/version.rs
- src/commands/mother/daemon.rs
- src/commands/launch/internal.rs
- src/commands/ai/surface.rs
- src/commands/scrape/events.rs
- src/commands/scrape/projection.rs
- src/commands/oxidize/mod.rs
- src/commands/oxidize/beliefs.rs
- src/commands/spec/internal/packets.rs
- src/commands/spec/internal/queries.rs
- src/release/internal.rs
- src/commands/dev/release.rs
- src/commands/dev/bump_version.rs
- src/test_support.rs
references:
- layer/core/patina-identity.md
- layer/core/dependable-rust.md
- layer/core/unix-philosophy.md
exit_criteria:
- id: car-a9-path-truth
  text: Highest-traffic path duplication sites are migrated to crate::paths and no new hardcoded .patina path literals are introduced.
  checked: true
- id: car-a17-safe-identifier-dedup
  text: Duplicate safe identifier helpers are unified into one shared implementation.
  checked: true
- id: car-a18-strip-frontmatter-dedup
  text: Duplicate strip_frontmatter implementations are unified behind one canonical function.
  checked: true
- id: car-a19-extract-section-dedup
  text: Duplicate extract_section_items implementations are unified with consistent number parsing behavior.
  checked: true
- id: car-a20-semver-dedup
  text: Semver bump logic is consolidated to one source used by release/dev paths.
  checked: true
- id: car-a21-test-helper-dedup
  text: Temp patina-home test helper is extracted and reused across test modules.
  checked: true
- id: car-dedup-proof
  text: Compile and lib tests pass; behavior-equivalence checks for touched utilities pass.
  checked: true
---

# fix: Code Audit Remediation — Dedup and Path Truth (A9, A17-A21)

Consolidation-only spec. Keep behavior stable while reducing divergence.

## Constraints

- Canonicalize toward existing module truth (`paths`, release internals, shared scrape helpers).
- No broad refactors beyond the listed duplicate families.
