---
type: belief
id: gjengset-lens-type-integrity
persona: architect
facets: [architecture, rust, type-safety, design-philosophy]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-03
revised: 2026-03-03
---

# gjengset-lens-type-integrity

Encode invariants in types whenever possible. Replace runtime validation with compile-time guarantees. Domain primitives escaping as String/i64/etc are entropy — they allow invalid states the compiler cannot catch. Failure must be loud and deterministic, never silently swallowed.

## Statement

Encode invariants in types whenever possible. Replace runtime validation with compile-time guarantees. Domain primitives escaping as String/i64/etc are entropy — they allow invalid states the compiler cannot catch. Failure must be loud and deterministic, never silently swallowed.

## Evidence

- [[session-20260303-101839]]: Formalized from Jon Gjengset's Rust philosophy as applied across Patina sessions. boundary-string-internal-enum captures the boundary pattern; this belief captures the broader principle. Applied in version-consolidation (BumpType enum over bare string), data-fast-incremental audit (Gjengset advised O(delta) not O(n) — the type-level insight that work should be proportional to change size, not project size) (weight: 0.9)

## Supports

- [[transparent-complexity]] — types make invariants compiler-visible
- [[boundary-string-internal-enum]] — specific application of type integrity at serialization boundaries

## Attacks

## Attacked-By

## Applied-In

- `src/plugin/internal/mod.rs:30-35` — PluginWorld enum over string dispatch
- Version consolidation: BumpType enum over `&str`, PreparedRelease typestate over "remember to call preflight"
- [[session-20260303-090741]]: Gjengset advised O(delta) not O(n) — work proportional to change size, not project size

## Revision Log

- 2026-03-03: Created — metrics computed by `patina scrape`
