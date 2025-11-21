# Spec: Progressive Adapters

## Overview
Progressive adapters extend frozen E5-base-v2 embeddings with trainable dimension-specific layers. This enables multidimensional code understanding without catastrophic forgetting.

## Architecture
```
┌─────────────────────────────────────────────────────────────┐
│  Input Text                                                  │
│  "function calculates fibonacci sequence recursively"        │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  E5-base-v2 (FROZEN)                                        │
│  768-dimensional base embedding                              │
└─────────────────────┬───────────────────────────────────────┘
                      │
        ┌─────────────┼─────────────┬─────────────┐
        ▼             ▼             ▼             ▼
┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐
│ Semantic  │  │ Temporal  │  │ Dependency│  │ ...       │
│ Adapter   │  │ Adapter   │  │ Adapter   │  │           │
│ 768→768   │  │ 768→256   │  │ 768→256   │  │           │
└─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘
      │              │              │              │
      └──────────────┴──────────────┴──────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  Concatenated Output: 2,304 dimensions                       │
│  [semantic:768 | temporal:256 | dependency:256 | ...]       │
└─────────────────────────────────────────────────────────────┘
```

## Dimension Adapters

| Adapter | Input | Output | Purpose | Training Signal |
|---------|-------|--------|---------|-----------------|
| Semantic | 768 | 768 | What code means | Session insights, documentation |
| Temporal | 768 | 256 | When code changed | Git commit sequences |
| Dependency | 768 | 256 | What code calls | Call graph edges |
| Syntactic | 768 | 256 | How code is structured | AST patterns |
| Architectural | 768 | 256 | Where code lives | File paths, module structure |
| Social | 768 | 256 | Who wrote/reviewed | Git blame, PR authors |

**Total output:** 768 + 256×5 = **2,048 dimensions** (or 2,304 with 768-dim semantic)

## Components

### 1. Training Pair Generators
**Location:** `src/adapters/training/`

Each adapter needs positive/negative pairs from project data:

```rust
// src/adapters/training/mod.rs
pub trait TrainingPairGenerator {
    fn generate_pairs(&self, project: &Project) -> Vec<TrainingPair>;
}

pub struct TrainingPair {
    pub anchor: String,      // The text to embed
    pub positive: String,    // Should be similar
    pub negative: String,    // Should be dissimilar
}
```

**Temporal Adapter Pairs:**
```rust
// src/adapters/training/temporal.rs
impl TrainingPairGenerator for TemporalGenerator {
    fn generate_pairs(&self, project: &Project) -> Vec<TrainingPair> {
        // Anchor: code snippet
        // Positive: code from same time period (within N commits)
        // Negative: code from distant time period
        let commits = git_log(project)?;
        // Generate pairs from commit sequences...
    }
}
```

**Dependency Adapter Pairs:**
```rust
// src/adapters/training/dependency.rs
impl TrainingPairGenerator for DependencyGenerator {
    fn generate_pairs(&self, project: &Project) -> Vec<TrainingPair> {
        // Anchor: function
        // Positive: functions it calls or is called by
        // Negative: unrelated functions
        let call_graph = load_call_graph(project)?;
        // Generate pairs from call edges...
    }
}
```

**Semantic Adapter Pairs (richest):**
```rust
// src/adapters/training/semantic.rs
impl TrainingPairGenerator for SemanticGenerator {
    fn generate_pairs(&self, project: &Project) -> Vec<TrainingPair> {
        // From sessions:
        // Anchor: session insight
        // Positive: code referenced in insight
        // Negative: unrelated code

        // From observations:
        // Anchor: observation content
        // Positive: code_refs from observation
        // Negative: random code
    }
}
```

### 2. Adapter Architecture
**Location:** `src/adapters/model/`

```rust
// src/adapters/model/adapter.rs
pub struct DimensionAdapter {
    pub name: String,
    pub input_dim: usize,   // 768 (E5 output)
    pub output_dim: usize,  // 768 or 256
    pub weights: Array2<f32>,
    pub bias: Array1<f32>,
}

impl DimensionAdapter {
    pub fn forward(&self, embedding: &[f32]) -> Vec<f32> {
        // Simple linear projection + ReLU
        let input = Array1::from_vec(embedding.to_vec());
        let output = self.weights.dot(&input) + &self.bias;
        output.mapv(|x| x.max(0.0)).to_vec()
    }

    pub fn train(&mut self, pairs: &[TrainingPair], epochs: usize) -> Result<()> {
        // Contrastive learning with triplet loss
        for epoch in 0..epochs {
            for pair in pairs {
                let anchor_emb = self.forward(&embed_base(&pair.anchor)?);
                let pos_emb = self.forward(&embed_base(&pair.positive)?);
                let neg_emb = self.forward(&embed_base(&pair.negative)?);

                let loss = triplet_loss(&anchor_emb, &pos_emb, &neg_emb);
                self.backward(&loss)?;
            }
        }
        Ok(())
    }
}
```

