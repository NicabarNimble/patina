# Spec: Lab Automation

**Status:** Ideas (Draft Only)
**Parent:** [build.md](../../../core/build.md)
**Depends on:** Model management complete, bench retrieval working
**Origin:** Observable Scry Phase 3 review — identified need for systematic measurement

---

> **Why Deferred:**
>
> Draft spec for automating benchmarking workflows. No implementation started.
>
> **Reason:**
> - Manual `patina bench` workflow is sufficient for current scale
> - Model comparison is infrequent (E5-base-v2 is working well)
> - Higher priority: observability, mother design
>
> **Resume trigger:** When model experimentation becomes frequent, or when retrieval quality regression tracking is needed.

---

## Problem

We have lab tools (`bench`, `eval`) but they require manual orchestration:

| Task | Current | Pain |
|------|---------|------|
| Model comparison | Edit config → oxidize → bench → repeat | 10+ minutes per model |
| Hyperparameter sweep | Edit config → bench → repeat | Manual, error-prone |
| Track improvement | Copy/paste results | No history |
| Error analysis | Read verbose output | No categorization |

Andrew Ng principle: "If you can't measure it quickly, you won't iterate on it."

---

## Solution

Automate the lab workflow while keeping tools composable.

### 1. Model A/B Testing

```bash
# Compare models without manual rebuild
patina lab compare-models \
  --models e5-base-v2,bge-small-en-v1-5 \
  --query-set .patina/lab/queries.json

# Output:
# Model Comparison: patina-core (5 queries)
# ─────────────────────────────────────────
# Model              MRR    R@5    R@10   Latency
# e5-base-v2        0.425   60%    100%    166ms
# bge-small-en-v1-5 0.512   80%    100%    142ms  ← winner
```

**Implementation:**
- For each model: set config → oxidize → bench → collect results
- Cache embeddings per model (already separate dirs)
- Restore original config after comparison

### 2. Hyperparameter Sweep

```bash
# Test RRF k values
patina lab sweep \
  --param rrf_k=20,40,60,80,100 \
  --query-set .patina/lab/queries.json

# Output:
# RRF K Sweep: patina-core
# ─────────────────────────
# rrf_k    MRR    R@5    R@10
# 20      0.380   60%    80%
# 40      0.412   60%    100%
# 60      0.425   60%    100%  ← current
# 80      0.418   60%    100%
# 100     0.410   60%    100%
```

### 3. Benchmark History

```bash
# Save benchmark results with timestamp
patina bench retrieval --query-set .patina/lab/queries.json --save

# View history
patina lab history

# Output:
# Benchmark History: patina-core
# ─────────────────────────────
# Date        Commit   MRR    R@5    R@10   Notes
# 2025-12-23  646f0f8  0.425  60%    100%   doc_id fix
# 2025-12-22  cec8b50  0.380  40%    80%    phase 3 complete
# 2025-12-20  ada02b8  0.350  40%    80%    orient mode
```

**Storage:** `.patina/lab/history.json`

### 4. Error Categorization

```bash
patina bench retrieval --query-set .patina/lab/queries.json --categorize

# Output:
# Error Analysis: patina-core
# ───────────────────────────
# Category              Count   Example
# session_doc_noise       2     semantic-oracle: session beat code
# wrong_granularity       1     scrape-entry: function beat module
# related_not_exact       1     embedding-load: models.rs beat onnx.rs
```

**Categories:**
- `session_doc_noise` — Session docs ranked above code
- `wrong_granularity` — Function/symbol when file expected (or vice versa)
- `related_not_exact` — Related file, not the specific answer
- `lexical_miss` — No lexical match for keywords in query
- `semantic_miss` — Low semantic similarity to expected doc

---

## Design Principles

### Unix Philosophy
Each capability is composable:
- `bench` measures one configuration
- `lab compare-models` orchestrates multiple bench runs
- `lab history` tracks over time

### Dependable Rust
Results are structured and machine-readable:
- `--json` output for all commands
- History file is append-only
- Comparisons produce diff-able output

### Local First
All data stays local:
- No cloud metrics service
- History in `.patina/lab/`
- Models in `~/.patina/cache/models/`

---

## Tasks

### Phase 1: Benchmark History

| Task | Scope |
|------|-------|
| Add `--save` flag to bench retrieval | ~30 lines |
| Create `.patina/lab/history.json` schema | ~20 lines |
| Add `patina lab history` command | ~50 lines |
| Include git commit SHA in saved results | ~10 lines |

### Phase 2: Model Comparison

| Task | Scope |
|------|-------|
| Add `patina lab compare-models` command | ~100 lines |
| Orchestrate: config swap → oxidize → bench → restore | ~80 lines |
| Format comparison table output | ~40 lines |
| Cache check: skip oxidize if embeddings exist for model | ~30 lines |

### Phase 3: Hyperparameter Sweep

| Task | Scope |
|------|-------|
| Add `patina lab sweep` command | ~80 lines |
| Parse `--param key=val1,val2,val3` syntax | ~30 lines |
| Run bench for each value, collect results | ~50 lines |

### Phase 4: Error Categorization

| Task | Scope |
|------|-------|
| Define error category heuristics | Design |
| Add `--categorize` flag to bench | ~60 lines |
| Classify each failure by category | ~100 lines |
| Aggregate and display summary | ~40 lines |

---

## Validation

| Criteria | How to Test |
|----------|-------------|
| Model comparison produces correct ranking | Compare manually, verify winner matches |
| History persists across runs | Run bench --save twice, check history has both |
| Sweep finds optimal value | Known optimal should be identified |
| Error categories are actionable | Each category suggests a fix |

---

## Baseline (Dec 2025)

Current metrics to improve from:

```
Query Set: patina-core (5 queries)
Model: e5-base-v2

Ablation:
  Semantic only:  MRR 0.201, R@5 40%, R@10 80%
  Lexical only:   MRR 0.422, R@5 60%, R@10 80%
  All (fusion):   MRR 0.442, R@5 80%, R@10 100%

Insight: Semantic is the bottleneck. Lexical does heavy lifting.
```

---

## References

- [spec-observable-scry.md](spec-observable-scry.md) — Feedback loop infrastructure
- [spec-model-management.md](spec-model-management.md) — Model swap mechanism
- [spec-work-deferred.md](spec-work-deferred.md) — Observable Scry gaps
