# Spec: Model Strategy - Open Source Embeddings

**Status:** Research Complete (2025-11-23)
**Session:** [20251123-222456](../sessions/20251123-222456.md)
**Context:** Open model ecosystem exploration for Apple Silicon integration

## Executive Summary

Patina's embedding strategy shifts from closed/legacy models to the rapidly evolving open source ecosystem, with special focus on Apple Silicon optimization. Current E5-base-v2 (2022, 768 dims) will be supplemented with Qwen3-Embedding series and MLX runtime support for Mac-native performance.

**Key Decision:** Multi-runtime architecture supporting both cross-platform ONNX and Mac-optimized MLX, allowing Patina to leverage the best open models as they emerge.

---

## Strategic Context

### The Open Model Explosion (2025)

From Nathan Lambert's PyTorch Conference talk on "State of Open Models":

> "After DeepSeek in January 2025, we've seen relentless releases from Chinese labs. Qwen cumulative downloads passed Llama. The summer of 2025 is when the ecosystem really flipped."

**Key Trends:**
1. **Chinese labs dominating:** Qwen, DeepSeek, MiniMax, GLM releasing SOTA models monthly
2. **Qwen ecosystem growth:** 32 open-source models, MLX-optimized, 100+ languages
3. **Local-first movement:** Models sized for consumer hardware (0.6B-8B parameters)
4. **Code specialization:** Qwen2.5-Coder, DeepSeek-Coder outperforming general models on code tasks

**Relevance to Patina:**
- Patina scrapes code, sessions, and git history → **code-aware embeddings critical**
- Cross-platform Rust tool → need portable runtime (ONNX) + Mac optimization (MLX)
- User hardware: Mac Studio M2 Max 96GB → perfect for 0.6B-8B models
- Fast-moving ecosystem → architecture must support model swapping

### Why Change from E5-base-v2?

**Current State:**
- Model: E5-base-v2 (Microsoft, 2022)
- Dimensions: 768
- MTEB Score: 83-85%
- Strengths: Proven, stable, good general-purpose performance
- Runtime: ONNX (cross-platform)

**Limitations:**
- **No code specialization:** Trained on general text, not optimized for code understanding
- **Limited language support:** English-focused, weak on multilingual code comments
- **Legacy architecture:** Pre-dates 2024-2025 embedding innovations (Matryoshka, instruction-aware)
- **No Mac optimization:** ONNX doesn't leverage Metal GPU/Neural Engine fully

**Open Model Advantages:**
- **Code-aware training:** Qwen3-Embedding trained on 100+ programming languages
- **Multilingual:** Same model handles English docs, Chinese comments, Spanish README
- **Instruction-aware:** Can be tuned for specific tasks (search, classification, clustering)
- **Flexible dimensions:** Matryoshka-style embeddings (truncate 4096→1024→256 without retraining)
- **Rapid iteration:** New models monthly vs. yearly for closed models

---

## Model Comparison Matrix

| Model | Released | Dims | MTEB | Code Support | Languages | ONNX | MLX | Size (INT8) | Notes |
|-------|----------|------|------|--------------|-----------|------|-----|-------------|-------|
| **E5-base-v2** (current) | 2022-12 | 768 | 83-85% | General | English | ✅ | ❌ | ~105MB | Proven baseline |
| **BGE-base-en-v1.5** | 2023-09 | 768 | ~63% | General | English | ✅ | ❌ | ~105MB | Langchain default |
| **Nomic-Embed-v1.5** | 2024-02 | 768 | ~62% | General | English | ✅ | ⚠️ | ~137MB | 8192 token context |
| **Qwen3-Embed-0.6B** | 2025-06 | 1,024 | ~65% | ✅ Strong | 100+ | ⚠️ Community | ✅ | ~500MB | **Recommended start** |
| **Qwen3-Embed-4B** | 2025-06 | 2,560 | ~68% | ✅ Strong | 100+ | ⚠️ Community | ✅ | ~2GB | Production target |
| **Qwen3-Embed-8B** | 2025-06 | 4,096 | **70.58%** 🥇 | ✅ Strong | 100+ | ❌ | ✅ | ~4GB | MTEB multilingual #1 |
| **OLMo 3-7B** | 2025-06 | N/A* | N/A | ✅ Strong | English | ✅ | ✅ | ~7GB | Fully open (data+code) |

