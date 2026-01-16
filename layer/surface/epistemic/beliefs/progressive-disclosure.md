---
type: belief
id: progressive-disclosure
persona: architect
facets: [architecture, llm, context-management]
confidence:
  score: 0.82
  signals:
    evidence: 0.87
    source_reliability: 0.82
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: medium
status: active
extracted: 2026-01-16
revised: 2026-01-16
---

# progressive-disclosure

Context should be loaded progressively - metadata always available, details on-demand - to manage context window efficiently.

## Statement

Context should be loaded progressively - metadata always available, details on-demand - to manage context window efficiently.

## Evidence

- [[session-20260116-095954]]: Skills use three-level loading (metadata → SKILL.md → resources) as documented in Claude Code skills system (weight: 0.90)

## Supports

- [[smart-model-in-room]] - LLM can request details when needed
- [[skills-for-structured-output]] - Skills implement this pattern

## Attacks

- [[load-everything-upfront]] (status: defeated, reason: wastes context window on unused information)

## Attacked-By

- [[latency-concerns]] (status: active, confidence: 0.3, scope: "when on-demand loading adds noticeable delay")

## Applied-In

- Skills three-level loading: metadata (~100 words) → SKILL.md (<5k) → resources (unbounded)
- Scry results: summary first, full content on request
- Layer system: core (eternal) → surface (active) → dust (archived)

## Revision Log

- 2026-01-16: Created (confidence: 0.82)
