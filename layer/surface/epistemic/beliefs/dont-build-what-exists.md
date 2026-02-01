---
type: belief
id: dont-build-what-exists
persona: architect
facets: [engineering, efficiency]
confidence:
  score: 0.90
  signals:
    evidence: 0.95
    source_reliability: 0.90
    recency: 0.85
    survival: 0.90
    user_endorsement: 0.80
entrenchment: high
status: active
extracted: 2026-01-15
revised: 2026-01-16
---

# dont-build-what-exists

Reuse existing infrastructure before building new.

## Statement

Before implementing new functionality, check what already exists. 90% of what you need may already be built - you just need to orchestrate it.

## Evidence

- [[session-20260115-053944]] - "Connection scoring uses existing tools" (weight: 0.95)
- [[session-20260115-053944]] - "90% of Mother's job already exists in oxidize/scry" (weight: 0.90)
- [[session-20260116-054624]]: [[commit-6aa7817e]] - Used existing assay tables for derive queries (weight: 0.85)

## Supports

- [[measure-first]]
- [[unix-philosophy]]
- [[smart-model-in-room]]

## Attacks

- [[not-invented-here]] (status: defeated)
- [[premature-abstraction]] (status: defeated)

## Attacked-By

- [[technical-debt-from-reuse]] (status: active, confidence: 0.2, scope: "when reused code doesn't fit well")

## Applied-In

- Mother architecture - orchestrates existing oxidize/scry
- Connection scoring - uses existing embeddings
- Graph routing - uses existing semantic search

## Revision Log

- 2026-01-15: Extracted from session-20260115-053944 (confidence: 0.75)
- 2026-01-16: Commit evidence added, entrenchment increased (confidence: 0.75 → 0.90)