**Legend:**
- ✅ Official support
- ⚠️ Community export (not officially tested)
- ❌ Not available
- N/A* = Full LLM, not dedicated embedding model (would extract hidden states)

**MTEB Benchmark Context:**
- MTEB = Massive Text Embedding Benchmark (English)
- MMTEB = Multilingual MTEB (216 tasks, 250+ languages)
- Higher is better, but code-specific performance not captured in general benchmarks

---

## Architecture: Multi-Runtime Support

### Design Principle

**Patina's existing `EmbeddingEngine` trait already supports multi-runtime architecture.** No refactor needed - just add implementations.

```rust
// src/embeddings/mod.rs (EXISTING - no changes)
pub trait EmbeddingEngine {
    fn embed(&mut self, text: &str) -> Result<Vec<f32>>;
    fn embed_query(&mut self, text: &str) -> Result<Vec<f32>>;    // Asymmetric models
    fn embed_passage(&mut self, text: &str) -> Result<Vec<f32>>;  // Asymmetric models
    fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;
    fn model_name(&self) -> &str;
}
```

**Key Insight:** Trait abstraction means projection training code (`oxidize`) never needs to know about runtimes. Swap ONNX↔MLX without changing training logic.

### Runtime Comparison

| Runtime | Platform | Performance | Ecosystem | Patina Status |
|---------|----------|-------------|-----------|---------------|
| **ONNX** | Cross-platform (Mac/Linux/Windows) | Moderate (CPU/CoreML) | Mature (2017+) | ✅ **Implemented** |
| **MLX** | macOS only (Apple Silicon) | **Fast** (Metal GPU/NE) | Cutting-edge (2023+) | 🚧 **Planned** |

**ONNX Runtime Details:**
- Execution providers: CPU, CoreML (Mac), CUDA (Linux), DirectML (Windows)
- Current Patina usage: CPU only
- Potential: CoreML backend available but not leveraged
- Rust support: `ort` crate (v2.0, actively maintained)

**MLX Runtime Details:**
- Apple's official ML framework for Apple Silicon
- Native Metal GPU + Neural Engine acceleration
- Unified memory architecture (CPU/GPU share 96GB)
- Rust support: `mlx-rs` v0.25 (Feb 2025, production-ready)
- C API available: `mlx-c` for FFI if needed

### Recommended Strategy: Hybrid

```
┌─────────────────────────────────────────┐
│  User's Hardware & Use Case             │
└─────────────────────────────────────────┘
                 │
                 ├─ Mac (Apple Silicon)
                 │  └─> MLX runtime (fast)
                 │      ├─ Qwen3-Embed-4B (2560 dims)
                 │      └─ Qwen3-Embed-8B (4096 dims)
                 │
                 └─ Linux/Windows/Mac (Intel)
                    └─> ONNX runtime (portable)
                        ├─ E5-base-v2 (768 dims)
                        ├─ BGE-base-en-v1.5 (768 dims)
                        └─ Qwen3-Embed-0.6B (1024 dims)
```

**Configuration:**

```yaml
# .patina/oxidize.yaml
version: 2
embedding_model: qwen3-embedding-0.6b
runtime: auto  # Options: auto | onnx | mlx

projections:
  semantic:
    layers: [1024, 1536, 256]  # Auto-adapts to model dimensions
    epochs: 10
```

**Runtime Selection Logic:**
1. `runtime: auto` → Detect platform, prefer MLX on macOS if available
2. `runtime: mlx` → Fail if not macOS Apple Silicon
3. `runtime: onnx` → Cross-platform fallback

