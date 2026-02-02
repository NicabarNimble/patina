---
type: belief
id: repo-add-complete-result
persona: architect
facets: [unix-philosophy, repo-command, user-experience]
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
extracted: 2026-01-25
revised: 2026-01-25
---

# repo-add-complete-result

Reference repos should get full semantic search by default - one command, complete result. Warn on failure, don't block.

## Statement

Reference repos should get full semantic search by default - one command, complete result. Warn on failure, don't block.

## Evidence

- session-20260125-141105: Discovered repo add only ran scrape, leaving repos without semantic search. Fixed by wiring oxidize into repo add. (weight: 0.9)

## Verification

```verify type="sql" label="oxidize_repo called from repo command" expect=">= 1"
SELECT COUNT(*) FROM call_graph WHERE callee LIKE '%oxidize\_repo%' ESCAPE '\' AND file LIKE '%repo/%'
```

## Supports

- [[layer/core/unix-philosophy]]: One tool, one job, done well

## Attacks

<!-- None identified -->

## Attacked-By

- Performance concern: Oxidize adds ~1-2 minutes to repo add for large codebases

## Applied-In

- `patina repo add` now runs oxidize by default (commit 9a3bcd5d)
- `--no-oxidize` flag available for speed when semantic search not needed

## Revision Log

- 2026-01-25: Created (confidence: 0.85)
