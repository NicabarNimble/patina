---
type: belief
id: smart-model-in-room
persona: architect
facets: [llm, architecture, cost]
confidence:
  score: 0.88
  signals:
    evidence: 0.90
    source_reliability: 0.85
    recency: 0.90
    survival: 0.80
    user_endorsement: 0.85
entrenchment: high
status: active
extracted: 2026-01-15
revised: 2026-01-16
---

# smart-model-in-room

Use frontier LLMs for synthesis, not local models.

## Statement

For tasks requiring intelligence (synthesis, conflict resolution, pattern extraction), use the adapter's frontier LLM (Claude/Gemini/OpenCode) rather than local models. Local model optimization is deferred until proven patterns exist.

## Evidence

- [[session-20260115-053944]] - "Frontier LLMs for synthesis, not local models" (weight: 0.90)
- [[session-20260115-053944]] - "Phase 3: Adapter LLM synthesis - smartest model in room" (weight: 0.85)
- [[spec-surface-layer]] - Mother is deterministic daemon, adapters do synthesis (weight: 0.80)

## Supports

- [[dont-build-what-exists]]
- [[deterministic-first]]

## Attacks

- [[local-model-always]] (status: defeated, reason: quality gap too large for synthesis)
- [[privacy-concerns]] (status: scoped, scope: "use local for sensitive data only")

## Attacked-By

- [[cost-concerns]] (status: active, confidence: 0.4, scope: "high-volume synthesis")
- [[latency-concerns]] (status: active, confidence: 0.3, scope: "real-time interactions")

## Scope

- Phase 1-2: Deterministic capture (no LLM needed)
- Phase 3: Adapter LLM synthesis (frontier model)
- Phase 6+: Local model optimization (only after patterns proven)

## Revision Log

- 2026-01-15: Extracted from session, corrected from "local Gemma" (confidence: 0.70)
- 2026-01-16: Spec evidence added (confidence: 0.70 → 0.88)