---

## Implementation Phases

### Phase 2A: Add Qwen3-Embedding-0.6B (ONNX)
**Effort:** 1-2 days
**Risk:** Low (uses existing infrastructure)

**Goals:**
- Validate Qwen3 quality on Patina's code/session data
- Test 1024-dim embeddings in projection pipeline
- Prove model swapping works without oxidize refactor

**Tasks:**
1. Download `onnx-community/Qwen3-Embedding-0.6B-ONNX` from HuggingFace
2. Add to `resources/models/registry.toml`:
   ```toml
   [models.qwen3-embedding-0-6b]
   name = "Qwen3-Embedding-0.6B"
   description = "Qwen3 multilingual embedding, code-aware (1024 dims)"
   path = "resources/models/qwen3-embedding-0.6b"
   dimensions = 1024
   metric = "cosine"
   source = "https://huggingface.co/Qwen/Qwen3-Embedding-0.6B"
   use_case = "Code retrieval, multilingual semantic search"
   performance = "MTEB ~65%, 100+ languages including code"
   size_int8 = "~500MB"
   maturity = "Cutting-edge (June 2025)"
   rust_support = "Excellent (ONNX community export)"
   runtimes = ["onnx"]
   download_quantized = "https://huggingface.co/onnx-community/Qwen3-Embedding-0.6B-ONNX/resolve/main/model_quantized.onnx"
   download_tokenizer = "https://huggingface.co/onnx-community/Qwen3-Embedding-0.6B-ONNX/resolve/main/tokenizer.json"
   ```

3. Test with existing `OnnxEmbedder` (no code changes needed)
4. Update recipe: `embedding_model: qwen3-embedding-0.6b`
5. Run `patina oxidize` with `layers: [1024, 1536, 256]`
6. Benchmark vs E5-base-v2:
   - Code search quality (HumanEval-style queries)
   - Session similarity (same_session pairs)
   - Inference speed (embeddings/sec)

**Success Criteria:**
- [ ] Qwen3-0.6B loads via ONNX
- [ ] 1024-dim embeddings generated successfully
- [ ] Projection training completes (768→1024→256 becomes 1024→1536→256)
- [ ] Code queries show improvement over E5-base-v2

### Phase 2B: Add MLX Runtime
**Effort:** 3-5 days
**Risk:** Moderate (new dependency, macOS-only testing)

**Goals:**
- Enable Mac-native performance (2-3x speedup expected)
- Support Qwen3-Embed-4B/8B models (too large for efficient ONNX)
- Validate MLX as production-ready for embeddings

**Tasks:**
1. **Add dependencies:**
   ```toml
   [dependencies]
   mlx-rs = { version = "0.25", optional = true }

   [features]
   default = []
   mlx = ["mlx-rs"]  # macOS-only feature
   ```

