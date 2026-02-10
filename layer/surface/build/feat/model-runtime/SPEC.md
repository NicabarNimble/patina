---
type: feat
id: model-runtime
status: ready
created: 2026-01-13
updated: 2026-02-06
blocked_by:
  - eval-repair
related:
  - layer/surface/build/fix/eval-repair/SPEC.md
  - layer/surface/build/explore/lab-automation/SPEC.md
---

# feat: Model & Runtime Strategy

> Model upgrades (E5 → Qwen3) and MLX runtime for Mac-native GPU acceleration.

**Key Principle:** "Don't optimize what you can't measure." Model swapping invalidates ALL trained projections, so only upgrade after eval infrastructure can measure the impact.

**Blocked by:** eval-repair (need working eval to measure model swap impact)

---

## Current State

| Component | Value |
|-----------|-------|
| Model | E5-base-v2 (Microsoft, 2022) |
| Dimensions | 768 |
| Runtime | ONNX (cross-platform) |
| Validation | +68% vs baseline on real session data |

**Why E5 works:** Asymmetric query/passage prefixes match Q&A pattern. Training includes Stack Overflow-style Q&A. Validated empirically on Patina's actual data.

## Architecture

`EmbeddingEngine` trait already supports multi-runtime — no refactor needed:

```rust
pub trait EmbeddingEngine {
    fn embed(&mut self, text: &str) -> Result<Vec<f32>>;
    fn embed_query(&mut self, text: &str) -> Result<Vec<f32>>;
    fn embed_passage(&mut self, text: &str) -> Result<Vec<f32>>;
    fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;
    fn model_name(&self) -> &str;
}
```

## Model Options

| Model | Dims | Runtime | Use Case |
|-------|------|---------|----------|
| E5-base-v2 | 768 | ONNX | Current baseline (validated) |
| Qwen3-Embed-0.6B | 1024 | ONNX | Code-aware, low-risk upgrade |
| Qwen3-Embed-4B | 2560 | MLX | Production target (Mac) |
| Qwen3-Embed-8B | 4096 | MLX | Max quality (Mac Studio) |

## Phases

### Phase 1: Add Qwen3-0.6B (ONNX)

- [ ] Add to `resources/models/registry.toml`
- [ ] Test with existing `OnnxEmbedder`
- [ ] Benchmark vs E5-base-v2 on code/session queries
- [ ] Retrain all projections (1024-dim base)

### Phase 2: Add MLX Runtime

- [ ] Add `mlx-rs` with feature flag
- [ ] Implement `MlxEmbedder`
- [ ] Support Qwen3-Embed-4B/8B
- [ ] Benchmark MLX vs ONNX speed

## Key Decisions

1. **Hybrid runtime** — ONNX for cross-platform, MLX additive for Mac
2. **Model swap = retrain** — Different dimensions invalidate all projections
3. **Validate before upgrade** — E5 proven on real data; Qwen3 is speculative
4. **Registry-based** — Models defined in TOML, no code changes to swap

## Exit Criteria

- [ ] Qwen3-0.6B benchmarked against E5-base-v2 with eval-repair metrics
- [ ] Model swap does not regress NL query precision (eval-repair Phase 2 baseline)
- [ ] MLX runtime passes same benchmarks as ONNX on Apple Silicon

## References

- Session 20251116-194408: E5 benchmark (+68% vs baseline)
- Session 20251123-222456: MLX research, Qwen3 analysis
- `resources/models/registry.toml`: Model definitions
- `src/embeddings/mod.rs`: EmbeddingEngine trait
