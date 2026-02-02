---
type: belief
id: specs-source-of-truth
persona: architect
facets: [architecture, workflow, specs]
confidence:
  score: 0.85
entrenchment: medium
status: active
extracted: 2026-01-27
revised: 2026-01-27
---

# specs-source-of-truth

Specs are the single source of truth; all external systems (GitHub issues, project boards, releases) are read-only projections that derive from spec state, never the reverse.

## Statement

Specs are the single source of truth; all external systems (GitHub issues, project boards, releases) are read-only projections that derive from spec state, never the reverse.

## Evidence

- session-20260127-085434: Version commands, forge sync, project boards all traced back to specs as origin. GitHub is visibility layer, not control plane. (weight: 0.9)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-01-27: Created (confidence: 0.85)