2. **Implement `MlxEmbedder`:**
   ```rust
   // src/embeddings/mlx.rs (NEW)
   use mlx_rs::{Array, Model};
   use super::EmbeddingEngine;

   pub struct MlxEmbedder {
       model: Model,
       tokenizer: Tokenizer,
       dimension: usize,
       model_name: String,
       query_prefix: Option<String>,
       passage_prefix: Option<String>,
   }

   impl MlxEmbedder {
       pub fn new_from_hf(
           model_id: &str,  // e.g., "Qwen/Qwen3-Embedding-0.6B"
           dimension: usize,
           query_prefix: Option<String>,
           passage_prefix: Option<String>,
       ) -> Result<Self> {
           // Load MLX model from HuggingFace
           let model = Model::from_pretrained(model_id)?;
           let tokenizer = Tokenizer::from_pretrained(model_id)?;

           Ok(Self {
               model,
               tokenizer,
               dimension,
               model_name: model_id.to_string(),
               query_prefix,
               passage_prefix,
           })
       }
   }

   impl EmbeddingEngine for MlxEmbedder {
       fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
           // Tokenize
           let tokens = self.tokenizer.encode(text, true)?;
           let input_array = Array::from_slice(&tokens);

           // Forward pass
           let output = self.model.forward(&input_array)?;

           // Extract [EOS] token embedding (Qwen3 convention)
           let eos_embedding = output.last()?;

           // Convert to Vec<f32>
           Ok(eos_embedding.to_vec())
       }

       fn embed_query(&mut self, text: &str) -> Result<Vec<f32>> {
           // Qwen3-Embedding supports instruction-aware queries
           // For now, simple prefix application
           let input = if let Some(prefix) = &self.query_prefix {
               format!("{}{}", prefix, text)
           } else {
               text.to_string()
           };
           self.embed(&input)
       }

       fn embed_passage(&mut self, text: &str) -> Result<Vec<f32>> {
           let input = if let Some(prefix) = &self.passage_prefix {
               format!("{}{}", prefix, text)
           } else {
               text.to_string()
           };
           self.embed(&input)
       }

       fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
           // MLX supports efficient batching via unified memory
           texts.iter().map(|t| self.embed(t)).collect()
       }

       fn dimension(&self) -> usize {
           self.dimension
       }

       fn model_name(&self) -> &str {
           &self.model_name
       }
   }
   ```

3. **Update factory function:**
   ```rust
   // src/embeddings/mod.rs
   fn create_embedder_from_config() -> Result<Box<dyn EmbeddingEngine>> {
       let config = Config::load()?;
       let model_def = config.get_model_definition()?;

       // Determine runtime (auto-detect or explicit)
       let runtime = config.embeddings.runtime.as_deref().unwrap_or("auto");

       let selected_runtime = match runtime {
           "auto" => {
               #[cfg(all(target_os = "macos", target_arch = "aarch64", feature = "mlx"))]
               {
                   if model_def.runtimes.contains(&"mlx".to_string()) {
                       "mlx"
                   } else {
                       "onnx"
                   }
               }
               #[cfg(not(all(target_os = "macos", target_arch = "aarch64", feature = "mlx")))]
               {
                   "onnx"
               }
           }
           explicit => explicit,
       };

       match selected_runtime {
           "onnx" => create_onnx_embedder(&model_def),
           #[cfg(feature = "mlx")]
           "mlx" => create_mlx_embedder(&model_def),
           _ => Err(anyhow!("Unsupported runtime: {}", selected_runtime)),
       }
   }

   #[cfg(feature = "mlx")]
   fn create_mlx_embedder(model_def: &ModelDefinition) -> Result<Box<dyn EmbeddingEngine>> {
       // MLX loads directly from HuggingFace Hub
       let hf_id = model_def.source
           .trim_start_matches("https://huggingface.co/")
           .to_string();

       Ok(Box::new(MlxEmbedder::new_from_hf(
           &hf_id,
           model_def.dimensions,
           model_def.query_prefix.clone(),
           model_def.passage_prefix.clone(),
       )?))
   }
   ```

4. **Add Qwen3-Embed-4B to registry:**
   ```toml
   [models.qwen3-embedding-4b]
   name = "Qwen3-Embedding-4B"
   description = "Qwen3 multilingual embedding, production-grade (2560 dims)"
   path = "resources/models/qwen3-embedding-4b"
   dimensions = 2560
   metric = "cosine"
   source = "https://huggingface.co/Qwen/Qwen3-Embedding-4B"
   use_case = "Production code retrieval, cross-lingual semantic search"
   performance = "MTEB ~68%, 100+ languages including code"
   size_fp16 = "~8GB"
   maturity = "Cutting-edge (June 2025, MTEB multilingual top-5)"
   rust_support = "Excellent (MLX official)"
   runtimes = ["mlx"]  # Too large for efficient ONNX on most systems
   ```

