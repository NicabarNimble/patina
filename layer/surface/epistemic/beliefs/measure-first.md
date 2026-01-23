---
type: belief
id: measure-first
persona: architect
facets: [engineering, quality, methodology]
confidence:
  score: 0.88
  signals:
    evidence: 0.90
    source_reliability: 0.85
    recency: 0.85
    survival: 0.90
    user_endorsement: 0.80
entrenchment: high
status: active
extracted: 2026-01-05
revised: 2026-01-16
---

# measure-first

Measure before building. Prove the problem exists with data.

## Statement

Before building infrastructure, measure the current state and prove the problem exists. Andrew Ng's principle: "If you can't show me the failure cases, you don't understand the problem."

## Evidence

- [[spec/mothership-graph]] - Phase G0 measured dumb routing before building graph (weight: 0.95)
- [[session-20260105]] - "Don't build infrastructure before proving the problem exists with data" (weight: 0.90)
- [[spec-review-q4-2025]] - MRR regression caught because of measurement culture (weight: 0.85)

## Supports

- [[spec-first]]
- [[dont-build-what-exists]]

## Attacks

- [[build-it-they-will-come]] (status: defeated)
- [[intuition-is-enough]] (status: defeated)

## Attacked-By

- [[measurement-overhead]] (status: active, confidence: 0.25, scope: "trivial changes")

## Applied-In

- Graph routing: 0% repo recall (dumb) → 100% (smart) proved the gap
- Quality gates: MRR baseline 0.624 established
- Ref repo semantic: measured before indexing

## Related

- [[andrew-ng-methodology]]

## Revision Log

- 2026-01-05: Extracted from mother-graph G0 phase (confidence: 0.75)
- 2026-01-08: Quality gate evidence added (confidence: 0.75 → 0.85)
- 2026-01-16: Q4 review evidence added (confidence: 0.85 → 0.88)
