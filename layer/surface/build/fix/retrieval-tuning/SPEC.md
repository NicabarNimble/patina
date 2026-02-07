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
  - measure-the-measurement
---

# fix: Retrieval Tuning — Fusion Quality & Product Metrics

> Continuation of eval-repair Phases 3-4. Now that we can measure (feedback loop,
> NL eval, per-oracle ablation), use the data to improve retrieval quality.

## Problem

### Measurement Correction (2026-02-07)

The original ablation data was inflated by a precision calculation bug: multiple
doc_ids from the same file (e.g. `engine.rs::fn:query`, `engine.rs::fn:new`)
each counted as separate hits, inflating P@K metrics. P@10 could exceed 100%.

The corrected data tells a **different story** from the original analysis:

```
CORRECTED Baseline (25 NL queries, uniform weights, dedup'd):

Pipeline                       P@5     P@10      MRR
unified (all)                27.5%    33.7%    0.408
lexical-only                 24.1%    31.1%    0.417
temporal-only                10.7%    20.5%    0.234
semantic-only                 0.0%     0.0%    0.000
persona-only                  0.0%     0.0%    0.000
belief-only                   8.0%     8.0%    0.094
no-belief                    26.1%    32.7%    0.353

By category:
  knowledge (13)   P@5 17.3%  P@10 19.9%  MRR 0.332
  structural (10)  P@5 32.0%  P@10 41.8%  MRR 0.478
  rationale (2)    P@5 66.7%  P@10 66.7%  MRR 0.600

ORIGINAL INFLATED (for reference — DO NOT USE):
  unified P@10 was 41.1% (actually 33.7%, inflated +7.4pp)
  lexical-only P@10 was 77.2% (actually 31.1%, inflated +46.1pp!)
```

### Phase 1 Baseline (52 NL queries, session 20260207-101812)

Expanded from 25 to 52 queries with train/test split (32/20).

```
EXPANDED Baseline (52 NL queries, uniform weights, train/test split):

Pipeline                       P@5     P@10      MRR
unified (all)                24.6%    33.3%    0.354
lexical-only                 26.0%    34.5%    0.412
temporal-only                 8.3%    21.2%    0.153
semantic-only                 0.0%     0.0%    0.000
persona-only                  0.0%     0.0%    0.000
belief-only                   5.1%     6.4%    0.086
no-belief                    25.5%    32.7%    0.373

By category:
  knowledge (23)   P@5 17.0%  P@10 25.7%  MRR 0.340
  structural (22)  P@5 25.2%  P@10 38.0%  MRR 0.377
  rationale (7)    P@5 42.9%  P@10 52.4%  MRR 0.331

By detected intent:
  General (25)     P@5 23.5%  P@10 30.7%  MRR 0.439
  Temporal (6)     P@5 27.8%  P@10 38.9%  MRR 0.389
  Rationale (7)    P@5 42.9%  P@10 52.4%  MRR 0.331
  Definition (7)   P@5  9.5%  P@10 28.6%  MRR 0.243
  Mechanism (7)    P@5 17.9%  P@10 32.1%  MRR 0.160

By split:
  train (32)       P@5 23.8%  P@10 34.7%  MRR 0.363
  test (20)        P@5 24.2%  P@10 34.2%  MRR 0.341

Intent detection coverage: 52% (27/52 queries get specific intent)
Train-test gap: -0.5pp P@10 (no pre-existing overfit)
```

**Key correction:** Fusion is *slightly beneficial*, not harmful. With 25 queries,
unified (33.7%) edged lexical-only (31.1%). With 52 queries, lexical-only (34.5%)
slightly leads unified (33.3%) by 1.2pp — the expanded query set shifted this
balance, but the difference is within noise. The original claim that "lexical-only
crushes unified" was an artifact of the doc_id double-counting bug.

### What's Actually Wrong (corrected assessment)

**F3: Fusion dilution on co-change is real but NL fusion is fine.**

The co-change subsystem test still shows temporal-only (100%) >> unified (57.5%).
But for NL queries, fusion is net positive. The co-change regression is a
synthetic-query problem, not a product-quality problem.

**F4: Semantic oracle contributes 0% to NL queries.**

With E5-base-v2 (ONNX), semantic search returns nothing useful for natural
language queries. This may be a model limitation, not a fundamental problem.
A better embedding model could change this entirely. Do not hardcode suppression.

**F5: Belief MRR at 0.172 is still low.**

Beliefs are found (80.9% hit rate) but ranked poorly. The single-oracle RRF
disadvantage is real. A score multiplier could help, but needs a held-out test
set before tuning.

**Knowledge queries are the weakest category.**

P@5 17.3%, MRR 0.332 for knowledge queries (13 of 25). Most expected results
are `layer/core/` docs or deep code files that aren't well-served by any oracle.

### Overfitting Risk

The NL eval now has 52 queries (32 train, 20 test). Intent-aware weighting
has 5 weights × 5 intents = 25 tunable parameters. With 32 training queries,
the ratio is ~1.3:1 data points to parameters — still tight, but with the
held-out test set we can now detect overfitting.

