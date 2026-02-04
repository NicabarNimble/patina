---
type: fix
id: eval-belief-ground-truth
status: implementation
created: 2026-02-04
related:
  - layer/surface/build/feat/mother-delivery/d1-belief-oracle/SPEC.md
  - layer/surface/build/feat/mother-delivery/d0-unified-search/SPEC.md
beliefs:
  - verification-is-annotation-not-retrieval
---

# fix: Eval Belief Ground Truth

> The eval only tests queries where beliefs can never be correct answers, then penalizes them for showing up.

## Problem

D1 exit criterion: "Re-run task-oriented A/B eval. Target: delta >= 0.0 (beliefs no longer hurt)."

Current eval methodology (`patina eval`) measures:
- **code→same-file**: function description → other functions in same file
- **file→co-change**: file path → co-change partners

Both tests have code-only ground truth. Beliefs can never be a correct answer. Every belief result in the top-K is a false positive by definition. The eval **structurally guarantees** a negative delta for beliefs.

D1 belief delta measured 2026-02-04:
```
Test                Unified    No-Belief    Delta
code→same-file        4.0%        7.6%     -3.6%  ✗
file→co-change       74.3%       79.9%     -5.7%  ✗
```

But in practice, `patina scry "why should we commit often"` returns `belief:commit-early-commit-often` at position 2 — exactly the right answer. The eval can't see this because it has no knowledge-query ground truth.

## Root Cause

The eval tests one retrieval mode (structural locality) with one answer type (code). Beliefs serve a different mode (knowledge retrieval) with a different answer type (project decisions). Missing eval = missing signal.

## Design

Add two eval sections that test what beliefs actually do:

### Test 1: Belief Self-Retrieval

Query with each belief's statement. The belief itself should appear in results.

```
Ground truth: beliefs table
Query:        belief.statement (e.g., "Make small focused commits frequently")
Expected:     doc_id "belief:<id>" in top-K results
Metric:       Mean Reciprocal Rank (MRR) — average of 1/rank across all beliefs
              Hit rate — fraction of beliefs found in top-K at all
```

This is the simplest possible test: can the system find its own beliefs? MRR rewards higher ranking — a belief at position 1 scores 1.0, at position 5 scores 0.2.

Run through both `unified` and `no-belief` engines. The delta measures whether BeliefOracle improves belief retrieval (it should, dramatically).

### Test 2: Belief-Code Co-Retrieval

For beliefs with `belief_code_reach` entries: query with the belief statement, check if both the belief AND its reached code files appear. Two separate signals:

**Belief-present@K (binary):** Is `belief:<id>` in the top-K results?

**Reach-hit@K (normalized recall):** Among the belief's reached files, how many appear in top-K? Normalized by `min(K, reach_count)` so beliefs reaching 3 files and beliefs reaching 19 files are comparable.

**Success criterion:** `belief_present AND reach_hit >= 1` — the system delivered both "why" (the belief) and at least one "what" (code it applies to). This matches the product claim.

```
Ground truth: beliefs table + belief_code_reach table
Query:        belief.statement
Metrics:
  belief_present_rate  — fraction of queries where belief:<id> in top-K
  reach_recall@K       — avg(reached files in top-K / min(K, reach_count))
  co_retrieval_rate    — fraction where belief present AND ≥1 reached file
```

Run through `unified` and `no-belief`. The unified pipeline should beat no-belief on all three: no-belief has no BeliefOracle, so belief_present will be near-zero.

### Test 3: Existing Tests (unchanged)

code→same-file and file→co-change remain. They measure structural retrieval.

### Structural Regression Budget

Beliefs taking RRF slots from code results is an expected cost, not a bug. But "slightly negative" must be a number.

**Budget: structural tests may regress up to 5pp each.** The measured deltas (-3.6pp and -5.7pp) are near this boundary. If either exceeds 5pp consistently, the BeliefOracle needs tuning (score thresholds, result count limits, or intent-based suppression).

Rationale: 5pp is ~1 result in top-20 shifting from code to belief. For a unified pipeline serving both code and knowledge queries, losing 1 code slot per query is an acceptable trade if knowledge queries gain substantially.

### Summary Table Update

```
━━━ D1 Belief Delta ━━━

Test                    Unified    No-Belief    Delta    Budget   Verdict
──────────────────────────────────────────────────────────────────────────
belief self-retrieval      MRR        MRR       +X.X       —      (positive = pass)
belief→code co-retr.     rate       rate       +X.X       —      (positive = pass)
code→same-file           P@10       P@10       -X.X     ≤5pp     (within budget = pass)
file→co-change           P@10       P@10       -X.X     ≤5pp     (within budget = pass)

D1 PASS if: knowledge deltas positive AND structural regression within budget
```

## Implementation

Changes to `src/commands/eval/mod.rs` only:

1. `eval_belief_self_retrieval(conn, engine, engine_name)` — query each belief's statement, check for `belief:<id>` in results, compute MRR + hit rate
2. `eval_belief_code_co_retrieval(conn, engine, engine_name)` — for beliefs with code reach, query statement, report belief_present_rate + reach_recall@K + co_retrieval_rate
3. Wire into `execute()` alongside existing tests, include in summary table and D1 delta section
4. D1 verdict uses structural regression budget (5pp) for code tests

Ground truth loads from existing tables — no new tables, no scrape changes.

### Future: Curated Knowledge Queries

After this fix lands, the next eval improvement is a small hand-labeled query set (`eval_knowledge_queries.jsonl`): real questions like "why should we commit often" mapped to expected belief IDs and code paths. This tests "does the system help users" vs "does it retrieve its own rows." Deferred — self-retrieval unblocks D1 measurement now.

## Exit Criteria

- [ ] Belief self-retrieval eval runs for all 47 beliefs, reports MRR + hit rate for unified vs no-belief
- [ ] Belief-code co-retrieval reports belief_present_rate, reach_recall@K, co_retrieval_rate
- [ ] D1 delta section includes all 4 test types with budget enforcement
- [ ] D1 measurement has positive knowledge-query deltas and structural regression within 5pp budget

## See Also

- [[d1-belief-oracle/SPEC.md]] — D1 exit criterion that this fix unblocks
- [[d0-unified-search/SPEC.md]] — Eval design section (structural tests)
