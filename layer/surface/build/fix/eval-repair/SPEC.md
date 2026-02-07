---
type: fix
id: eval-repair
status: complete
created: 2026-02-06
sessions:
  origin: 20260206-060219
related:
- layer/surface/build/explore/lab-automation/SPEC.md
beliefs:
- measure-first
- measure-the-measurement
- fix-data-not-tools
- error-analysis-over-architecture
---

# fix: Eval Repair — Measure the Right Thing

> "You're measuring the subsystems, not the product. You have no eval for the thing that
> actually matters: does the LLM get useful context when it asks a question?"

## Problem

Patina's eval infrastructure measures retrieval subsystems independently but not the product.
The Andrew Ng methodology says: error analysis before fixes, identify the real product metric,
fix the instrument before the observation, know when to stop.

**Eval run 2026-02-06 (current state):**

```
Pipeline                                     P@5         P@10    vs Random
───────────────────────────────────────────────────────────────────────────
unified (code→same-file)                    6.5%         7.5%        13.7x
no-belief (code→same-file)                  4.5%         6.7%        12.2x
semantic-only (code→same-file)              0.0%         0.0%         0.0x
unified (file→co-change)                   28.6%        57.5%        13.3x
no-belief (file→co-change)                 33.9%        59.2%        13.7x
temporal-only (file→co-change)             99.0%       100.0%        23.1x

belief self-retrieval MRR:    0.172    Hit Rate: 80.9%
belief→code co-retrieval:     50.0%
feedback loop precision:      0.0%    (133 queries, 2652 retrievals, 0 hits)
```

**Five failures identified:**

### F1: Feedback Loop Is Broken (CRITICAL)

`patina eval --feedback` shows 0% precision across 2652 retrievals. The "retrieval led to
commit" linkage never fires. This is our only measurement of real-world product quality and
it's returning nothing. The instrument is broken — we cannot tell if patina is useful.

### F2: No Natural-Language Query Eval (CRITICAL)

All eval tests use synthetic inputs:
- code→same-file: queries with function signatures (`Function classify_layer_path in src/...`)
- file→co-change: queries with file paths (`File src/main.rs Rust source`)
- belief self-retrieval: queries with exact belief text

No test asks "how does the scrape pipeline work?" or "what patterns should I follow for error
handling?" — the actual queries LLMs send via MCP. We are testing retrieval mechanics, not
product quality.

### F3: Fusion Dilutes the Best Oracle (HIGH)

The temporal oracle alone achieves 100% P@10 on co-change. Adding other oracles via RRF
**reduces** this to 57.5%. The fusion pipeline makes the best oracle worse.

```
temporal-only (file→co-change)    99.0%  100.0%   23.1x
unified (file→co-change)          28.6%   57.5%   13.3x   ← 42pp regression
```

This is the opposite of what fusion should do. RRF assumes each oracle contributes useful
signal. When one oracle is dominant, RRF dilutes it with noise from weaker oracles.

### F4: Semantic Oracle Adds Nothing for Code (MEDIUM)

semantic-only scores 0% on code→same-file. E5-base-v2 embeddings don't help find structural
code neighbors. The semantic oracle's value is for natural-language queries (which we don't
test — see F2).

### F5: Belief MRR Stuck at 0.172 (MEDIUM)

Beliefs are found (80.9% hit rate) but ranked at position ~6. The structural RRF disadvantage
is known: single-oracle results score max 0.016, multi-oracle results score 0.028-0.060.
Fix 1 (score multiplier) and Fix 2 (add beliefs to LexicalOracle) are designed but not
implemented. See [[belief-retrieval-quality]].

---

## Root Cause Analysis (Andrew Ng Methodology)

### Step 1: What is the real product metric?

The product claim: "Give an LLM the right context about your project."

The real product metric is **session query precision**: given real queries from session history,
does scry return files/patterns/beliefs that the LLM actually ended up reading or editing?

