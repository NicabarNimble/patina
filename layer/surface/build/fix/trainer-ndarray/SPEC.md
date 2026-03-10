---
type: fix
id: trainer-ndarray
status: draft
created: 2026-03-10
sessions:
  origin: 20260309-182853
related:
- oxidize-parallelism
exit_criteria:
  - Projection weights stored as Array2<f32> not Vec<Vec<f32>>
  - forward pass uses ndarray dot on contiguous arrays, not hand-rolled loops
  - backprop uses ndarray ops for weight gradient accumulation
  - save/load safetensors works with contiguous Array2 without unsafe pointer casts
  - gradient computation uses pre-update weights (fix current trainer bug)
  - repeated training on the same input produces identical losses and identical saved weights
  - existing tests pass — cargo test
  - DuckDB dependency projection trains measurably faster (add timing to measure)
---
# fix: MLP trainer: Vec of Vec to ndarray without breaking determinism

> Projection trainer uses Vec<Vec<f32>> for weight matrices and hand-rolled dot/backprop loops. That gives poor memory locality and a lot of scalar hot-loop work. The fix is to move to contiguous ndarray storage and vectorized ndarray ops while keeping oxidize builds deterministic.

## Problem

After [[oxidize-parallelism]] fixed embedding CPU utilization (19% → 92%), the MLP training phase is now the dominant bottleneck in oxidize for large repos. DuckDB's dependency projection: 92k triplets × 3 forward passes × 10 epochs = 2.7M forward passes through a 768→1024→256 network, all using scalar Rust loops with non-contiguous memory.

## Root Cause

`src/commands/oxidize/trainer.rs` stores weight matrices as `Vec<Vec<f32>>`:

```rust
pub struct Projection {
    pub w1: Vec<Vec<f32>>,  // hidden_dim × input_dim
    pub b1: Vec<f32>,
    pub w2: Vec<Vec<f32>>,  // output_dim × hidden_dim
    pub b2: Vec<f32>,
}
```

Every operation chases pointers through nested heap allocations:

- `dot()` (line 430): scalar `a.iter().zip(b.iter()).map(|(x,y)| x*y).sum()` — no SIMD vectorization
- `forward()` (line 79): iterates rows of `Vec<Vec<f32>>`, calling `dot()` per row — this is a matrix-vector multiply but the compiler can't see it
- `update_weights()` (line 194): nested `for i in 0..w2.len() { for j in 0..w2[i].len() }` — hand-rolled gradient accumulation across 3 triplet branches
- `backprop_linear()` (line 409): same pattern, transpose multiply by hand

ndarray is already in deps (`ndarray = "0.16"`) but only used in `onnx.rs` for tensor reshaping. The trainer doesn't use it at all. The current code also has a correctness bug: `update_weights()` mutates `w2` and then immediately backprops through the updated matrix, so `w1`/`b1` gradients do not match the forward pass.

## Fix

### 1. Keep deterministic math as the default build path

**File:** `Cargo.toml`

```toml
ndarray = "0.16"
```

Do not make BLAS part of the default `oxidize` path in this fix. Build determinism matters more than absolute peak throughput here.

If a faster non-deterministic path is desirable later, add it separately behind an explicit opt-in feature/env/config for non-build use. That is not part of this spec.

### 2. Replace Vec<Vec<f32>> with Array2<f32>

**File:** `src/commands/oxidize/trainer.rs`

```rust
use ndarray::{Array1, Array2, Axis};

pub struct Projection {
    pub w1: Array2<f32>,  // [hidden_dim, input_dim]
    pub b1: Array1<f32>,
    pub w2: Array2<f32>,  // [output_dim, hidden_dim]
    pub b2: Array1<f32>,
}
```

Contiguous row-major memory. In the default deterministic configuration, ndarray uses its normal pure-Rust path over contiguous arrays. That still removes pointer-chasing and simplifies the math substantially.

### 3. Rewrite forward pass

