---
type: belief
id: delete-dead-code
persona: architect
facets: [code-quality, maintenance, simplicity]
confidence:
  score: 0.85
  signals:
    evidence: 0.90
    source_reliability: 0.85
    recency: 0.95
    survival: 0.50
    user_endorsement: 0.90
entrenchment: medium
status: active
extracted: 2026-01-29
revised: 2026-01-29
---

# delete-dead-code

Dead code should be deleted, not annotated with `#[allow(dead_code)]`. Code either serves a purpose or is removed.

## Statement

Dead code should be deleted, not annotated with `#[allow(dead_code)]`. Code either serves a purpose or is removed.

## Evidence

- session-20260129-074742: Found `#[allow(dead_code)]` on `get_spec_milestones()` function. User explicitly chose "remove it" over "use it for progress display". Deleted 30 lines. (weight: 0.9)
- Aligns with unix-philosophy: one tool, one job. Dead code has no job. (weight: 0.8)
- Aligns with signal-over-noise: dead code is noise in the codebase. (weight: 0.8)

## Supports

- [[signal-over-noise]]: Removing dead code reduces noise
- [[truthful-specs]]: Code should reflect actual state, not aspirational state

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Speculative: "Keep code for future use" — but YAGNI usually wins

## Applied-In

- `src/commands/version/internal.rs`: Removed `get_spec_milestones()` instead of keeping it "in case we need progress tracking"

## Revision Log

- 2026-01-29: Created (confidence: 0.85)