```
session query precision = (retrievals that led to file reads/edits) / (total retrievals)
```

This is what `--feedback` was supposed to measure. It doesn't work.

### Step 2: Why doesn't the feedback loop work?

The feedback loop tries to connect `retrievals → commits`. But:
1. Retrievals are logged in eventlog during scry queries
2. Commits are logged in the git history
3. The linkage assumes: if a retrieved file appears in a subsequent commit, the retrieval "led to" that commit

**Hypothesis for 0%:** The linkage window, file matching, or event format is broken. The
mechanism needs debugging — not the retrieval system.

### Step 3: What should we measure instead / additionally?

| Metric | Measures | How |
|--------|----------|-----|
| **Session query precision** | Do scry results match files the session actually touched? | Match scry results against `git diff` of session commits |
| **Context relevance** | Does `context` output include patterns relevant to session work? | Compare context output against files changed in session |
| **Natural-language retrieval** | Does scry answer "how does X work?" correctly? | Curated query→expected_results test set from real session queries |
| **Stale context rate** | What % of served content has temporal drift? | spec-drift-detection Phase 1 |
| **LLM adoption** | How often does the LLM call scry/context? | Count MCP tool calls from session logs |

### Step 4: Fix order (highest impact first)

1. **Fix the feedback loop** (F1) — unblock real-world measurement
2. **Build NL query eval** (F2) — test the actual product, not subsystems
3. **Address fusion dilution** (F3) — intent-aware oracle weighting
4. **Belief MRR** (F5) — Fix 1 (score multiplier) is low-effort, high-impact
5. **Semantic oracle assessment** (F4) — may need NL eval data first

---

## Design

### Phase 1: Fix the Feedback Loop (F1)

Debug why `eval --feedback` returns 0% precision:

1. **Trace the data flow:**
   - Where are retrieval events logged? (eventlog table, event type?)
   - Where are commit events logged? (commits table from scrape git)
   - What is the linkage query? (SQL join between retrievals and commits)
   - What time window is used? (retrieval before commit, how long?)

2. **Find the break:**
   - Are retrieval events being written? (`SELECT COUNT(*) FROM eventlog WHERE event_type LIKE 'scry%'`)
   - Are they matching any commits? Debug the JOIN conditions
   - Is the file path format consistent between retrieval results and commit file paths?

3. **Fix and verify:**
   - Ensure retrieval events use the same file path format as commits
   - Ensure the time window is reasonable (retrieval within same session → commit)
   - Re-run: target > 0% (even 5% means the instrument works)

### Phase 2: Natural-Language Query Eval (F2)

Build a curated test set from real session queries:

1. **Extract real queries:**
   - Mine session archives for what LLMs actually asked patina
   - Extract MCP tool calls from session history (scry queries, context calls)
   - Select 20-30 diverse queries covering: knowledge, structural, temporal, rationale

2. **Define expected results:**
   - For each query, manually identify 3-5 files/patterns/beliefs that SHOULD be returned
   - Store as `resources/eval/nl-queries.json`:
     ```json
     {
       "query": "how does the scrape pipeline work?",
       "expected": [
         "src/commands/scrape/mod.rs",
         "layer/core/build.md",
         "src/commands/scrape/layer/mod.rs"
       ],
       "category": "knowledge"
     }
     ```

3. **Measure:**
   - Run each query through scry
   - Compute Precision@5, Precision@10, MRR against expected results
   - Ablation: unified vs per-oracle (which oracle contributes most to NL queries?)

4. **Establish baseline:**
   - This becomes the ground truth for all future retrieval changes
   - Regression = NL precision drops

### Phase 3: Fusion Quality (F3 + F5)

After Phases 1-2 establish what's actually broken:

1. **Intent-aware weighting:**
   - Structural queries (file paths, function names): boost temporal, suppress belief
   - Knowledge queries (how, why, explain): boost belief, boost semantic
   - The intent system exists — extend it with oracle-specific weights

