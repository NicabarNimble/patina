---
type: belief
id: milestones-in-specs
persona: architect
facets: [architecture, versioning, spec-system]
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
extracted: 2026-01-26
revised: 2026-01-26
---

# milestones-in-specs

Milestones belong in specs, not separate files. Git is the source of truth - derive indexes for performance, don't maintain parallel state.

## Statement

Milestones belong in specs, not separate files. Git is the source of truth - derive indexes for performance, don't maintain parallel state.

## Evidence

- session-20260126-060540: Analyzed Option A (frontmatter), B (milestones.toml), C (hybrid). Chose A+scrape - source in spec, derive index. (weight: 0.9)

## Supports

- milestones-immutable: If milestones live in git, they inherit git's append-only nature
- versioning-inference: Spec milestones work for both owned and fork repos

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-01-26: Created (confidence: 0.85)