**Attempted and reverted (session 20260207-094828):** Suppressed semantic (0.3)
and persona (0.2) for General intent. NL P@10 appeared to improve from 41.1%
to 59.4%, but:
1. The "41.1%" baseline was inflated (really 33.7%)
2. Subsystem tests collapsed (co-change 57.5% → 0.7%, belief MRR 0.172 → 0.014)
3. 25 parameters vs 25 queries = no statistical validity

Weights reverted to uniform baseline. Observations kept as documentation.

## Pre-Requisites (before any tuning)

1. **~~Held-out test set~~** ✅ — 52 queries with train/test split (32/20).
   Train-test gap is -0.5pp P@10, confirming no pre-existing overfit.
2. **Model assessment** — Evaluate whether E5-base-v2 is the right embedding
   model before deciding to suppress semantic. ONNX flexibility means we can
   swap models without code changes.
3. **~~Per-query intent mapping~~** ✅ — Intent detection coverage is 52% (27/52).
   48% of queries fall through to General. Mechanism intent has worst MRR (0.160)
   despite decent P@10 (32.1%) — finds files but ranks them poorly.

## Design

### Phase 1: Expand Eval Coverage ✅

Before tuning anything, fix the measurement:

- ✅ Add 27 new NL queries (52 total) with train/test split (32/20)
- ✅ Map each query to detected intent — 52% coverage, 48% General fallthrough
- ✅ Add intent-breakdown to `eval --nl` output (By Detected Intent section)
- ✅ Add split-breakdown to `eval --nl` output (By Split + Train vs Test sections)

### Phase 2: Intent-Aware Weighting (with proper eval)

Only after Phase 1 provides a held-out test set:

- Tune weights on training set, evaluate on held-out set
- Start with the high-confidence observations (semantic 0%, persona 0%) but
  validate on held-out data before shipping
- Consider whether intent detection needs broadening (most queries → General)

### Phase 3: Belief Score Multiplier (F5)

Compensate for single-oracle RRF disadvantage:

- Need held-out belief queries (not just self-retrieval synthetic test)
- Target: belief MRR improved from current level
- Must not regress co-change or NL precision

### Phase 4: Hub File Suppression

- `src/commands/mod.rs` appears frequently in results
- Validate this is actually a problem with corrected metrics
- If confirmed: dampen scores for high-frequency hub files

### Phase 5: Product Metric Dashboard

Wire session query precision into `patina eval`:

```
$ patina eval --product

Product Metrics (last 10 sessions):
  Session query precision:   34%
  NL retrieval P@5:          28%
  MCP adoption:              12 calls/session
```

## Exit Criteria

### Phase 1: Eval Coverage ✅
- [x] 52 NL queries with train/test split (32 train / 20 test)
- [x] Per-intent metric breakdown in eval --nl output
- [x] Intent detection coverage: 52% specific, 48% General fallthrough

### Phase 2: Intent-Aware Weighting
- [ ] Weights tuned on train set, validated on held-out test set
- [ ] No regression on subsystem tests (co-change, belief self-retrieval)
- [ ] Statistical significance of improvement (not just point estimates)

### Phase 3: Belief Score Multiplier
- [ ] Belief MRR improved with held-out validation
- [ ] Knowledge category MRR improved from 0.332

### Phase 4: Hub File Suppression
- [ ] Hub file problem confirmed with corrected metrics
- [ ] Suppression implemented and validated

### Phase 5: Product Metric
- [ ] Session query precision computed for at least 5 sessions
- [ ] Product metric reported in `patina eval` output

## Observations (not yet actionable)

These are findings from the ablation. They are observations, not tuning decisions.
They now have a held-out test set but still need validation before acting on.

**Confirmed across 52 queries (was 25):**
- Semantic oracle (E5-base-v2 ONNX): still 0% P@K on all 52 NL queries
- Persona oracle: still 0% P@K on all 52 NL queries
- Lexical oracle: best single-oracle MRR (0.412), now slightly leads unified on P@10
  (34.5% vs 33.3%) — the fusion benefit seen with 25 queries is within noise at 52
- Belief oracle: 6.4% P@10 (was 8.0%), low but non-zero
- Temporal oracle: 21.2% P@10, useful but not dominant

**New from Phase 1 expansion:**
- Mechanism intent has worst MRR (0.160) despite decent P@10 (32.1%) — finds relevant
  files but ranks them low. This is a ranking problem, not a recall problem.
- Definition intent has worst P@5 (9.5%) — slow to surface pattern docs.
  Likely needs lexical boost for `layer/core/` content.
- Rationale intent performs best (52.4% P@10) — "why" questions are well-served
  by lexical FTS5 matching commit messages and session notes.
- Intent detection only fires on 52% of queries. The other 48% are keyword-style
  ("belief oracle implementation") that fall to General. Broadening detection
  may help more than weight tuning.
- The ONNX model choice (E5-base-v2) drives semantic results — a different
  model could make semantic the dominant oracle. Don't bake in model assumptions.

## References

- [[eval-repair]] — Phases 1-2 (complete), established measurement infrastructure
- [[belief-retrieval-quality]] — F5 root cause and Fix 1-4 designs
- [[measure-first]] — Prove the problem exists with data
- [[measure-the-measurement]] — Fix the instrument before the observation
- [[error-analysis-over-architecture]] — Categorize failures before adding complexity
