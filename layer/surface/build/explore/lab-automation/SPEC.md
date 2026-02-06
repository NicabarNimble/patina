---
type: explore
id: lab-automation
status: active
created: 2026-01-13
updated: 2026-02-06
tags: [eval, benchmarking, quality, measurement]
references: [measure-first, measure-the-measurement, error-analysis-over-architecture]
related:
  - layer/surface/build/fix/eval-repair/SPEC.md
---

# Explore: Lab Automation — Make Quality Measurement a Habit

> eval-repair fixes broken instruments. Lab automation is the workbench for continuous improvement.

## Core Question

How do we make retrieval quality measurement repeatable, trackable, and habitual — not a manual one-shot?

## Unique Value (not covered by eval-repair)

### 1. Persistent Benchmark History

Track structured retrieval metrics over time, not just session snapshots.

```
Date        Commit   MRR    P@5    P@10   Notes
2026-02-06  7295647  0.624  —      67.5%  post-mother-delivery
2026-01-22  a500de9  0.588  —      —      post-database-identity
```

Storage: `.patina/lab/history.json` — append-only, git-tracked.
Detects regressions automatically. `patina lab history` shows trends.

### 2. Model A/B Testing

Compare embedding models without manual rebuild cycles.

```bash
patina lab compare-models --models e5-base-v2,bge-small-en-v1-5
```

For each model: set config → oxidize → bench → collect → restore.
Answers: "is there a better model than E5-base-v2?"

### 3. Hyperparameter Sweeps

Fusion dilutes the best oracle (temporal 100% → unified 57.5%). That's likely a tuning
problem. Sweep tooling finds optimal parameters without manual iteration.

```bash
patina lab sweep --param rrf_k=20,40,60,80,100
```

Answers: "what RRF k value minimizes fusion dilution?"

### 4. Automated Error Categorization

Andrew Ng error analysis as repeatable tooling, not manual one-shot.

```bash
patina bench retrieval --categorize
```

Categories: session_doc_noise, wrong_granularity, related_not_exact, lexical_miss, semantic_miss.
Each category suggests a fix. Track category counts over time to see if fixes work.

## Relationship to Other Specs

- **eval-repair**: fixes the instruments (feedback loop, NL eval, fusion). Lab automation uses those instruments repeatedly.
- **spec-report**: tracks project state. Lab automation tracks retrieval quality specifically.
- **spec-observability**: tracks command usage. Lab automation tracks retrieval outcomes.

## Open Questions

- Should benchmark history live in `.patina/lab/` or in the eventlog?
- Is `patina lab` the right command namespace or should this extend `patina bench`/`patina eval`?
- What's the minimum history needed before trend detection is useful?
- Should sweeps run automatically at session boundaries (doctor-dev integration)?

## References

- Andrew Ng: "If you can't measure it quickly, you won't iterate on it."
- Session 20260206-060219: eval run + error analysis (manual, one-shot)
- Baseline: MRR 0.624, Recall@10 67.5%, Latency ~135ms (as of v0.14.2)
