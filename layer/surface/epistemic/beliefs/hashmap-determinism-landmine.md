---
type: belief
id: hashmap-determinism-landmine
persona: architect
facets: [rust, determinism, ml-training, serialization]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-09
revised: 2026-02-09
---

# hashmap-determinism-landmine

HashMap iteration order is non-deterministic in Rust — any pipeline requiring reproducible output (training, serialization, checksums) must sort collected iterators or use BTreeMap. This applies to all layers: data ordering, algorithm state, serialization format, and runtime execution.

## Statement

HashMap iteration order is non-deterministic in Rust — any pipeline requiring reproducible output (training, serialization, checksums) must sort collected iterators or use BTreeMap. This applies to all layers: data ordering, algorithm state, serialization format, and runtime execution.

## Evidence

- [[session-20260209-075426]]: Phase 5c validation discovered 3 HashMap non-determinism sources (generator iteration in temporal/dependency/commits, recipe projection ordering, safetensors metadata serialization) plus HashSet iteration in partner selection. Each caused different output checksums despite identical inputs and fixed seeds. (weight: 0.95)

## Supports

- [[andrew-ng-over-shoulder]] — reproducible measurements require deterministic pipelines; non-determinism makes before/after comparisons meaningless
- [[corpus-composition-over-model]] — corpus ordering affects training outcomes; non-deterministic ordering masks corpus composition effects

## Attacks

<!-- None identified -->

## Attacked-By

- Performance cost: BTreeMap is O(log n) vs HashMap O(1). Sorting adds overhead. Acceptable in training pipelines but may matter in hot paths.
- ONNX Runtime still has residual float non-determinism even with `deterministic_compute(true)` — sorting alone is necessary but not sufficient for full ML pipeline determinism.

## Applied-In

- `src/commands/oxidize/temporal.rs` — sorted `files_with_cochanges`, `all_files_vec`, `partners_vec` from HashMap/HashSet iterators ([[6c8c790a]])
- `src/commands/oxidize/dependency.rs` — sorted `functions_with_calls`, `all_functions_vec`, `partners_vec` from HashMap/HashSet iterators ([[6c8c790a]])
- `src/commands/oxidize/commits.rs` — sorted `all_files` from HashMap keys ([[6c8c790a]])
- `src/commands/oxidize/mod.rs` — sorted `recipe.projections` HashMap iteration ([[6c8c790a]])
- `src/commands/oxidize/trainer.rs` — removed HashMap metadata from safetensors serialization ([[6c8c790a]])

## Revision Log

- 2026-02-09: Created — metrics computed by `patina scrape`