2. **Belief score multiplier (F5, Fix 1):**
   - Compensate for single-oracle RRF disadvantage
   - `belief_weight = 3.0` in IntentWeights for knowledge intents
   - Low effort, directly addresses MRR 0.172 → target 0.300+

3. **Measure fusion delta:**
   - With NL eval from Phase 2: does fusion help or hurt NL queries?
   - If semantic oracle adds value for NL (unlike code queries), keep it
   - If not, consider removing it from default pipeline

### Phase 4: Product Metric Dashboard

Wire session query precision into `patina eval` or `patina doctor --dev`:

```
$ patina eval --product

Product Metrics (last 10 sessions):
  Session query precision:   34%  (retrievals that matched edited files)
  Context relevance:         67%  (context patterns relevant to session work)
  NL retrieval P@5:          45%  (from curated test set)
  Stale context rate:         8%  (specs with >30 day drift)
  MCP adoption:              12 calls/session average
```

---

## Exit Criteria

### Phase 1: Feedback Loop ✓

- [x] Root cause of 0% feedback precision identified and documented
  - 4 bugs: session ID format mismatch, ROW_NUMBER partition, doc_id `::` suffix, git rename paths
- [x] `patina eval --feedback` returns > 0% on at least 1 session with known retrieval→edit pairs
  - Achieved 2.2% precision (commit 38acca48)
- [x] Feedback linkage methodology documented (event types, time windows, file matching)

### Phase 2: Natural-Language Eval ✓

- [x] 20+ curated NL queries with expected results in `resources/eval/nl-queries.json`
  - 25 queries across knowledge (13), structural (10), rationale (2) categories
- [x] `patina eval --nl` runs NL queries and reports P@5, P@10, MRR
  - Commit 18e71d58
- [x] Baseline NL metrics recorded
  - **Corrected** (dedup fix 4772bd20): P@5 27.5%, P@10 33.7%, MRR 0.408
  - Original was inflated by doc_id double-counting (+7.4pp P@10)
  - Knowledge weakest: P@5 17.3%, MRR 0.332
- [x] Ablation: per-oracle contribution to NL queries measured
  - **Corrected**: unified (33.7%) slightly beats lexical-only (31.1%) on P@10
  - Original claim "lexical-only dominates at 77.2%" was inflated (+46pp)
  - semantic-only and persona-only: 0% (no NL contribution — may be model-dependent)
  - Commit 0f79d151, corrected 4772bd20

### Phase 3: Fusion Quality

- [ ] Belief MRR improved from 0.172 to >= 0.300 (Fix 1: score multiplier)
- [ ] Intent-aware oracle weighting implemented (structural vs knowledge queries)
- [ ] Fusion delta measured: unified vs temporal-only on co-change (target: < 10pp regression)
- [ ] Fusion delta measured: unified vs semantic-only on NL queries

### Phase 4: Product Metric

- [ ] Session query precision computed for at least 5 sessions
- [ ] Product metric reported in `patina eval` output
- [ ] Baseline product metric recorded

---

## What This Spec Does NOT Tackle

- **Scrape pipeline changes** — this fixes measurement, not data collection
- **New oracles** — no new search channels, just better weighting of existing ones
- **MCP tool changes** — delivery format unchanged, ranking quality improved
- **Cross-project / federation eval** — local project only for now

---

## References

- [[belief-retrieval-quality]] — F5 root cause analysis and Fix 1-4 designs
- [[measure-first]] — Prove the problem exists with data
- [[measure-the-measurement]] — Fix the instrument before the observation
- [[error-analysis-over-architecture]] — Categorize failures before adding complexity
- [[fix-data-not-tools]] — The retrieval data is fine; the delivery/measurement is broken
- Andrew Ng error analysis methodology (applied in session 20260205-200816)