### 3. Adapter Registry
**Location:** `src/adapters/registry.rs`

```rust
pub struct AdapterRegistry {
    adapters: HashMap<String, DimensionAdapter>,
}

impl AdapterRegistry {
    pub fn load(path: &Path) -> Result<Self> {
        // Load all trained adapters from disk
    }

    pub fn embed_multidimensional(&self, text: &str) -> Result<Vec<f32>> {
        let base = embed_e5(text)?;  // 768-dim

        let mut output = Vec::new();
        for (name, adapter) in &self.adapters {
            output.extend(adapter.forward(&base));
        }
        Ok(output)  // 2,048 or 2,304 dim
    }
}
```

### 4. Dimension-Weighted Search
**Location:** `src/query/weighted.rs`

```rust
pub struct WeightedQuery {
    pub text: String,
    pub weights: DimensionWeights,
}

pub struct DimensionWeights {
    pub semantic: f32,    // default 1.0
    pub temporal: f32,    // default 0.5
    pub dependency: f32,  // default 0.5
    pub syntactic: f32,   // default 0.3
    pub architectural: f32, // default 0.3
    pub social: f32,      // default 0.1
}

pub fn weighted_search(query: &WeightedQuery, index: &USearchIndex) -> Vec<SearchResult> {
    let query_emb = embed_multidimensional(&query.text)?;

    // Apply weights to each dimension slice
    let weighted_emb = apply_weights(&query_emb, &query.weights);

    // Search with weighted embedding
    index.search(&weighted_emb, 10)
}
```

**CLI:**
```bash
# Default weights
patina query "error handling"

# Emphasize temporal (find recent similar code)
patina query --temporal 2.0 "error handling"

# Emphasize dependencies (find related functions)
patina query --dependency 2.0 "fibonacci"
```

### 5. Patina Thickness Model
**Location:** `src/adapters/thickness.rs`

Thickness determines training data richness:

| Stage | Data Sources | Adapter Quality |
|-------|--------------|-----------------|
| Bare | None | Base E5 only |
| Fresh | Git + code | Temporal, dependency basic |
| Working | + sessions | Semantic emerging |
| Mature | + persona beliefs | Full multidimensional |
| Ancient | + crystallized graphs | Meta-cognitive |

```rust
pub enum PatinaThickness {
    Bare,
    Fresh,
    Working,
    Mature,
    Ancient,
}

impl Project {
    pub fn thickness(&self) -> PatinaThickness {
        let session_count = count_sessions(&self.path);
        let event_count = count_events(&self.path);

        match (session_count, event_count) {
            (0, 0) => PatinaThickness::Bare,
            (0, _) => PatinaThickness::Fresh,
            (1..=10, _) => PatinaThickness::Working,
            (11..=50, _) => PatinaThickness::Mature,
            _ => PatinaThickness::Ancient,
        }
    }
}
```

## Training Pipeline

```bash
# Generate training pairs for a project
patina adapters generate-pairs --project .

# Train specific adapter
patina adapters train --adapter temporal --epochs 100

# Train all adapters
patina adapters train --all

# Evaluate adapter quality
patina adapters eval --adapter semantic
```

## Storage

```
~/.patina/
├── adapters/
│   ├── semantic.adapter      # Trained weights
│   ├── temporal.adapter
│   ├── dependency.adapter
│   ├── syntactic.adapter
│   ├── architectural.adapter
│   └── social.adapter
└── ...

<project>/.patina/
├── training/
│   ├── pairs/
│   │   ├── temporal_pairs.json
│   │   ├── dependency_pairs.json
│   │   └── ...
│   └── metrics/
│       └── training_history.json
└── ...
```

## Hardware Considerations
- Mac Studio M2 Ultra: MLX for training acceleration
- Training time: 2-4 hours per adapter with 10K-50K pairs
- Inference: <10ms per embedding (all adapters)

## Acceptance Criteria
- [ ] Training pair generators produce valid pairs from project data
- [ ] Adapters train without OOM on Mac Studio
- [ ] `patina adapters train` completes in <4 hours
- [ ] Multidimensional embeddings improve retrieval quality vs base E5
- [ ] Dimension-weighted search returns contextually appropriate results
- [ ] Patina thickness correctly reflects project maturity
