---
type: fix
id: oxidize-parallelism
status: ready
created: 2026-03-10
sessions:
  origin: 20260309-182853
exit_criteria:
- id: onnx-session-exposes-separate-fast-and-deterministic-modes
  text: ONNX session exposes separate fast and deterministic modes
  checked: false
- id: single-query-paths-scry-assay-use-fast-auto-threaded-inference
  text: single-query paths (scry, assay) use fast auto-threaded inference
  checked: false
- id: oxidize-uses-deterministic-single-thread-onnx-sessions-with-outer-parallelism
  text: oxidize uses deterministic single-thread ONNX sessions with outer parallelism
  checked: false
- id: embed-batch-performs-real-batched-onnx-inference-with-padded-tensors
  text: embed_batch performs real batched ONNX inference with padded tensors
  checked: false
- id: prefix-aware-batch-apis-preserve-query-passage-formatting-for-asymmetric-models
  text: prefix-aware batch APIs preserve query/passage formatting for asymmetric models
  checked: false
- id: oxidize-training-loop-uses-rayon-par-chunks-with-per-thread-deterministic-embedders
  text: oxidize training loop uses rayon par_chunks with per-thread deterministic embedders
  checked: false
- id: oxidize-index-building-loop-uses-rayon-par-chunks-with-per-thread-deterministic-embedders-while-preserving-output-order
  text: oxidize index building loop uses rayon par_chunks with per-thread deterministic embedders while preserving output order
  checked: false
- id: cpu-saturates-during-large-repo-oxidize-on-m2-max
  text: CPU saturates during large repo oxidize on M2 Max
  checked: false
- id: projection-artifacts-remain-stable-across-consecutive-oxidize-runs
  text: projection artifacts remain stable across consecutive oxidize runs
  checked: false
- id: existing-tests-pass-cargo-test
  text: existing tests pass — cargo test
  checked: false
- id: duckdb-repo-re-oxidize-completes-in-under-10-minutes-was-40
  text: DuckDB repo re-oxidize completes in under 10 minutes (was 40+)
  checked: false
---
# fix: Oxidize embedding parallelism

> ONNX embedder hardcodes intra/inter threads to 1, embed_batch is sequential, and oxidize loops embed_passage one-at-a-time — large repos (92k functions) take 40+ minutes on M2 Max at 19% CPU instead of saturating all cores

## Problem

Adding DuckDB as a reference repo (92k functions, 52k commits) took 40+ minutes for oxidize embedding on an M2 Max. btop showed 19% CPU — one core busy, eleven idle. The system should be saturating all cores during batch embedding work.

## Root Cause

Three compounding issues:

1. **ONNX session is locked to deterministic single-thread execution globally** (`src/embeddings/onnx.rs:84-92`):
   ```rust
   .with_intra_threads(1)
   .with_inter_threads(1)
   .with_deterministic_compute(true)
   ```
   This is correct for reproducible oxidize artifacts, but it also forces interactive single-query paths to stay single-threaded. Patina currently has no split between "fast query inference" and "deterministic build inference".

2. **`embed_batch` is fake** (`src/embeddings/onnx.rs:275-278`):
   ```rust
   fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
       texts.iter().map(|t| self.embed(t)).collect()
   }
   ```
   Sequential loop, no batched tensor construction. Each call has full ONNX session overhead.

3. **oxidize loops `embed_passage` one-at-a-time** (`src/commands/oxidize/mod.rs:272-276`):
   ```rust
   for pair in &pairs {
       anchors.push(embedder.embed_passage(&pair.anchor)?);
       positives.push(embedder.embed_passage(&pair.positive)?);
       negatives.push(embedder.embed_passage(&pair.negative)?);
   }
   ```
   92k triplets = 278k sequential ONNX inference calls. Same pattern in `build_projection_index` (lines 364-375).

## Fix

Three layers, each independent and additive:

### Layer 1: Split fast and deterministic embedder modes

**Files:** `src/embeddings/onnx.rs`, `src/embeddings/mod.rs`

Add an explicit execution mode to `OnnxEmbedder::new_from_paths`:
- `FastQuery` — `intra_threads=0`, `inter_threads=0`, deterministic compute disabled. Used by single-query paths (`scry`, `assay`) where latency matters more than bitwise reproducibility.
- `DeterministicBuild` — `intra_threads=1`, `inter_threads=1`, deterministic compute enabled. Used by `oxidize`, where we want stable projection artifacts across runs.

Factory split:
- `create_embedder()` becomes the fast default for interactive paths.
- Add `create_embedder_deterministic()` for existing single-thread oxidize behavior.
- Add `create_embedder_for_parallel()` for rayon workers; it uses the same deterministic ONNX settings as `create_embedder_deterministic()`.

Do **not** remove `with_deterministic_compute(true)` from the deterministic build path. That flag was added to fix reproducibility drift in oxidize outputs and should remain part of the build-mode contract.

### Layer 2: Real batched ONNX inference with prefix-aware APIs

**Files:** `src/embeddings/onnx.rs`, `src/embeddings/mod.rs`

