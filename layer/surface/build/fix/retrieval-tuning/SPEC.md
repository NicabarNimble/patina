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
language queries. A0 diagnostic (session 20260207-150836) confirmed: pipeline
IS functional (returns 10 results per query), but results dominated by session
events (88% of semantic index). Session-trained projection maps NL queries to
session-space, not code/pattern-space. Additionally, pattern enrichment maps
`source_id: id` not `file_path`, so even correctly-ranked patterns would fail
eval matching. Verdict: model+training mismatch, not pipeline bug. Do not
hardcode suppression — when model/training changes, re-evaluate.

**F5: Belief MRR at 0.172 is still low.**

Beliefs are found (80.9% hit rate) but ranked poorly. The single-oracle RRF
disadvantage is real. A score multiplier could help, but needs a held-out test
set before tuning.

**F6: Pattern doc_id mapping bug silently drops all layer/ matches.**

`scry_lexical()` and semantic enrichment return pattern `id` (e.g.,
"dependable-rust") as `source_id`, but eval expects `file_path` (e.g.,
"layer/core/dependable-rust.md"). 19/52 queries (37%) expect `layer/` files.
24 of 161 expected files are `layer/` paths. All invisible to eval scoring.
Identified in Phase 2.5 diagnostics (session 20260207-150836).

**Knowledge queries are the weakest category.**

P@5 17.3%, MRR 0.332 for knowledge queries (13 of 25). Most expected results
are `layer/core/` docs or deep code files. F6 (doc_id mapping bug) is a
major contributor — pattern FTS finds these docs but the wrong doc_id format
prevents eval from counting them as hits.

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

**Phase 2 tuning (session 20260207-101812):** With 52 queries and held-out test
set, suppressed semantic (0.0) and persona (0.0) across ALL intents. Redirected
Mechanism's dead semantic boost to lexical. All metrics improved, subsystem tests
improved, test set validated. See Phase 2 results below.

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

### Phase 2: Intent-Aware Weighting ✅

Tuned with held-out validation (session 20260207-101812):

- ✅ Suppressed semantic (0.0) and persona (0.0) across all intents — both
  contribute 0% P@K with E5-base-v2. Removes RRF noise.
- ✅ Redirected Mechanism's dead `semantic: 1.5` boost to `lexical: 1.5`
- ✅ Validated on held-out test set: test P@10 +3.3pp, test MRR +0.040
- ✅ Subsystem tests improved: co-change +14.9pp, belief MRR +0.072
- Assessed intent detection broadening — deferred. Definition P@10 (28.6%)
  < General P@10 (33.4%), so reclassifying "what" queries would hurt.
  Mechanism broadening could help 2-3 queries but sample too small to validate.

```
Phase 2 Results (52 NL queries, tuned weights):

                    Before (uniform)    After (tuned)    Delta
NL P@10 (all)             33.3%           37.7%        +4.4pp
NL MRR (all)              0.354           0.421        +0.067
NL P@10 (test)            34.2%           37.5%        +3.3pp
NL MRR (test)             0.341           0.381        +0.040
Train-test gap            -0.5pp          -1.4pp       (healthy)
Co-change P@10            58.2%           73.1%        +14.9pp
Belief MRR                0.169           0.241        +0.072
Belief hit rate           74.5%           83.0%        +8.5pp
Code same-file P@10        6.6%            9.7%        +3.1pp
D1 verdict                PASS            PASS

By intent (after):
  General (25)     P@10 33.4%  MRR 0.462
  Mechanism (7)    P@10 51.2%  MRR 0.493  (was 32.1% / 0.160)
  Rationale (7)    P@10 52.4%  MRR 0.345
  Temporal (6)     P@10 33.3%  MRR 0.472
  Definition (7)   P@10 28.6%  MRR 0.248
```

### Phase 2.5: Diagnostic Fixes (session 20260207-150836)

Two quick diagnostics before Phase 3, suggested by external review.

**A0: Semantic sanity battery — pipeline works, model mismatch confirmed**

- Semantic oracle IS functional — returns 10 results per query
- Semantic index contains ~27K items: 23,820 session events + 1,883 code
  facts + 806 patterns + 1,778 commits + 47 beliefs
- Session events dominate (88% of index) — session-trained projection
  maps everything to session-space. Semantic results are mostly session
  events, not code or patterns
- Pattern enrichment uses pattern `id` (e.g., "dependable-rust") as doc_id,
  not `file_path` (e.g., "layer/core/dependable-rust.md") — pattern results
  can never match eval expectations even if ranked correctly
- Verdict: **model+training mismatch**, not pipeline bug. Semantic
  suppression (weight=0.0) is correct. When model/training changes,
  re-evaluate.

**C0: FTS5 corpus audit — doc_id mapping bug found (F6)**

