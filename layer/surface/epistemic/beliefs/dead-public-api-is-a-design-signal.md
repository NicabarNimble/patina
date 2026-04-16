---
type: belief
id: dead-public-api-is-a-design-signal
persona: architect
facets: [api-design, code-quality, architecture]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-10
revised: 2026-04-10
---

# dead-public-api-is-a-design-signal

Compiler warnings on unused public exports signal a real design gap, not a noise problem — don't suppress them, investigate why no consumer needs what you're exporting.

## Statement

Compiler warnings on unused public exports signal a real design gap, not a noise problem — don't suppress them, investigate why no consumer needs what you're exporting.

## Evidence

- [[session-20260409-143847-707078000]] - SDK type re-export discussion: a pub use was producing unused-import warnings, suggesting suppression with #[allow] looked like it solved the problem but was hiding a real architectural mistake — the types weren't consumable across crate boundaries given the type system constraints (weight: 0.9)
- [[commit-78902215]] - Audit findings on a fix spec called out the dead public exports as a real design gap; the spec's exit criteria require removing them, not silencing the warning
- [[spec-sdk-public-surface-alignment]] - The fix spec exists because the dead exports were a design signal that the API surface was wrong, not because the warnings were noisy

## Supports

- [[compiler-enforced-safety]] — taking compiler signals seriously is the foundation of compiler-enforced safety; suppressing them undermines the practice
- [[dependable-rust]] — small public interfaces are honest; dead exports inflate the public surface beyond what the design can keep

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- A fix spec was opened to remove dead public re-exports from a library crate after the compiler warning was traced to a real architectural impossibility (separate code generators producing distinct types from the same source contract). The resolution kept the internal substrate but dropped the public promise the architecture couldn't keep.

## Revision Log

- 2026-04-10: Created — metrics computed by `patina scrape`
