---
type: belief
id: error-analysis-over-architecture
persona: architect
facets: [methodology, measurement, development-process]
confidence:
  score: 0.88
  signals:
    evidence: 0.93
    source_reliability: 0.88
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: medium
status: active
extracted: 2026-01-17
revised: 2026-01-17
---

# error-analysis-over-architecture

When systems fail or underperform, analyze failure cases before adding complexity.

## Statement

When systems fail or underperform, analyze failure cases before adding complexity. Follow Andrew Ng's methodology: "Show me the failure cases" → error analysis → fix data gaps before architectural changes.

## Evidence

- [[spec-epistemic-layer]] Andrew Ng methodology section: "Error analysis on real examples" (weight: 0.9)
- [[session-20260116-221800]] Q1 baseline failed (score 2) → error analysis revealed missing sync-first belief → treatment improved to 5 (weight: 0.85)
- [[session-20260116-080414]] Q10 analysis identified SQLite preference scattered across beliefs, not explicit (weight: 0.8)

## Supports

- [[measure-first]] - Measurement reveals what to fix
- [[spec-first]] - Specs document learnings from error analysis

## Attacks

- [[premature-optimization]] (status: defeated, reason: "optimize after measuring failures")
- [[architecture-first]] (status: scoped, reason: "architecture comes after understanding failure modes")

## Attacked-By

- [[move-fast-break-things]] (status: defeated, confidence: 0.3, scope: "valid for throwaway prototypes only")
- [[analysis-paralysis]] (status: active, confidence: 0.4, scope: "only when error analysis exceeds 2 days without findings")

## Applied-In

- [[spec-epistemic-layer]] Evaluation section - Q1-Q10 error analysis revealed data gaps
- [[session-20260116-221800]] CI failure → error analysis → found config.toml gitignore issue
- [[spec/mothership-graph]] G0-G2.5 phases all start with measurement before implementation

## Revision Log

- 2026-01-17: Created (confidence: 0.88)
