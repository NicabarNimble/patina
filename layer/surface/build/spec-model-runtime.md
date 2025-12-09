# Spec: Model & Runtime Strategy

**Status:** Blocked until Phase 2.5 evaluation validates architecture
**Phase:** 6 (Future)

## Overview

Model upgrades and MLX runtime integration. Currently using E5-base-v2 via ONNX. Future: Qwen3 models with optional MLX for Mac-native performance.

**Key Principle:** "Don't optimize what you can't measure." Model swapping invalidates ALL trained projections, so only upgrade after evaluation proves current architecture valuable.

## Current State

| Component | Value |
|-----------|-------|
| Model | E5-base-v2 (Microsoft, 2022) |
| Dimensions | 768 |
| Runtime | ONNX (cross-platform) |
| Validation | +68% vs baseline on real session data |

**Why E5 works:**
- Asymmetric query/passage prefixes match Q&A pattern
- Training includes Stack Overflow-style Q&A
- Validated empirically on Patina's actual data (not just MTEB benchmarks)

## Architecture

Existing `EmbeddingEngine` trait already supports multi-runtime:

```rust
pub trait EmbeddingEngine {
    fn embed(&mut self, text: &str) -> Result<Vec<f32>>;
    fn embed_query(&mut self, text: &str) -> Result<Vec<f32>>;
    fn embed_passage(&mut self, text: &str) -> Result<Vec<f32>>;
    fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;  // Propagates to projections automatically
    fn model_name(&self) -> &str;
}
```

**No refactor needed** - just add `MlxEmbedder` implementation when ready.

## Future Model Options

| Model | Dims | Runtime | Use Case |
|-------|------|---------|----------|
| E5-base-v2 | 768 | ONNX | Current baseline (validated) |
| Qwen3-Embed-0.6B | 1024 | ONNX | Code-aware, low-risk upgrade |
| Qwen3-Embed-4B | 2560 | MLX | Production target (Mac) |
| Qwen3-Embed-8B | 4096 | MLX | Max quality (Mac Studio) |

**Qwen3 advantages:**
- Code-aware training (100+ programming languages)
- Multilingual (docs, comments, READMEs)
- MTEB multilingual #1 (8B model: 70.58%)

## Runtime Strategy

```
Platform Detection:
├── Mac (Apple Silicon) + --features mlx
│   └─> MLX runtime (Metal GPU, 2-3x faster)
└── Linux/Windows/Mac (Intel)
    └─> ONNX runtime (cross-platform)
```

**Recipe configuration:**
```yaml
# .patina/oxidize.yaml
embedding_model: qwen3-embedding-0.6b
runtime: auto  # Options: auto | onnx | mlx
```

## Implementation Phases

### Phase 4a: Add Qwen3-0.6B (ONNX)
**Blocked until:** Phase 2.5 evaluation complete
**Effort:** 1-2 days

- [ ] Add to `resources/models/registry.toml`
- [ ] Test with existing `OnnxEmbedder`
- [ ] Benchmark vs E5-base-v2 on code/session queries
- [ ] Retrain all projections (1024-dim base)

### Phase 4b: Add MLX Runtime
**Blocked until:** Phase 4a validates Qwen3 quality
**Effort:** 3-5 days

- [ ] Add `mlx-rs` with feature flag
- [ ] Implement `MlxEmbedder`
- [ ] Support Qwen3-Embed-4B/8B
- [ ] Benchmark MLX vs ONNX speed

## Key Decisions

1. **Hybrid runtime** - ONNX for cross-platform, MLX additive for Mac
2. **Model swap = retrain** - Different dimensions invalidate all projections
3. **Validate before upgrade** - E5 proven on real data; Qwen3 is speculative
4. **Registry-based** - Models defined in TOML, no code changes to swap

## References

- Session 20251116-194408: E5 benchmark (+68% vs baseline)
- Session 20251123-222456: MLX research, Qwen3 analysis
- `resources/models/registry.toml`: Model definitions
- `src/embeddings/mod.rs`: EmbeddingEngine trait