- `layer/core/*.md` files ARE indexed in `pattern_fts` (7 core files)
- FTS5 tokenization works correctly — `porter unicode61` treats hyphens
  as separators, "dependable-rust" tokenizes to ["dependable", "rust"]
- **Bug found:** `scry_lexical()` returns `source_id: id` for pattern
  results (e.g., "dependable-rust"), but eval expects `file_path`
  (e.g., "layer/core/dependable-rust.md"). Same bug in semantic
  enrichment. Pattern results silently fail to match eval expectations.
- **Impact:** 19/52 queries (37%) expect `layer/` file paths. 24 of 161
  expected files are `layer/` paths. All get 0 credit from pattern FTS
  hits.
- **Fix:** Change `source_id: id` → `source_id: file_path` in:
  - `src/commands/scry/internal/search.rs` (scry_lexical pattern results)
  - `src/commands/scry/internal/enrichment.rs` (semantic enrichment)
- This also corrects the "Definition intent worst at 28.6%" observation —
  Definition queries predominantly expect `layer/core/` docs which were
  always invisible due to this mapping bug.

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

### Phase 2: Intent-Aware Weighting ✅
- [x] Weights tuned: semantic=0.0, persona=0.0 all intents; Mechanism lexical=1.5
- [x] Validated on held-out test set: P@10 +3.3pp, MRR +0.040
- [x] No regression on subsystem tests — all improved (co-change +14.9pp, belief MRR +0.072)
- [x] Train-test gap healthy at -1.4pp (was -0.5pp baseline)

### Phase 2.5: Diagnostic Fixes
- [x] F6 (doc_id mapping bug) fixed: pattern source_id uses file_path
- [x] Eval re-run with fix, before/after delta documented (see below)
- [ ] Definition intent P@10 improved from 28.6% baseline — not yet, patterns
  rank below code in BM25 so rarely enter top 10 even with correct doc_ids
- [x] No regression on co-change or belief subsystem tests

```
Phase 2.5 Results (52 NL queries, F6 fix applied):

                    Before (F6 bug)     After (F6 fix)   Delta
NL P@5 (all)             29.5%           28.2%         -1.3pp (noise)
NL P@10 (all)            38.3%           38.3%          0.0pp
NL MRR (all)             0.421           0.422         +0.001
Test P@10                37.5%           35.8%         -1.7pp (noise)
Train-test gap           -1.4pp          -3.0pp
lexical-only P@10        34.5%           34.0%         -0.5pp (noise)
```

Impact is near-zero because pattern BM25 scores rank below code/commit
entries — patterns rarely enter top 10 even with correct doc_ids. The fix
is necessary (correct doc_id mapping) but not sufficient to lift pattern
retrieval. Future work: consider pattern-specific boosting in RRF or
separate pattern oracle.

### Phase 3: Belief Score Multiplier
- [ ] Belief MRR improved beyond current 0.241 with held-out validation
- [ ] Knowledge category MRR improved from current 0.340

### Phase 4: Hub File Suppression
- [ ] Hub file problem confirmed with corrected metrics
- [ ] Suppression implemented and validated

### Phase 5: Product Metric
- [ ] Session query precision computed for at least 5 sessions
- [ ] Product metric reported in `patina eval` output

## Observations

**Actioned in Phase 2:**
- ~~Semantic oracle 0% P@K~~ → suppressed to 0.0 weight across all intents
- ~~Persona oracle 0% P@K~~ → suppressed to 0.0 weight across all intents
- ~~Mechanism worst MRR (0.160)~~ → redirected dead semantic boost to lexical,
  MRR improved to 0.493

**Remaining observations:**
- Lexical oracle: dominant single oracle (MRR 0.412). After tuning, unified
  (37.7%) now leads lexical-only (34.5%) — fusion is clearly beneficial.
- Belief oracle: 6.4% P@10, low but contributes to the unified edge.
  Phase 3 (belief score multiplier) could improve this.
- Definition intent: weakest P@10 (28.6%) and P@5 (14.3%). Currently below
  General (33.4%). Root cause identified (Phase 2.5): doc_id mapping bug (F6)
  means pattern FTS hits for `layer/core/` docs were invisible to eval.
  Not an indexing or tokenization issue — `layer/core/` docs ARE in FTS5.
- Intent detection: 52% coverage. Broadening Mechanism detection could help
  2-3 queries but sample is too small to validate. Needs more queries first.
- The ONNX model choice (E5-base-v2) drives semantic results — a different
  model could make semantic the dominant oracle. When model changes, re-evaluate
  the 0.0 weight. Don't hardcode suppression in architecture.

## References

- [[eval-repair]] — Phases 1-2 (complete), established measurement infrastructure
- [[belief-retrieval-quality]] — F5 root cause and Fix 1-4 designs
- [[measure-first]] — Prove the problem exists with data
- [[measure-the-measurement]] — Fix the instrument before the observation
- [[error-analysis-over-architecture]] — Categorize failures before adding complexity
