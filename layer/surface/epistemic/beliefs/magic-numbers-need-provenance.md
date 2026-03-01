---
type: belief
id: magic-numbers-need-provenance
persona: architect
facets: [rust, scoring, documentation, maintainability]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-01
revised: 2026-03-01
---

# magic-numbers-need-provenance

Scoring weights, thresholds, and divisors must have documented rationale — not just named constants, but WHY this value. A named constant without provenance is just a magic number with a name tag.

## Statement

Scoring weights, thresholds, and divisors must have documented rationale — not just named constants, but WHY this value. A named constant without provenance is just a magic number with a name tag.

## Evidence

- [[session-20260301-165723]]: Belief health_score uses weights W_USE=0.3, W_TRUTH=0.4, W_FRESH=0.3 with no documented rationale. Magic divisor /3.0 for citation clipping is unexplained. Health threshold 0.4 for "low health" warning is ad-hoc. (weight: 0.9)
- [[session-20260301-165723]]: Activity level thresholds (7/30/90 days) in derive.rs have no comment explaining why these cutoffs. Centrality score divides by 100.0 with no explanation of the normalization basis. (weight: 0.8)
- [[session-20260301-165723]]: EXACT_SEARCH_THRESHOLD=10_000 in semantic.rs IS documented with empirical evidence (P@10 gap measurement) — this is what good looks like. (weight: 0.85)

## Supports

- [[ground-assertions-or-pay-review-tax]] — undocumented magic numbers are ungrounded assertions about what values are "right"

## Attacks

<!-- None known -->

## Attacked-By

- Over-documentation: not every constant needs an essay — mitigated by focusing on scoring/ranking constants where wrong values silently degrade quality

## Applied-In

- `src/retrieval/oracles/semantic.rs:199-202` — EXACT_SEARCH_THRESHOLD documented with P@10 measurement (positive example)

## Revision Log

- 2026-03-01: Created from structural audit findings
