---
type: value
id: adapter-pattern
status: active
entrenchment: very-high
facets: [architecture, patterns, traits, gjengset]
references: [dependable-rust, unix-philosophy, gjengset-lens-type-integrity]
created: 2026-02-27
revised: 2026-04-03
distilled_from: layer/core/adapter-pattern.md
---
# Adapter Pattern

Define trait boundaries at external system edges. Prove the boundary with 2+ real implementations — don't abstract speculatively. Honest signatures, domain types at the seam, implementation details behind the boundary.

## Test

Do you have 2+ real implementations? Can you write a useful mock without changing calling code? If not, the trait boundary is either premature or in the wrong place.

## Consequence

External systems change independently of Patina. Implementations are swappable black boxes. Testing is clean. But only where proven — a trait with one implementation is ceremony, not architecture.
