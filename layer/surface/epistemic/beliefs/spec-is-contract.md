---
type: belief
id: spec-is-contract
persona: architect
facets: [architecture, process, spec-system]
confidence:
  score: 0.80
  signals:
    evidence: 0.85
    source_reliability: 0.80
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: medium
status: active
extracted: 2026-01-30
revised: 2026-01-30
---

# spec-is-contract

Always verify implementation against spec before committing. The spec defines the contract — if code deviates, fix the code not the spec. Without explicit spec checks, signature mismatches and design shortcuts ship uncaught.

## Statement

Always verify implementation against spec before committing. The spec defines the contract — if code deviates, fix the code not the spec. Without explicit spec checks, signature mismatches and design shortcuts ship uncaught.

## Evidence

- session-20260130-131543: resolve_adapter had hidden current_dir() dependency violating spec signature. Caught only by explicit spec review, not by tests or clippy. (weight: 0.9)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-01-30: Created (confidence: 0.80)
