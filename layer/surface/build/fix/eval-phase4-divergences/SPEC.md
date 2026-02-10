---
type: fix
id: eval-phase4-divergences
status: complete
created: 2026-02-09
sessions:
  origin: 20260208-224211
  discovered: 20260208-171005
related:
- layer/surface/build/refactor/semantic-structural-split/SPEC.md
beliefs:
- spec-driven-design
- never-tune-on-eval
- andrew-ng-over-shoulder
- dependable-rust
- unix-philosophy
---

# fix: Eval Phase 4 Divergences — Governance Cleanup

> Phase 4 eval shipped with three divergences from the SPEC plan text.
> All three were reasonable engineering decisions but bypassed the governance
> chain: session → code, not session → SPEC → code. This fix SPEC restores
> the chain per [[spec-driven-design]].

## Problem

Session [[20260208-171005]] implemented Phase 4 eval with three divergences
from the SPEC plan (lines 249-264 of semantic-structural-split/SPEC.md).
The code works correctly and baselines are documented, but the SPEC plan
text still describes the original design, not what was built. Per
[[spec-driven-design]] Rule 2: "When a SPEC is wrong or incomplete, the
fix is a new linked spec — not a silent divergence."

## Divergences

### 1. Query Set Replacement (SPEC line 251)

**SPEC said:** "Uses existing 52 NL queries (these are mostly factual queries)"

**Reality:** Created new 25-query assay set and 20-query scry set. Old
52-query `nl-queries.json` preserved but not used by new eval modes.

**Root cause:** 8 of 52 queries reference files deleted during Phase 1:
- `src/retrieval/oracles/lexical.rs` (deleted)
- `src/retrieval/oracles/temporal.rs` (deleted)
- `src/retrieval/oracles/persona.rs` (deleted)
- `src/retrieval/oracles/belief.rs` (deleted)
- `src/retrieval/intent.rs` (deleted)

Affected queries: "belief oracle implementation", "belief scrape population
storage retrieval", "what oracles are available in the retrieval system",
"when was the belief system introduced", "how does intent detection work
for queries", "how does the persona oracle work", "how does the temporal
oracle implement co-change ranking", "lexical oracle full-text search".

**[[never-tune-on-eval]] conflict resolved:** The SPEC said "use existing
52 queries" but the belief says "design queries for new architecture."
These conflict. Resolution: the SPEC was written before Phase 1 deleted
5 oracle files, invalidating 8 queries. Patching stale paths is itself a
form of eval contamination — adjusting the eval to match the system rather
than testing whether the system serves user needs. The belief is correct:
fresh queries designed for the post-split architecture are more honest.
The SPEC plan text was wrong about the 52 queries surviving the split.

**Authorization:** New query sets authorized. Old `nl-queries.json` retained
for historical reference but not used by `--assay`, `--scry`, or `--combined`
eval modes. The old `--nl` eval mode continues to use the 52-query set for
backward compatibility with pre-split measurements.

### 2. FTS5-Only Assay Eval (SPEC line 250)

**SPEC said:** "FTS5 + temporal"

**Reality:** `assay_eval.rs` calls `assay_search()` which performs FTS5
ranked search only. Co-change temporal analysis is not tested.

**Root cause:** Co-change is a structurally different query type. It takes
a file path as input (`query_co_changes(conn, file_path, limit)`) and
returns neighboring files ranked by co-change frequency. It answers "what
files change together with X?" — not "find files matching this NL
description." NL eval methodology (query → expected results → P@K)
doesn't apply to co-change queries.

Per [[unix-philosophy]]: one eval, one job. Assay eval tests factual NL
search (FTS5). Co-change eval, if needed, would be a separate eval mode
with file-path inputs and neighbor-file expected results — a different
test design entirely.

**Authorization:** Assay eval tests FTS5 ranked search only. Co-change
temporal analysis is a separate query type with different input/output
shape. A co-change eval mode could be added in the future as independent
work, not as part of the assay NL eval.

### 3. Simpler Combined Eval (SPEC lines 262-264)

**SPEC said:** "Given a development context, does the system surface the
right context?" — described as "the product metric."

**Reality:** `combined_eval.rs` runs both query sets through both systems
with facts-first interleaving and HashSet deduplication. It measures how
assay and scry complement each other (cross-system contribution, combined
P@K, delta from single-system baselines). Simpler than a context-level
eval that would take development scenarios as input.

**Root cause:** A richer "development context" eval requires defining what
a "development context" is (file being edited? current task? session
history?) and what "right context" means (useful for the LLM? reduces
hallucination? improves code quality?). This is Phase 5 product metric
territory — it requires the eval infrastructure Phase 4 builds, but goes
beyond measuring retrieval accuracy.

Per [[andrew-ng-over-shoulder]]: measure what you can measure honestly.
The current combined eval honestly measures pipeline behavior. The richer
product metric requires more design work.

**Authorization:** Current combined eval accepted as Phase 4 implementation.
The richer "development context → right context" product metric is Phase 5
territory and should be designed as a separate SPEC when the eval
infrastructure matures.

## Exit Criteria

- [x] Fix SPEC created documenting all three divergences
- [x] Each divergence has root cause, resolution, and authorization
- [x] [[never-tune-on-eval]] vs SPEC conflict resolved explicitly
- [x] Original SPEC Phase 4 plan text updated to match reality
- [x] Governance chain restored: session → fix SPEC → original SPEC update

## References

- [[semantic-structural-split]] — parent SPEC with Phase 4 plan text
- [[spec-driven-design]] — governance pattern requiring this fix
- [[never-tune-on-eval]] — belief in tension with original SPEC text
- [[20260208-171005]] — session where divergences occurred
- [[20260208-224211]] — session where fix SPEC was created
