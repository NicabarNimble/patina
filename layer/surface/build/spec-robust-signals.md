# Spec: Robust Signals (Phase 1.5)

**Status:** Planned
**Parent:** [build.md](../../core/build.md)
**Depends on:** Phase 1 (Assay Signals) - complete

## Problem

Current `importer_count` signal is ~60% accurate because:
- Relative imports (`use super::foo`) don't match full module paths
- Language-specific import syntax (Rust `use`, Python `from/import`, JS `import`)
- Glob imports, re-exports, aliasing

Trying to fix accuracy is wrong approach - leads to language-specific code, fragile heuristics.

## ML/RL Framing

We're building a **retrieval re-ranking system**:

```
Query → Semantic/Lexical candidates → Re-rank by signals → Top-k
```

Structural signals are **priors** - query-independent importance scores. Like PageRank for web search: "how important is this page before I know your query?"

### Key Insight

We don't need *accurate* features. We need features that are:

1. **Correlated** with what we care about (usefulness)
2. **Robust** across distributions (different repos, languages)
3. **Cheap** to compute

A 60% accurate signal is still useful. Many weak signals > one accurate signal.

## Proposed Signals

### Git-based (Language-agnostic, High Reliability)

| Signal | Computation | Accuracy |
|--------|-------------|----------|
| `commit_count` | COUNT of git.commit events touching file | ~95% |
| `contributor_count` | COUNT DISTINCT authors | ~95% |
| `days_since_commit` | NOW - MAX(timestamp) | ~99% |
| `churn_rate` | lines_added + lines_removed over time | ~90% |

### Filename-based (Language-agnostic, High Reliability)

| Signal | Computation | Accuracy |
|--------|-------------|----------|
| `is_entry_point` | Matches: main.*, index.*, __init__.py, mod.rs | ~99% |
| `is_test_file` | Path contains test/, tests/, _test., .test. | ~99% |
| `directory_depth` | Count of / in path | ~99% |
| `is_internal` | Path contains internal/, private/, _internal | ~95% |

### Size-based (Language-agnostic)

| Signal | Computation | Accuracy |
|--------|-------------|----------|
| `file_size_rank` | Percentile rank by bytes | ~99% |
| `line_count_rank` | Percentile rank by lines | ~99% |
| `function_count` | From function_facts table | ~85% |

### Structural (Language-dependent, Accept Noise)

| Signal | Computation | Accuracy |
|--------|-------------|----------|
| `importer_count` | Current LIKE matching | ~60% |
| `centrality_score` | Degree in call_graph | ~70% |

## Schema Update

```sql
ALTER TABLE module_signals ADD COLUMN commit_count INTEGER;
ALTER TABLE module_signals ADD COLUMN contributor_count INTEGER;
ALTER TABLE module_signals ADD COLUMN is_entry_point INTEGER;
ALTER TABLE module_signals ADD COLUMN is_test_file INTEGER;
ALTER TABLE module_signals ADD COLUMN directory_depth INTEGER;
ALTER TABLE module_signals ADD COLUMN file_size_rank REAL;
```

## Normalization

All signals normalized to 0-1 for fusion:

```rust
fn normalize(value: f64, min: f64, max: f64) -> f64 {
    (value - min) / (max - min)
}

// Or percentile rank for skewed distributions
fn percentile_rank(value: f64, sorted_values: &[f64]) -> f64 {
    // position in sorted list / total count
}
```

## Composite Score

For StructuralOracle ranking:

```rust
let importance =
    0.3 * commit_count_norm +
    0.2 * contributor_count_norm +
    0.2 * (1.0 - days_since_commit_norm) +  // fresher = better
    0.1 * importer_count_norm +
    0.1 * centrality_norm +
    0.1 * file_size_rank;

// Phase 3: learn these weights per-repo
```

## Validation

| Metric | Baseline (Phase 1) | Target |
|--------|-------------------|--------|
| MRR | 0.542 | 0.60+ |
| Recall@5 | 42.7% | 50%+ |

## Implementation Steps

1. Add new columns to module_signals schema
2. Update `assay derive` to compute new signals
3. Add normalization pass after all signals computed
4. Update StructuralOracle to use composite importance score
5. Re-run `patina bench retrieval` and compare

## Design Principle

```
Many weak signals > One accurate signal
```

Don't over-engineer individual signal accuracy. Let Phase 3 (learned fusion weights) figure out what matters for each repo.
