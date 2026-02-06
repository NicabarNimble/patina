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

## Root Cause Analysis (Do First)

Before fixing anything, do error analysis per Ng methodology:

### Step 1: Categorize self-retrieval misses

For the 21.3% of beliefs NOT found in top-10:
- Are they short statements (low FTS5 signal)?
- Are they generic (match too many documents)?
- Are they recent (not yet embedded)?

### Step 2: Categorize co-retrieval failures

For the 78.6% where belief is present but code isn't (57.2% of queries):
- How many reached files does the belief have? (many = harder)
- Are the reached files being retrieved by other oracles but deduped away?
- Are the reached files not in any oracle's results at all?

### Step 3: RRF slot analysis

For the file→co-change regression:
- How many belief results appear in a typical temporal query?
- Are they displacing the 10th result or the 3rd result?
- Would capping belief results to K=3 fix the regression?

## Potential Fixes (Prioritize After Error Analysis)

- **RRF score calibration** — BeliefOracle scores may not be calibrated to compete with temporal/semantic scores
- **Result count limits** — cap BeliefOracle to N results per query to limit RRF slot consumption
- **Intent-based routing** — suppress beliefs for clearly-structural queries (file paths, function signatures)
- **Belief score boosting** — when query semantically matches a belief, boost its RRF weight

## Exit Criteria

- [ ] Error analysis complete for all three issues (categorized, root causes identified)
- [ ] Self-retrieval MRR >= 0.400 (beliefs in top 2-3 on average)
- [ ] Co-retrieval rate >= 40% (belief + code delivered together)
- [ ] file→co-change regression within 5pp budget
- [ ] D1 VERDICT: PASS

## See Also

- [[eval-belief-ground-truth]] — eval that produced these measurements
- [[d1-belief-oracle/SPEC.md]] — D1 exit criterion