5. **Benchmark MLX vs ONNX:**
   - Inference speed: embeddings/sec on M2 Max
   - Training speed: projection epochs/hour
   - Memory usage: unified memory advantage
   - Model quality: identical output verification

**Success Criteria:**
- [ ] `cargo build --features mlx` succeeds on macOS
- [ ] Qwen3-Embed-4B loads via MLX
- [ ] 2560-dim embeddings generated successfully
- [ ] MLX shows 2-3x speedup over ONNX on M2 Max
- [ ] Cross-platform: ONNX still works for non-Mac users

### Phase 2C: Multi-Model Ecosystem
**Effort:** 2-3 days
**Risk:** Low (validates architecture scales)

**Goals:**
- Prove model swapping is seamless
- Document model selection guide
- Support both legacy (E5, BGE) and cutting-edge (Qwen3) models

**Tasks:**
1. Add remaining models to registry:
   - Qwen3-Embedding-8B (MLX only)
   - Keep E5-base-v2, BGE-base-en-v1.5 (ONNX)
   - Add Nomic-Embed-v1.5 (ONNX, optional MLX)

2. Test cross-runtime workflows:
   ```bash
   # Developer A (Mac Studio M2 Max 96GB)
   embedding_model: qwen3-embedding-4b
   runtime: mlx

   # Developer B (Linux Threadripper 64GB)
   embedding_model: qwen3-embedding-0.6b
   runtime: onnx

   # Same recipe version, different runtimes
   # Both build projections from same eventlog
   # Verify semantic similarity convergence
   ```

3. Write model selection guide (`layer/surface/guide-model-selection.md`):
   - Decision tree: Hardware → Use case → Model
   - Dimension tradeoffs (384 vs 768 vs 1024 vs 2560)
   - Runtime selection criteria
   - Migration path from E5→Qwen3

4. Update documentation:
   - `CLAUDE.md`: Add MLX feature flag instructions
   - `build.md`: Phase 2 multi-runtime completion
   - `spec-oxidize.md`: Runtime field in recipe format

**Success Criteria:**
- [ ] 5+ models in registry (E5, BGE, Nomic, Qwen3-0.6B/4B/8B)
- [ ] Model swapping requires only recipe change
- [ ] Documentation guides users to right model/runtime
- [ ] CI tests both ONNX (Linux) and MLX (if macOS runner available)

---

## Open Source Model Principles

### 1. **Ecosystem Over Lock-In**

Patina embraces the Linux philosophy: choice over dictatorship. Support multiple runtimes and models so users aren't locked to a single vendor's roadmap.

**Anti-Pattern:** Hard-coded OpenAI API calls
**Patina Pattern:** Trait-based abstraction with pluggable backends

### 2. **Local-First, Cloud-Optional**

Open models enable fully local workflows. No API keys, no rate limits, no vendor shutdowns.

**Example:** User with M2 Max can run Qwen3-Embed-8B (4096 dims) locally at 100+ embeddings/sec. Same workflow works offline on airplane.

### 3. **Transparency Over Black Boxes**

Models with public training data (OLMo) and inference code (MLX) allow debugging and improvement. Patina's oxidize pipeline benefits from inspectable model internals.

**Future:** OLMo 3 integration could enable training data tracing via OlmoTrace (which training examples influenced this embedding?)

### 4. **Community Over Corporate Roadmaps**

Qwen releases 32 models in 6 months. Meta releases Llama once per year. Open community iterates faster.

**Patina Benefit:** New code embedding models (e.g., Qwen2.5-Coder) can be integrated within days of release.

### 5. **Hardware-Aware Performance**

MLX leverages M2 Max's unified memory (96GB shared CPU/GPU). ONNX works everywhere but optimizes for nothing.

**Philosophy:** Match runtime to user's hardware. Don't force Linux users to use macOS frameworks or vice versa.

---

## Hardware Target: Mac Studio M2 Max

**User's Specs:**
- Chip: Apple M2 Max
- Memory: 96GB unified
- Architecture: ARM64 (Apple Silicon)
- GPU: 38-core (integrated)
- Neural Engine: 16-core

