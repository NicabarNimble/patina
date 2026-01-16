---
type: belief
id: spec-first
persona: architect
facets: [development-process, design]
confidence:
  score: 0.85
  signals:
    evidence: 0.90
    source_reliability: 0.85
    recency: 0.80
    survival: 0.85
    user_endorsement: 0.70
entrenchment: high
status: active
extracted: 2026-01-15
revised: 2026-01-16
---

# spec-first

Design before coding. Write specs as artifacts of learning, not blueprints.

## Statement

Prefer designing the solution in a spec document before implementing code. Specs capture where thinking was at that moment and serve as exploration artifacts.

## Evidence

- [[session-20260115-121358]] - "Spec first, spike second" pattern observed (weight: 0.9)
- [[session-20260115-053944]] - Spec review before implementation (weight: 0.8)
- [[spec-surface-layer]] - Example of spec-driven design (weight: 0.7)

## Supports

- [[exploration-driven-development]]
- [[measure-first]]

## Attacks

- [[move-fast-break-things]] (status: defeated, reason: leads to rework)

## Attacked-By

- [[analysis-paralysis]] (status: active, confidence: 0.3, scope: "only when spec exceeds 1 week")

## Revision Log

- 2026-01-15: Extracted from session-20260115-121358 (confidence: 0.7)
- 2026-01-16: Multiple session evidence added (confidence: 0.7 → 0.85)
