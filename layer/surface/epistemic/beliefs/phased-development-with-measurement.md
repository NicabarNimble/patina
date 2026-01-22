---
type: belief
id: phased-development-with-measurement
persona: architect
facets: [methodology, architecture, measurement, development-process]
confidence:
  score: 0.89
  signals:
    evidence: 0.94
    source_reliability: 0.89
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: high
status: active
extracted: 2026-01-17
revised: 2026-01-17
---

# phased-development-with-measurement

Complex features should be built in phases, with each phase starting with measurement to establish baseline before implementation.

## Statement

Complex features should be built in phases, with each phase starting with measurement to establish baseline before implementation. Each phase defines success criteria before coding, validates with metrics, then proceeds to next phase.

## Evidence

- [[spec-mothership-graph]] G0 (baseline measurement) → G1 (graph CLI) → G2 (routing) → G2.5 (feedback loop) - each phase started with metrics (weight: 0.92)
- [[spec-epistemic-layer]] E0 (prototype) → E1 (manual population) → E2 (creation system) → E3 (scry integration) - phased with validation criteria (weight: 0.90)
- [[spec-epistemic-layer]] Andrew Ng methodology: "Establish a baseline first" before iterating (weight: 0.88)
- Git history shows G0-G2.5 commits each tagged phase completion with measurement results (weight: 0.85)

## Supports

- [[measure-first]] - Each phase starts with measurement
- [[spec-first]] - Phase design documents baseline and success criteria
- [[error-analysis-over-architecture]] - Measurement reveals what next phase needs

## Attacks

- [[big-bang-release]] (status: defeated, reason: "no metrics to validate progress")
- [[continuous-iteration]] (status: scoped, reason: "iteration happens within phases, not instead of phases")

## Attacked-By

- [[phase-overhead]] (status: active, confidence: 0.4, scope: "acceptable for complex features, skip for trivial changes")
- [[phase-paralysis]] (status: active, confidence: 0.3, scope: "mitigated by clear exit criteria per phase")

## Applied-In

- Mothership graph: G0 (baseline 2.7), G1 (graph management), G2 (routing improved to 4.9), G2.5 (feedback loop)
- Epistemic layer: E0 (5 beliefs), E1 (20+ beliefs goal), E2 (skill system), E3 (scry integration)
- Forge abstraction: Phase 1 (reader trait), Phase 2 (scraper migration), Phase 3 (PR context), Phase 4 (writer trait)

## Revision Log

- 2026-01-17: Created (confidence: 0.89)
