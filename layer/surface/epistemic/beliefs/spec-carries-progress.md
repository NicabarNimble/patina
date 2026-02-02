---
type: belief
id: spec-carries-progress
persona: architect
facets: [process, spec-system, sessions]
confidence:
  score: 0.85
entrenchment: medium
status: active
extracted: 2026-01-30
revised: 2026-01-30
---

# spec-carries-progress

Specs must have checkboxed build steps that track incremental progress toward exit criteria. Without them, picking up work across sessions requires reading git log instead of opening one file.

## Statement

Specs must have checkboxed build steps that track incremental progress toward exit criteria. Without them, picking up work across sessions requires reading git log instead of opening one file.

## Evidence

- session-20260130-131543: 0.9.2 spec had exit criteria and a migration path but no way to track which build steps were done. Added 16 numbered checkboxes across 3 phases. Any future session opens the spec and sees where to pick up. (weight: 0.9)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-01-30: Created (confidence: 0.85)
