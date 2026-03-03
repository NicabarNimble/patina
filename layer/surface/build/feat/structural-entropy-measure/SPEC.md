---
type: feat
id: structural-entropy-measure
status: active
created: 2026-03-03
target: '4'
sessions:
  origin: 20260303-101839
related:
- drift-detection
- scrape-diff-driven
beliefs:
- measure-is-ambient-health-for-llm-context
- gjengset-lens-type-integrity
- steenberg-lens-immutable-core
exit_criteria:
- id: structural-metrics
  text: '`patina measure` reports structural metrics: module count (directories under src/), public interface count (pub fn/struct/enum/trait from scraper data), and dependency count (from Cargo.toml)'
  checked: true
- id: coupling-metric
  text: '`patina measure` reports cross-module coupling as fan-out per module (how many other modules each module imports from) using existing import_facts data, showing average and max'
  checked: true
- id: entropy-delta-diagnostic
  text: '`patina measure` warns when structural metrics increase beyond hardcoded thresholds between scrapes (delta from last measure.capture.structure event)'
  checked: true
- id: entropy-in-context
  text: '`patina context` includes current structural entropy summary so LLMs see codebase shape before making changes'
  checked: true
---
# feat: Structural Entropy Tracking in Measure System

> Extend `patina measure` to track structural metrics — module count,
> public interface count, dependency count, cross-module coupling — so
> that "entropy increased" is a measurable diagnostic, not a manual table
> filled in by the agent.

## Problem

Today `patina measure` tracks 5 protocol verbs (capture, index, search,
believe, evolve) with operational metrics: scrape duration, embedding
freshness, search quality, belief count, spec velocity. These measure
whether Patina is *working*. They don't measure whether the *codebase*
is healthy.

The compression-first workflow needs structural metrics to answer:
- Did this change increase or decrease entropy?
- Are public interfaces growing beyond intent?
- Is cross-module coupling increasing?
- Are new dependencies justified?

Without measurement, entropy claims are subjective. Per
[[measure-is-ambient-health-for-llm-context]]: if it's not measured,
it's not managed.

## Solution

Add a `structure` verb to the measure system (or extend the `capture`
verb) that computes structural metrics from the code scraper's existing
data:

| Metric | Source | What it measures |
|--------|--------|-----------------|
| Module count | `src/` directory walk at scrape time | System decomposition breadth |
| Public interface count | scraper AST data (pub fn/struct/enum/trait) | API surface area |
| Dependency count | Cargo.toml parse at scrape time | External coupling |
| Cross-module fan-out | `import_facts` cross-module aggregation | Internal coupling |

These feed into diagnostics: "public_interface_count increased by 12
since last scrape" triggers investigation, same pattern as scrape duration
regression diagnostic from [[data-fast-incremental]].

The metrics flow into `patina context` so LLMs see codebase shape
alongside beliefs and patterns.

## Exit Criteria

See frontmatter.

## Non-Goals

- **Code quality scoring.** No subjective "code quality" number. Only
  objective structural counts that trend over time.
- **Blocking builds.** Metrics are diagnostics (warnings), not gates.
  [[drift-detection]] adds the enforcement layer.
- **Language-specific analysis.** Metrics use existing scraper output
  (function_facts, import_facts), not new parsing.
