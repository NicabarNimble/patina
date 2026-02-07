---
type: fix
id: retrieval-tuning
status: draft
created: 2026-02-07
sessions:
  origin: 20260207-093335
blocked_by: []
blocks: []
related:
  - layer/surface/build/fix/eval-repair/SPEC.md
beliefs:
  - measure-first
  - error-analysis-over-architecture
---

# fix: Retrieval Tuning — Fusion Quality & Product Metrics

> Continuation of eval-repair Phases 3-4. Now that we can measure (feedback loop,
> NL eval, per-oracle ablation), use the data to improve retrieval quality.

## Problem

The eval-repair ablation reveals structural issues in the retrieval pipeline:

### F3: Fusion Dilutes the Best Oracle

temporal-only achieves 100% P@10 on co-change queries. Adding other oracles via
RRF **reduces** this to 57.5% — a 42pp regression. For NL queries, lexical-only
(77.2% P@10) outperforms unified (41.1%). Fusion is making the best oracle worse.

### F5: Belief MRR Stuck at 0.172

Beliefs are found (80.9% hit rate) but ranked at position ~6. Single-oracle RRF
disadvantage: belief results max at 0.016 score while multi-oracle results score
0.028-0.060. Score multiplier fix designed but not built.

### Hub File Pollution

`src/commands/mod.rs` appears in 14/20 NL query results as a hub file. High
co-change count makes it a universal "answer" that dilutes precision.

### Belief Oracle Over-Reach

Beliefs appear in structural queries where they add noise, not signal. The
no-belief configuration slightly outperforms unified on NL queries (+3.0pp P@10).

## Baseline (Regression Target)

From eval-repair Phase 2 (25 curated NL queries):

```
Pipeline                       P@5     P@10      MRR
unified (all)                30.8%    41.1%    0.412
lexical-only                 37.8%    77.2%    0.416
temporal-only                12.0%    22.8%    0.239
semantic-only                 0.0%     0.0%    0.000
persona-only                  0.0%     0.0%    0.000
belief-only                   8.0%     8.0%    0.094
no-belief                    29.1%    44.1%    0.348

By category:
  knowledge (13)   P@5 17.3%  P@10 19.9%  MRR 0.332
  structural (10)  P@5 41.2%  P@10 58.2%  MRR 0.478
  rationale (2)    P@5 66.7%  P@10 83.3%  MRR 0.600
```

Any change must not regress unified P@10 below 41.1% or MRR below 0.412.

## Design

### Phase 1: Intent-Aware Oracle Weighting (F3)

The intent classification system exists. Extend it with oracle-specific weights:

- **Structural queries** (file paths, function names): boost temporal + lexical, suppress belief
- **Knowledge queries** (how, why, explain): boost belief + semantic, suppress temporal
- Measure: unified P@10 on NL queries should improve toward lexical-only (77.2%)

### Phase 2: Belief Score Multiplier (F5)

Compensate for single-oracle RRF disadvantage:

- `belief_weight = 3.0` in IntentWeights for knowledge intents
- Low effort, directly addresses MRR 0.172 → target 0.300+
- Measure: belief self-retrieval MRR, NL knowledge category MRR

### Phase 3: Hub File Suppression

Penalize hub files that appear in too many results:

- Track per-file retrieval frequency
- Dampen scores for files appearing in >50% of queries
- Target: `src/commands/mod.rs` drops out of top-10 for most NL queries

### Phase 4: Product Metric Dashboard

Wire session query precision into `patina eval`:

```
$ patina eval --product

Product Metrics (last 10 sessions):
  Session query precision:   34%
  NL retrieval P@5:          45%
  MCP adoption:              12 calls/session
```

## Exit Criteria

### Phase 1: Intent-Aware Weighting
- [ ] Intent system extended with per-oracle weight multipliers
- [ ] Fusion delta on co-change: unified within 10pp of temporal-only
- [ ] NL P@10 improved from 41.1% toward 60%+

### Phase 2: Belief Score Multiplier
- [ ] Belief MRR improved from 0.172 to >= 0.300
- [ ] Knowledge category MRR improved from 0.332

### Phase 3: Hub File Suppression
- [ ] Hub file detection implemented
- [ ] NL queries no longer dominated by `src/commands/mod.rs`

### Phase 4: Product Metric
- [ ] Session query precision computed for at least 5 sessions
- [ ] Product metric reported in `patina eval` output

## References

- [[eval-repair]] — Phases 1-2 (complete), established measurement infrastructure
- [[belief-retrieval-quality]] — F5 root cause and Fix 1-4 designs
- [[measure-first]] — Prove the problem exists with data
- [[error-analysis-over-architecture]] — Categorize failures before adding complexity
