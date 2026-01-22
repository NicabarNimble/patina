---
type: belief
id: spec-needs-code-verification
persona: architect
facets: [process, specs, refactoring]
confidence:
  score: 0.85
  signals:
    evidence: 0.90
    source_reliability: 0.85
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: medium
status: active
extracted: 2026-01-21
revised: 2026-01-21
---

# spec-needs-code-verification

Refactoring specs require a read-code verification pass before implementation; static dependency tracing misses ~30% of affected paths (callers, assets, helper functions)

## Statement

Refactoring specs require a read-code verification pass before implementation; static dependency tracing misses ~30% of affected paths (callers, assets, helper functions)

## Evidence

- session-20260121-212603: remove-dev-env spec missed launch/internal.rs caller, template assets, init helpers - found during implementation (weight: 0.9)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- **remove-dev-env refactor**: Spec was ~70% accurate; verification pass with `grep` and `Read` found 6 additional items
- **Verification technique**: `grep -r` for symbols, then read each file to trace full call paths and helper functions

## Revision Log

- 2026-01-21: Created (confidence: 0.85)
