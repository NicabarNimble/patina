---
type: belief
id: grounding-follows-index-rebuilds
persona: architect
facets: [scrape, beliefs, performance]
confidence:
  score: 0.90
entrenchment: high
status: active
extracted: 2026-03-03
references: [scrape-diff-driven]
---

# grounding-follows-index-rebuilds

Belief grounding depends on the vector index, not on code changes.

## Statement

Code file changes do not affect belief grounding results. Grounding only changes when: (a) new beliefs are added, or (b) `patina oxidize` rebuilds the usearch vector index. The correct optimization is to track the index mtime as a watermark, not to intersect changed file paths with grounding evidence paths.

## Evidence

- [[scrape-diff-driven]] Phase 4 — DESIGN.md originally proposed SQL path intersection (`WHERE g.path IN (changed_files)`). Implementation discovered that grounding is computed from vector similarity in the usearch index, not from file paths. The index only changes when oxidize runs.
- [[session-20260303-113842]] — "Key insight: grounding only changes when (a) new beliefs added or (b) usearch index rebuilt by oxidize. Code changes alone don't affect grounding."
- Benchmark: skipping grounding saves ~1.5s per incremental scrape when conditions are met.

## Supports

- [[eventlog-is-truth]] — grounding is a derived view; the usearch index is the source it derives from

## Attacks

- (none)

## Implications

- Future specs should NOT resurrect the SQL path intersection approach for incremental grounding — it's solving the wrong problem.
- Any change to how `patina oxidize` builds the usearch index may affect grounding correctness.
- The watermark approach uses mtime comparison (second granularity). If oxidize ever produces identical-mtime index files, grounding could be stale for one cycle.

## Applied-In

- `src/commands/scrape/beliefs/mod.rs` — `grounding_index_changed()`, `update_grounding_watermark()`
- `layer/surface/build/refactor/scrape-diff-driven/DESIGN.md` — Q3 resolution

## Revision Log

- 2026-03-03: Extracted from [[scrape-diff-driven]] audit session. Reframes Phase 4 approach.