**Implications for Model Selection:**

| Model | Params | Size (FP16) | Fits in 96GB? | Inference Speed (est.) | Recommended |
|-------|--------|-------------|---------------|------------------------|-------------|
| Qwen3-Embed-0.6B | 600M | ~1.2GB | ✅ Yes | ~200 embed/sec | ✅ Start here |
| Qwen3-Embed-4B | 4B | ~8GB | ✅ Yes | ~100 embed/sec | ✅ Production |
| Qwen3-Embed-8B | 8B | ~16GB | ✅ Yes | ~50 embed/sec | ✅ Max quality |
| OLMo 3-32B | 32B | ~64GB | ⚠️ Tight | ~10 tokens/sec | ⚠️ Not embedding model |

**Recommended Configuration:**
```yaml
# .patina/oxidize.yaml (Mac Studio optimized)
version: 2
embedding_model: qwen3-embedding-4b  # Sweet spot: quality + speed
runtime: mlx  # Mac-native Metal acceleration

projections:
  semantic:
    layers: [2560, 3072, 512]  # 2560 from Qwen3-4B
    epochs: 20
    batch_size: 64  # Large batch fits in 96GB unified memory
```

**Performance Expectations (MLX on M2 Max):**
- Embedding generation: ~100 embeddings/sec (vs ~30/sec ONNX)
- Projection training: ~50K pairs in 10-15 minutes (vs 30-40 minutes ONNX)
- Memory usage: ~12GB for model + ~4GB for projection weights + ~2GB batch data = ~18GB total

---

## Migration Path

### For Existing Patina Users

**Step 1: Stay on E5-base-v2 (No Changes)**
```yaml
# .patina/oxidize.yaml
version: 1
embedding_model: e5-base-v2  # Existing default
runtime: onnx  # Explicit (was implicit before)
```

**Step 2: Upgrade to Qwen3-0.6B (ONNX, Low Risk)**
```yaml
version: 2
embedding_model: qwen3-embedding-0.6b
runtime: onnx  # Still cross-platform

projections:
  semantic:
    layers: [1024, 1536, 256]  # 768→1024 dimension bump
```

**Step 3: Enable MLX (Mac Users Only)**
```bash
# Rebuild with MLX feature
cargo build --release --features mlx
cargo install --path . --features mlx
```

```yaml
version: 2
embedding_model: qwen3-embedding-4b  # Bigger model now feasible
runtime: mlx  # Mac-native speed
```

**Step 4: Re-train Projections**
```bash
patina oxidize --full  # Rebuild all projections from scratch
```

**Note:** Different embedding dimensions require re-training projections. Bumping 768→1024→2560 invalidates old projection weights. This is expected and desired (better base embeddings = better projections).

---

## Testing Strategy

### Unit Tests

```rust
// src/embeddings/mlx.rs
#[cfg(all(test, feature = "mlx"))]
mod tests {
    use super::*;

    #[test]
    fn test_mlx_embedder_creation() {
        let embedder = MlxEmbedder::new_from_hf(
            "Qwen/Qwen3-Embedding-0.6B",
            1024,
            None,
            None,
        ).expect("Model should load");

        assert_eq!(embedder.dimension(), 1024);
    }

    #[test]
    fn test_mlx_embed_basic() {
        let mut embedder = get_test_embedder();
        let embedding = embedder.embed("fn main() {}").unwrap();

        assert_eq!(embedding.len(), 1024);

        // Check L2 normalization
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_mlx_code_similarity() {
        let mut embedder = get_test_embedder();

        let rust_fn = embedder.embed("fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();
        let python_fn = embedder.embed("def add(a, b): return a + b").unwrap();
        let weather = embedder.embed("The weather is nice today").unwrap();

        let sim_rust_python = cosine_similarity(&rust_fn, &python_fn);
        let sim_rust_weather = cosine_similarity(&rust_fn, &weather);

        // Code should be more similar across languages than unrelated text
        assert!(sim_rust_python > sim_rust_weather);
    }
}
```