```rust
pub fn forward(&self, input: &Array1<f32>) -> Array1<f32> {
    let z1 = self.w1.dot(input) + &self.b1;
    let h1 = z1.mapv(|z| z.max(0.0));  // ReLU
    self.w2.dot(&h1) + &self.b2
}
```

One line per layer. The main win is contiguous storage and ndarray's optimized dense ops, not mandatory BLAS linkage.

### 4. Rewrite backprop with ndarray ops and pre-update gradients

The weight update currently does 6 nested loops (W1, W2, across 3 branches). With ndarray:

```rust
// dL/dW2 = dL_dout_a * h1_a^T + dL_dout_p * h1_p^T + dL_dout_n * h1_n^T
let grad_w2 = dl_dout_a.view().insert_axis(Axis(1)).dot(&h1_a.view().insert_axis(Axis(0)))
            + dl_dout_p.view().insert_axis(Axis(1)).dot(&h1_p.view().insert_axis(Axis(0)))
            + dl_dout_n.view().insert_axis(Axis(1)).dot(&h1_n.view().insert_axis(Axis(0)));
```

Similar for W1. The implementation must compute `grad_w2`, `grad_b2`, `dl_dh1`, `grad_w1`, and `grad_b1` against the original weights, then apply updates afterward. Do not mutate `self.w2` before `dl_dh1` is computed.

Use preallocated arrays or in-place accumulation where practical. The goal is not just prettier math syntax; it is less pointer chasing without replacing it with large per-triplet temporary allocations.

### 5. Fix safetensors save/load

Current save uses `unsafe` pointer casts from `Vec<Vec<f32>>`. With `Array2`, data is already contiguous:

```rust
use zerocopy::AsBytes;

let w1_slice = self.w1.as_slice_memory_order().expect("w1 must be contiguous");
let w1_bytes: &[u8] = w1_slice.as_bytes();
```

Use `zerocopy::AsBytes` (already in deps) on `as_slice_memory_order()` output for safe `&[f32]` → `&[u8]` byte views. `as_slice_memory_order()` for `Array2` (contiguous regardless of layout), `as_slice()` for `Array1`.

### 6. Update ForwardCache

Cache stores `Vec<f32>` — switch to `Array1<f32>` to avoid conversion overhead between forward and backward passes.

### 7. Adapt callers

`forward()` is called from `build_projection_index` in `oxidize/mod.rs` with `&[f32]` input. Keep a `&[f32] -> Vec<f32>` public entry point for callers, and use `ArrayView1<'_, f32>` internally so borrowed callers do not pay avoidable conversion costs.

### 8. Add determinism coverage

This refactor must preserve the build contract:

- same seeded initialization
- same sample order
- same epoch order
- same save order in safetensors
- same outputs across repeated runs on the same machine/config

Add a test that trains twice on the same small fixture corpus and asserts:

- loss history matches exactly
- saved safetensors bytes match exactly

## Risks

- **Determinism regression**: enabling BLAS or multithreaded math by default would weaken the current reproducibility goal for `oxidize`. Keep the deterministic path as the only default in this spec.
- **Temporary allocation churn**: naive ndarray expressions can allocate intermediate arrays in the hot loop. Prefer views, mutable buffers, and staged updates.
- **API surface**: `Projection::forward` currently takes `&[f32]` and returns `Vec<f32>`. Preserve a simple caller-facing API even if the internals use `ArrayView1`/`Array1`.

## Exit Criteria

1. Projection weights stored as `Array2<f32>` not `Vec<Vec<f32>>`
2. Forward pass uses ndarray `dot()` over contiguous arrays
3. Backprop uses ndarray ops for weight gradient accumulation
4. Save/load safetensors works with contiguous `Array2` without unsafe pointer casts
5. Gradient computation uses pre-update weights and fixes the current `w2` backprop bug
6. Repeated training on the same input yields identical losses and identical saved weights
7. Existing tests pass — `cargo test`
8. DuckDB dependency projection trains measurably faster (emit timing via `patina::measure`)