Implement true batch processing in `embed_batch`:
- Tokenize all texts in the batch
- Pad sequences to max length in batch (attention_mask handles padding)
- Stack into `[batch_size, max_seq_len]` tensors for input_ids, attention_mask, token_type_ids
- Single ONNX `session.run()` call for the whole batch
- Extract per-item embeddings from `[batch_size, seq_len, hidden_dim]` output
- Mean-pool and normalize each item

This amortizes ONNX session overhead and enables better SIMD/cache utilization on batched matrix ops.

Preserve asymmetric model behavior:
- Add `embed_query_batch(&[String])` and `embed_passage_batch(&[String])`, or an equivalent internal helper that applies model-specific prefixes before tokenization.
- `embed_batch` alone is not enough for E5/BGE models because query and passage formatting differ.

### Layer 3: Deterministic outer parallelism in oxidize

**Files:** `src/commands/oxidize/mod.rs`, `src/embeddings/mod.rs`

`ort::Session::run` takes `&mut self` — a single session can't be shared across threads. Use rayon's `par_chunks` + `map_init` pattern: each rayon worker creates its own deterministic single-thread embedder and reuses it across its chunk.

**Training loop** (`train_projection`):
```rust
use rayon::prelude::*;

let indexed_texts: Vec<(usize, String)> = pairs.iter()
    .enumerate()
    .flat_map(|(i, p)| [
        (i * 3, p.anchor.clone()),
        (i * 3 + 1, p.positive.clone()),
        (i * 3 + 2, p.negative.clone()),
    ])
    .collect();

let mut all_embeddings: Vec<(usize, Vec<f32>)> = indexed_texts
    .par_chunks(64)
    .map_init(
        || create_embedder_for_parallel().unwrap(),
        |embedder, chunk| {
            chunk.iter()
                .map(|(idx, text)| {
                    Ok((*idx, embedder.embed_passage(text)?))
                })
                .collect::<Result<Vec<_>>>()
        },
    )
    .collect::<Result<Vec<_>>>()?
    .flatten()
    .collect();

all_embeddings.sort_by_key(|(idx, _)| *idx);
// Deinterleave back to anchors/positives/negatives in original order
```

**Index building loop** (`build_projection_index`):
```rust
let mut embeddings: Vec<(usize, i64, Vec<f32>)> = events
    .iter()
    .enumerate()
    .collect::<Vec<_>>()
    .par_chunks(64)
    .map_init(
        || create_embedder_for_parallel().unwrap(),
        |embedder, chunk| {
            chunk.iter().map(|(order, (id, content))| {
                let emb = embedder.embed_passage(content)?;
                let vec = match projection {
                    Some(proj) => proj.forward(&emb),
                    None => emb,
                };
                Ok((*order, *id, vec))
            }).collect::<Result<Vec<_>>>()
        },
    )
    .collect::<Result<Vec<_>>>()?
    .flatten()
    .collect();

embeddings.sort_by_key(|(order, _, _)| *order);
for (_, id, vector) in &embeddings {
    index.add(*id as u64, vector)?;
}
```

rayon auto-detects core count. With N workers × deterministic single-thread ONNX sessions, we get N concurrent embedding jobs without changing the floating-point execution inside a single inference call. This is the path to higher CPU utilization while preserving reproducible oxidize artifacts.

Add an explicit worker cap:
- Default to `min(available_parallelism, 8)` or similar conservative bound.
- Allow override via config/env for high-memory machines.

## Risks

- **Model file loading**: Each rayon thread loads the ONNX model file independently. The model is 33MB (E5-base-v2 quantized) — at 12 threads that's ~400MB transient memory. Acceptable on M2 Max (32GB+), may need thread count capping on smaller machines.
- **Error handling in map_init**: `create_embedder().unwrap()` inside rayon will panic the thread. Should propagate errors cleanly — collect `Result` values and check after join.
- **Determinism leakage**: If results are inserted into the index in rayon completion order instead of source order, artifact stability may still drift even if individual embeddings are deterministic.
- **Prefix bugs**: Batched inference that skips E5/BGE query/passage prefixes will silently hurt retrieval quality.
- **Oversubscription**: Using auto-threaded ONNX sessions inside rayon workers would multiply core usage and hurt throughput. Build workers must stay single-threaded internally.

## Exit Criteria

1. ONNX session exposes separate `FastQuery` and `DeterministicBuild` modes
2. `scry`/`assay` use fast auto-threaded inference
3. `oxidize` uses deterministic single-thread ONNX sessions with outer rayon parallelism
4. `embed_batch` performs real batched ONNX inference with padded tensors
5. Batch APIs preserve query/passage prefixes for asymmetric models
6. oxidize training loop uses `rayon::par_chunks` with per-thread deterministic embedders
7. oxidize index building loop uses `rayon::par_chunks` with per-thread deterministic embedders and stable insertion order
8. Consecutive oxidize runs produce identical projection artifacts on the same repo input
9. CPU saturates during large repo oxidize on M2 Max
10. Existing tests pass — `cargo test`
11. DuckDB repo re-oxidize completes in under 10 minutes (was 40+)
