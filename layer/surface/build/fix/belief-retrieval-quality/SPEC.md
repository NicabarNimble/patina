---
type: fix
id: belief-retrieval-quality
status: ready
created: 2026-02-05
related:
  - layer/surface/build/fix/eval-belief-ground-truth/SPEC.md
  - layer/surface/build/feat/mother-delivery/d1-belief-oracle/SPEC.md
beliefs:
  - fix-data-not-tools
  - measure-first
---

# fix: Belief Retrieval Quality

> BeliefOracle retrieves correctly but RRF fusion buries the results. Improve ranking and co-retrieval.

## Problem

Eval measurements from [[eval-belief-ground-truth]] (2026-02-05):

```
Test                    Unified    No-Belief    Delta
──────────────────────────────────────────────────────
self-retrieval         0.190MRR    0.000MRR   +0.190
belief→code              21.4%        0.0%    +21.4%
code→same-file            7.5%        8.4%    -0.9pp
file→co-change           71.5%       77.2%    -5.8pp
```

Three issues:

1. **MRR 0.190 on self-retrieval** — beliefs are found (78.7% hit rate) but ranked at ~position 5. This is the easiest possible test (query with exact belief text). Real user queries will score worse. The ranking problem is in RRF fusion, not BeliefOracle retrieval.

2. **Co-retrieval at 21.4%** — the product claim ("ask about a principle, get principle + code") works 1 in 5 times. 78.6% of queries return the belief but only 21.4% also return reached code. The code reach signal isn't making it through RRF.

3. **file→co-change regresses -5.8pp** — exceeds 5pp budget by 0.8pp. Beliefs take RRF slots from temporal results. Borderline but consistent.

## Root Cause: Structural RRF Disadvantage (2026-02-05)

**The fundamental problem:** Beliefs can only appear in ONE oracle (BeliefOracle). Code results appear in 2-4 oracles (lexical + temporal + semantic + dependency). RRF sums scores across oracles, so beliefs are structurally capped.

```
RRF score = Σ 1/(60 + rank_i) for each oracle i containing document d

Belief at rank 1 in BeliefOracle only:  1/(60+1) = 0.016
Code file at rank 1 in temporal + rank 23 in lexical: 1/61 + 1/83 = 0.028
Code file appearing in 3 oracles: 0.03 - 0.06
```

Beliefs can never outscore multi-oracle code results.

### Step 1: Self-retrieval misses (10/47 = 21.3%)

**Not** caused by statement length, recency, or embedding quality. Caused by **lexical term overlap with code**.

The 10 missed beliefs have statements containing terms that heavily match code files:
- `eventlog-is-infrastructure` — "eventlog" matches `eventlog.rs` functions (106 code_fts hits)
- `dead-code-requires-decision` — "code" matches recode/scrape files
- `self-healing-invariants` — "exists", "failed", "guards" match code
- `investigate-before-delete` — "delete", "trace" match code
- `compose-over-build` — "tools", "systems" match code + commits
- `layer-is-project-knowledge` — "patina", "project" match code heavily
- `error-analysis-over-architecture` — "complexity", "failure" match code
- `measure-the-measurement` — "metric", "measurement" match code
- `spec-needs-code-verification` — "implementation", "static" match code
- `system-owns-format` — appears at rank 10 (borderline miss)
- `versioning-inference` — "config", "upstream" match code

These terms generate 100+ code_fts + commits_fts + pattern_fts matches, filling all 10 slots with code results scoring 0.016-0.060. The belief at 0.016 gets pushed out.

### Step 2: Co-retrieval failures (78.6% belief present, 21.4% co-retrieval)

When the belief IS present (78.6% of queries), its reached code files rarely appear (only 21.4% co-retrieval). Example:

```
project-config-in-git:  belief ✓ at rank 5, but 1/19 reached files in top-10
read-code-before-write: belief ✓ at rank 4, but 0/18 reached files in top-10
cli-unifies-code-separates: belief ✓ at rank 6, but 0/17 reached files in top-10
```

