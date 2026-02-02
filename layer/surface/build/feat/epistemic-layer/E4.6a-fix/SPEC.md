---
type: feat
id: epistemic-e4.6a-fix
status: ready
created: 2026-02-02
updated: 2026-02-02
sessions:
  origin: 20260202-130018
related:
  - layer/surface/build/feat/epistemic-layer/SPEC.md
  - layer/surface/build/feat/epistemic-layer/design.md
---

# feat: Multi-Hop Code Grounding (E4.6a-fix)

> Beliefs can't reach code via direct cosine similarity — bridge the gap through commits.

**Parent:** [epistemic-layer](../SPEC.md) phase E4.6
**Prerequisite:** E4.6a complete. Discovery from hardening session 20260202-130018.

---

## Problem

E4.6a assumed direct cosine similarity between belief embeddings and code function
embeddings would work at the 0.85 threshold. It doesn't. After full rebuild + oxidize + scrape,
`grounding_code_count` is **zero for all 47 beliefs**. Code function representations are terse
signatures ("Function `insert_event` in `./src/eventlog.rs`, params: conn, event_type...") while
beliefs are natural language claims ("The eventlog is shared infrastructure"). These come from
different text distributions — the embedding model (e5-base-v2) projects them to the same 256-dim
space but cosine similarity never reaches 0.85 across that gap.

This is not a threshold problem. Lowering the threshold would let noise in. It's a distribution
mismatch: **same-type similarity works, cross-type similarity doesn't.**

| Pair Type | Cosine Score | Works? |
|-----------|-------------|--------|
| Belief ↔ belief | 0.87-0.93 | Yes — same distribution (natural language claims) |
| Belief ↔ commit | 0.85-0.91 | Yes — both natural language about intent |
| Belief ↔ session | 0.85-0.91 | Yes — both natural language about work |
| Belief ↔ code | < 0.85 | **No** — signatures vs. claims, different distributions |

---

## Solution: Multi-Hop Grounding

Use semantic hops where they work (belief → commit) combined with structural hops where
they're exact (commit → files → functions). Each tool operates within its strength.
No new embeddings needed.

**Hop chain:**

```
belief
  │ semantic (cosine ≥ 0.85, already works)
  ▼
commit(s)
  │ structural (commit_files table, exact)
  ▼
file_path(s)
  │ structural (function_facts table, exact)
  ▼
function(s)
  │ structural (module_signals table, exact)
  ▼
signals (importer_count, activity_level, is_entry_point)
```

**Confidence:** Product of hop scores. If belief→commit scores 0.89, that's the confidence for
all files in that commit. Files touched by multiple belief-adjacent commits get higher aggregate
confidence. A file touched by 3 commits at 0.87, 0.89, 0.91 is more confidently grounded than
one touched by a single commit at 0.91.

---

## Storage

New table `belief_code_reach` computed during scrape:

```sql
CREATE TABLE IF NOT EXISTS belief_code_reach (
    belief_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    reach_score REAL,          -- aggregate confidence across hops
    commit_count INTEGER,      -- how many belief-adjacent commits touch this file
    function_count INTEGER,    -- functions in this file
    hop_path TEXT,             -- e.g., "commit:abc1234,commit:def5678"
    PRIMARY KEY (belief_id, file_path)
);
```

---

## Implementation

Extend `compute_belief_grounding()` in `src/commands/scrape/beliefs/mod.rs`.
After the existing kNN search that already finds commit neighbors:

1. For each belief, collect commit SHAs from neighbors in COMMIT_ID_OFFSET range (already found)
2. Look up those SHAs in `commit_files` → collect file_paths
3. Aggregate: count commits per file, compute reach_score as max(commit_scores) for that file
4. Look up function_count from `function_facts` per file
5. INSERT INTO `belief_code_reach`
6. Update `grounding_code_count` on beliefs table from reach data (count of files with reach > 0)

---

## Build Steps

- [ ] 1. Create `belief_code_reach` table in `create_materialized_views()`
- [ ] 2. Extend `compute_belief_grounding()` — after kNN, walk commit neighbors through
  `commit_files` to collect file paths and aggregate scores
- [ ] 3. Populate `belief_code_reach` with per-file entries
- [ ] 4. Update `grounding_code_count` from reach data instead of direct cosine
- [ ] 5. Update `find_belief_impact()` to use `belief_code_reach` for code→belief linking
  (replaces broken direct cosine path)
- [ ] 6. Test full cycle: scrape --rebuild, oxidize, scrape, belief audit — verify code grounding
  is nonzero for beliefs with commit neighbors

---

## Exit Criteria

- [ ] `grounding_code_count` > 0 for beliefs with commit neighbors (currently always 0)
- [ ] `belief_code_reach` populated with file paths reachable from each belief
- [ ] `--impact` shows belief annotations on code results via multi-hop
- [ ] No new embeddings or models required — pure SQL joins after semantic hop

---

## Design Principle

Combine patina's tools within their limitations. Semantic search bridges natural language
(belief ↔ commit). Structural joins bridge exact relationships (commit → file → function →
signals). Local-first, edge hardware, no cloud. The constraint is the architecture.

---

## What This Changes

- `grounding_code_count` becomes meaningful (currently always 0)
- `belief audit` GROUND column shows real code reach
- `--impact` can use `belief_code_reach` instead of direct cosine for code→belief linking
- Future: `scry --impact` checks which beliefs reach a file when that file appears in results

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-02 | ready | Specced during session 20260202-130018 |