### Integration Tests

```rust
// tests/multi_runtime_test.rs
#[test]
fn test_onnx_mlx_convergence() {
    // Load same model via different runtimes
    let mut onnx_embedder = create_onnx_embedder("qwen3-embedding-0.6b")?;

    #[cfg(feature = "mlx")]
    let mut mlx_embedder = create_mlx_embedder("qwen3-embedding-0.6b")?;

    let test_text = "fn fibonacci(n: u32) -> u32";

    let onnx_embed = onnx_embedder.embed(test_text)?;
    #[cfg(feature = "mlx")]
    let mlx_embed = mlx_embedder.embed(test_text)?;

    // Embeddings should be nearly identical (allow for floating-point variance)
    #[cfg(feature = "mlx")]
    {
        let similarity = cosine_similarity(&onnx_embed, &mlx_embed);
        assert!(similarity > 0.999, "ONNX and MLX should produce same embeddings");
    }
}
```

### Benchmark Suite

```bash
# scripts/benchmark-models.sh
#!/bin/bash

MODELS=("e5-base-v2" "qwen3-embedding-0.6b" "qwen3-embedding-4b")
QUERIES=(
    "how do i handle errors in rust?"
    "fn main() { println!(\"Hello\"); }"
    "git commit best practices"
)

for model in "${MODELS[@]}"; do
    echo "Benchmarking $model..."

    # Update config
    sed -i '' "s/embedding_model: .*/embedding_model: $model/" .patina/oxidize.yaml

    # Measure embedding speed
    time patina oxidize --dry-run

    # Test query quality
    for query in "${QUERIES[@]}"; do
        patina scry "$query" --model $model --top 5
    done
done
```

---

## Future Directions

### 1. Instruction-Tuned Embeddings

Qwen3-Embedding supports custom instructions:
```python
# Example from Qwen docs (Python, for reference)
instruction = "Given a code function, retrieve semantically similar functions"
query_embedding = model.encode(query, instruction=instruction)
```

**Patina Integration:**
```rust
// Future: src/embeddings/mod.rs
pub trait InstructionAwareEmbedder: EmbeddingEngine {
    fn embed_with_instruction(&mut self, text: &str, instruction: &str) -> Result<Vec<f32>>;
}

// Config: .patina/oxidize.yaml
projections:
  semantic:
    instruction: "Given a code function, retrieve semantically similar functions"
```

### 2. Matryoshka Embeddings (Truncatable Dimensions)

Newer models support dimension truncation without retraining:
```rust
// Full embedding: 4096 dims
let full_embed = embedder.embed(text)?;

// Truncate to 1024 dims (8x smaller index, minor quality loss)
let truncated_embed = &full_embed[..1024];
```

**Use Case:** Store multiple index sizes:
- `semantic.usearch` (4096 dims, best quality)
- `semantic-fast.usearch` (1024 dims, 8x faster search)

### 3. OLMo Integration (Fully Open)

OLMo 3 is the only truly open LLM (training data + code + checkpoints):
```yaml
# Future: .patina/oxidize.yaml
embedding_model: olmo-3-7b
extraction_layer: -2  # Use second-to-last transformer layer
pooling: mean  # Mean-pool token embeddings
```

**Benefit:** OlmoTrace can show which training examples influenced embeddings.

### 4. Multi-Modal Embeddings (Code + Text + Diagrams)

Future models may embed code, documentation, and architecture diagrams into shared space:
```rust
// Speculative future API
embedder.embed_code("fn main() {}")?;
embedder.embed_text("This function is the entry point")?;
embedder.embed_image("architecture-diagram.png")?;
// All return Vec<f32> in same semantic space
```

---

## Open Questions