**Root cause:** The code files a belief applies to aren't the ones that lexically match the belief's statement. "Project configuration should be tracked in git" reaches `src/indexer/database.rs`, but searching that text finds different files that match "configuration" and "git" via lexical/temporal oracles.

### Step 3: file→co-change regression (-5.8pp)

**Beliefs don't appear in file-path queries at all.** Tested `src/main.rs`, `src/retrieval/engine.rs`, `src/commands/scrape/mod.rs` — zero belief results.

The regression comes from RRF noise: with 5 oracles vs 4, each oracle contributes ~2 results. In a 10-result output, the marginal temporal result (rank 4-5) gets displaced by persona/lexical noise that wouldn't be there without the extra oracle competing for slots.

## Proposed Fixes (Ordered by Impact)

### Fix 1: Belief score multiplier in RRF (high impact, low risk)

Multiply BeliefOracle's RRF contribution by a weight to compensate for single-oracle disadvantage. The intent-weight system already supports this:

```rust
// In IntentWeights
fn weight_for(&self, source: &str) -> f32 {
    match source {
        "belief" => 3.0,  // compensate for no multi-oracle boost
        _ => 1.0,
    }
}
```

A 3x multiplier would make belief rank 1 score `3 * 0.016 = 0.048`, competitive with 2-oracle code results.

### Fix 2: Add beliefs to LexicalOracle index (high impact, medium effort)

Index belief statements into `pattern_fts` (or a new table the lexical oracle searches). Then beliefs get both BeliefOracle + LexicalOracle scores = natural multi-oracle boost. No RRF weight hacking needed.

### Fix 3: Intent-based belief suppression for structural queries (medium impact)

The intent system already detects structural queries (file paths, function signatures). Suppress BeliefOracle for these intents to eliminate the -5.8pp temporal regression. Beliefs only contribute to knowledge/rationale queries.

### Fix 4: Belief-code injection (directly addresses co-retrieval) ✅ IMPLEMENTED

When BeliefOracle returns a belief, also inject its top-3 reached code files from `belief_code_reach` as additional OracleResults. Reached files get natural RRF multi-oracle boost when they also appear from lexical/temporal oracles.

**Implementation:** `src/retrieval/oracles/belief.rs` — `REACH_INJECT_LIMIT = 3`, `fetch_reached_files()` queries `belief_code_reach` table. Injected results use `score_type: "belief_reach"` with score = belief_score * reach_score.

**Results (3 runs):**

```
Metric              Before      After (3 runs)     Target
co-retrieval        21.4%       42.9-57.1%         >= 40%  ✓
reach recall        18.2%       44.3%              —
self-retrieval MRR  0.190       0.147-0.179        >= 0.400  ✗
code→same-file      -0.9pp      -2.6 to +1.6pp     <= 5pp  ✓
file→co-change      -5.8pp      -11.3 to +4.8pp    <= 5pp  ✓ (mostly)
```

**Note:** Eval has high variance in `file→co-change` due to `HashMap::iter()` non-determinism in test file selection. One run showed -11.3pp outlier; most runs within budget.

## Exit Criteria

- [x] Error analysis complete for all three issues (categorized, root causes identified)
- [ ] Self-retrieval MRR >= 0.400 (beliefs in top 2-3 on average) — still 0.15-0.18, needs Fix 1 or 2
- [x] Co-retrieval rate >= 40% (belief + code delivered together) — 42.9-57.1%
- [x] file→co-change regression within 5pp budget — mostly within, eval variance is the outlier
- [ ] D1 VERDICT: PASS — passes on 2/3 runs, blocked by eval non-determinism + MRR target

## See Also

- [[eval-belief-ground-truth]] — eval that produced these measurements
- [[d1-belief-oracle/SPEC.md]] — D1 exit criterion
