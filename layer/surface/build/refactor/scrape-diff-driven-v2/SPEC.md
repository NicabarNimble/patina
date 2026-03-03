---
type: refactor
id: scrape-diff-driven-v2
status: draft
created: 2026-03-03
target: '3'
blocked_by:
- knowledge-system-architecture
related:
- scrape-diff-driven
beliefs:
- grounding-follows-index-rebuilds
- incremental-maintenance-requires-stable-ids
exit_criteria: []
split_from: scrape-diff-driven
---

# scrape-diff-driven-v2

EC7 (mother-scrape-dispatch): Hook sends diff event to Mother over UDS; Mother dispatches to warm pipeline plugins. Requires KSA Phase 1 to expand Mother's plugin hosting beyond mother-child world. Also includes follow-up items from audit: rename cleanup gap (old file FTS5 entries persist until full rebuild), PipelineEngine linker duplication refactor (new Linker per plugin entry is unnecessary overhead at scale).

## Recovery

Parent spec content: `git show spec/scrape-diff-driven-v1-complete:layer/surface/build/refactor/scrape-diff-driven/SPEC.md`