1. **Model licensing for distribution?**
   - Qwen3: Apache 2.0 ✅
   - OLMo: Apache 2.0 ✅
   - Can we bundle quantized models with Patina releases? (Probably yes, but verify)

2. **ONNX export reliability for Qwen3?**
   - Community exports exist but not officially supported
   - Should we contribute official ONNX export to Qwen team?

3. **MLX backward compatibility?**
   - `mlx-rs` v0.25 is recent (Feb 2025)
   - What's the API stability guarantee? Need to track upstream changes?

4. **Hybrid runtime for partial batches?**
   - MLX for large batches (training)
   - ONNX for single queries (low latency)
   - Worth the complexity?

5. **Fine-tuning on Patina data?**
   - Could we fine-tune Qwen3-Embed on scraped code/sessions?
   - Would custom embedding model outperform projection layers?

---

## References

### Talks & Papers

- **Nathan Lambert - "State of Open Models"** (PyTorch Conference 2025)
  - Video: https://www.youtube.com/watch?v=WfwtvzouZGA
  - Key insight: Qwen downloads surpassed Llama in summer 2025

- **Qwen3 Embedding Paper** (Alibaba, June 2025)
  - Paper: https://arxiv.org/abs/2506.05176
  - Blog: https://qwenlm.github.io/blog/qwen3-embedding/
  - MTEB multilingual #1 (70.58 score)

- **OLMo 3 Blog Post** (AI2, June 2025)
  - https://allenai.org/blog/olmo3
  - Fully open LLM with training data + code

### Model Repositories

- **Qwen3-Embedding Series:** https://huggingface.co/Qwen
  - Qwen3-Embedding-0.6B / 4B / 8B
  - ONNX exports: https://huggingface.co/onnx-community

- **E5-base-v2:** https://huggingface.co/intfloat/e5-base-v2
  - Current baseline

- **MLX Models:** https://huggingface.co/mlx-community
  - MLX-converted versions of popular models

### Tools & Frameworks

- **MLX Framework:** https://github.com/ml-explore/mlx
  - Apple's official ML framework for Apple Silicon

- **mlx-rs:** https://github.com/oxideai/mlx-rs
  - Rust bindings for MLX

- **ONNX Runtime:** https://onnxruntime.ai/
  - Cross-platform inference engine

- **ort crate:** https://crates.io/crates/ort
  - Rust bindings for ONNX Runtime

---

## Appendix: Example Configurations

### Minimal (Cross-Platform, 384 dims)
```yaml
# .patina/oxidize.yaml
version: 1
embedding_model: all-minilm-l6-v2
runtime: onnx

projections:
  semantic:
    layers: [384, 512, 128]
    epochs: 10
```

### Recommended (Mac Studio, Code-Aware, 1024 dims)
```yaml
version: 2
embedding_model: qwen3-embedding-0.6b
runtime: auto  # Detects Mac → uses MLX if available

projections:
  semantic:
    layers: [1024, 1536, 256]
    epochs: 20

  temporal:
    layers: [1024, 1536, 256]
    epochs: 15
```

### High-End (Mac Studio, Production, 2560 dims)
```yaml
version: 2
embedding_model: qwen3-embedding-4b
runtime: mlx

projections:
  semantic:
    layers: [2560, 3072, 512]
    epochs: 30

  temporal:
    layers: [2560, 3072, 512]
    epochs: 20

  dependency:
    layers: [2560, 3072, 512]
    epochs: 20
```

### Research (Fully Open, Traceable)
```yaml
version: 2
embedding_model: olmo-3-7b  # Future
runtime: mlx
extraction_layer: -2
pooling: mean

projections:
  semantic:
    layers: [4096, 5120, 1024]  # OLMo hidden size
    epochs: 50
    traceable: true  # Enable OlmoTrace integration
```

---

**Status:** Spec complete, ready for Phase 2A implementation.
**Next Steps:** Begin Qwen3-Embedding-0.6B ONNX integration, validate quality on Patina codebase.
