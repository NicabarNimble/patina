---
type: belief
id: semantic-value-requires-vocabulary-gap
persona: architect
facets: [retrieval, semantics, evaluation]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-09
revised: 2026-02-09
---

# semantic-value-requires-vocabulary-gap

Semantic search adds value over keywords when vocabulary diverges between query and content; for descriptive natural language with consistent terminology, FTS5 keyword search is more effective

## Statement

Semantic search adds value over keywords when vocabulary diverges between query and content; for descriptive natural language with consistent terminology, FTS5 keyword search is more effective

## Evidence

- [[session-20260208-235517]]: FTS5 simulation: 9/15 (60%) vs scry 5/15 (33%) on session queries. Scry-only hits (Q3, Q15) both involve vocabulary gaps; FTS5-only hits (6) all have direct keyword overlap (weight: 0.95)
- [[684a578d]]: Q3 "credential access containers" → semantic found "Host-based credential fetch, Touch ID, inject into container tmpfs" (vocabulary gap bridged); FTS5 found wrong session with surface keyword matches (weight: 0.9)
- [[684a578d]]: Q15 "automatic repository creation" → semantic found "opinionated defaults: auto-create private GitHub repo" (vocabulary gap bridged); FTS5 found "just works experience, no prompts" in wrong session (weight: 0.9)

## Supports

- [[corpus-composition-over-model]]: Confirms corpus characteristics (keyword-friendly vs vocabulary-diverse) matter more than model choice for retrieval effectiveness

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[semantic-structural-split]] Phase 5b: FTS5 simulation determined `eventlog_fts` in assay is higher ROI than session-semantic tuning for session content
- `resources/eval/session-fts5-simulation.sql`: Reproducible evidence artifact

## Revision Log

- 2026-02-09: Created — metrics computed by `patina scrape`
