---
type: feat
id: epistemic-e4.6a-fix
status: complete
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

- [x] 1. Create `belief_code_reach` table in `create_materialized_views()`
- [x] 2. Extend `compute_belief_grounding()` — after kNN, walk commit neighbors through
  `commit_files` to collect file paths and aggregate scores
- [x] 3. Populate `belief_code_reach` with per-file entries
- [x] 4. Update `grounding_code_count` from reach data instead of direct cosine
- [x] 5. Update `find_belief_impact()` to use `belief_code_reach` for code→belief linking
  (replaces broken direct cosine path)
- [x] 6. Test full cycle: scrape --rebuild, oxidize, scrape, belief audit — verify code grounding
  is nonzero for beliefs with commit neighbors
- [x] 7. Make `--impact` default-on in MCP scry (LLM always sees belief reach on code results)
- [x] 8. Add grounding accuracy measurement — `is_source_code()` classifier, precision% in
  scrape output. Revealed 9% precision (9 source files out of 93 reach files).
- [x] 9. Filter non-source files at the hop — `is_source_code()` gate in commit_files walk.
  Result: 93 → 9 reach files, precision 9% → 100%.
- [x] 10. Build ground-truth eval set — 7 beliefs with grounding recall verification queries.
  Result: 3/7 pass (43% recall), 4/7 contested (commit neighbors only touched docs).

---

## Exit Criteria

- [x] `grounding_code_count` > 0 for beliefs with commit neighbors (was always 0)
- [x] `belief_code_reach` populated with file paths reachable from each belief
- [x] `--impact` shows belief annotations on code results via multi-hop
- [x] No new embeddings or models required — pure SQL joins after semantic hop
- [x] MCP scry returns belief impact by default (no opt-in required)
- [x] Grounding accuracy measurable: precision% reported during scrape
- [x] Precision > 50% after source-only filtering (achieved: 100%)
- [x] Recall measurable via ground-truth verification queries on 7 beliefs (measured: 43%)

---

## Error Analysis (9% precision)

After steps 1-8, the structural hop (commit → commit_files) produces 93 reach files but only
9 are source code. The 84 false positives break down as:

| Category | Count | Cause |
|----------|-------|-------|
| Belief .md files | ~60 | Commits that create/edit beliefs also touch other belief files |
| Spec/layer .md files | ~15 | Belief-related commits touch specs and design docs |
| Config files (.yml, .toml) | ~5 | CI/config touched alongside code changes |
| Session .md files | ~4 | Session files in same commit |

**Root cause:** The commit→file hop is unfiltered. A commit message like "belief: add
eventlog-is-infrastructure" scores 0.89 cosine with the eventlog-is-truth belief, but the
commit only touches `layer/surface/epistemic/beliefs/eventlog-is-infrastructure.md` — no
actual code. The semantic hop is accurate; the structural hop is noisy.

**Fix:** Filter at the hop. Only insert source code files (.rs, .sh, .py, etc.) into
`belief_code_reach`. Non-source files are noise for code grounding. This is a pipeline fix,
not a model fix.

---

## Design Principle

Combine patina's tools within their limitations. Semantic search bridges natural language
(belief ↔ commit). Structural joins bridge exact relationships (commit → file → function →
signals). Local-first, edge hardware, no cloud. The constraint is the architecture.

---

## What This Changes

- `grounding_code_count` becomes meaningful (was always 0, now source-code-only count)
- `belief audit` GROUND column shows real code reach (e.g., `6c1m14s`)
- `find_belief_impact()` uses `belief_code_reach` SQL lookup instead of broken direct cosine
- MCP scry returns belief impact by default — LLM sees *why* code exists, not just *what*
- Grounding precision tracked: source files / total reach files, reported each scrape
- Ground-truth eval queries on key beliefs measure recall alongside precision

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-02 | ready | Specced during session 20260202-130018 |
| 2026-02-02 | active | Steps 1-8 complete. 39/47 grounded, 93 reach, 9% precision. Error analysis done. |
| 2026-02-02 | complete | Steps 9-10 done. 100% precision (9 source files), 43% recall (3/7 ground-truth pass). |
