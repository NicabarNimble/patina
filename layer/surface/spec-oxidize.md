# Spec: Oxidize

**Status:** Phase 2 MVP Complete (2025-11-23) ✅

## Overview

Oxidize transforms materialized SQLite data into searchable vectors using a recipe-driven approach. Recipes are git-tracked; artifacts are built locally.

**Pipeline position:**
```
Events → materialize → SQLite → oxidize → Vectors
                       (input)   (this)   (output)
```

**MVP Implementation:**
- ✅ Recipe parser (`.patina/oxidize.yaml`)
- ✅ Training pair generator (SameSessionPairs from eventlog)
- ✅ 2-layer MLP trainer (triplet loss, gradient descent)
- ✅ E5-base-v2 embedding integration
- ✅ End-to-end pipeline working
- ⏳ ONNX export (pending)
- ⏳ USearch index builder (pending)

## Recipe Format

**Location:** `.patina/oxidize.yaml`

```yaml
# .patina/oxidize.yaml
version: 1
embedding_model: e5-base-v2

projections:
  # Dimension projections (simple learned layers)
  semantic:
    type: dimension
    layers: [768, 1024, 256]
    training:
      source: sessions.observations
      pair_type: same_session  # observations from same session are similar
      epochs: 10
      batch_size: 32

  temporal:
    type: dimension
    layers: [768, 1024, 256]
    training:
      source: git.co_changes
      pair_type: co_changed  # files changed together are similar
      epochs: 10

  dependency:
    type: dimension
    layers: [768, 1024, 256]
    training:
      source: code.call_graph
      pair_type: caller_callee  # functions that call each other are similar
      epochs: 10

  syntactic:
    type: dimension
    layers: [768, 1024, 256]
    training:
      source: code.functions
      pair_type: ast_similar  # similar AST structure
      epochs: 10

  architectural:
    type: dimension
    layers: [768, 1024, 256]
    training:
      source: code.functions
      pair_type: same_module  # same directory/module
      epochs: 10

  # World-model projections (dynamics-aware)
  state-encoder:
    type: world-model
    layers: [768, 1024, 512]
    training:
      source: git.transitions
      pair_type: state_similarity
      epochs: 20

  action-encoder:
    type: world-model
    layers: [768, 1024, 256]
    training:
      source: git.commits
      pair_type: effect_similarity
      epochs: 20
```

## Components

### 1. Recipe Parser
**Location:** `src/commands/oxidize/recipe.rs`

```rust
#[derive(Deserialize)]
pub struct OxidizeRecipe {
    pub version: u32,
    pub embedding_model: String,
    pub projections: HashMap<String, ProjectionConfig>,
}

#[derive(Deserialize)]
pub struct ProjectionConfig {
    pub projection_type: ProjectionType,
    pub layers: Vec<usize>,
    pub training: TrainingConfig,
}

#[derive(Deserialize)]
pub enum ProjectionType {
    Dimension,
    WorldModel,
}

#[derive(Deserialize)]
pub struct TrainingConfig {
    pub source: String,      // e.g., "sessions.observations"
    pub pair_type: String,   // e.g., "same_session"
    pub epochs: usize,
    pub batch_size: Option<usize>,
}
```

### 2. Embedding Model Registry
**Location:** `src/adapters/embeddings/`

```rust
pub trait EmbeddingModel: Send + Sync {
    fn name(&self) -> &str;
    fn dimensions(&self) -> usize;
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
}

// Implementations
pub struct E5BaseV2 { /* ONNX runtime */ }
pub struct BgeSmall { /* ONNX runtime */ }
pub struct NomicEmbed { /* ONNX runtime */ }

pub fn get_embedding_model(name: &str) -> Result<Box<dyn EmbeddingModel>> {
    match name {
        "e5-base-v2" => Ok(Box::new(E5BaseV2::load()?)),
        "bge-small" => Ok(Box::new(BgeSmall::load()?)),
        "nomic-embed" => Ok(Box::new(NomicEmbed::load()?)),
        _ => Err(anyhow!("Unknown embedding model: {}", name)),
    }
}
```

### 3. Training Pair Generators
**Location:** `src/adapters/projections/training/`

```rust
pub trait PairGenerator {
    fn generate_pairs(&self, db: &Connection) -> Result<Vec<TrainingPair>>;
}

pub struct TrainingPair {
    pub anchor: String,
    pub positive: String,
    pub negative: String,
}

// Same session observations are similar
pub struct SameSessionPairs;
impl PairGenerator for SameSessionPairs {
    fn generate_pairs(&self, db: &Connection) -> Result<Vec<TrainingPair>> {
        // SELECT observations grouped by session_id
        // Positive: another observation from same session
        // Negative: observation from different session
    }
}

// Co-changed files are similar
pub struct CoChangedPairs;
impl PairGenerator for CoChangedPairs {
    fn generate_pairs(&self, db: &Connection) -> Result<Vec<TrainingPair>> {
        // SELECT from co_changes table
        // Positive: file that changed together
        // Negative: file that never changed together
    }
}

// Caller/callee are similar
pub struct CallGraphPairs;
impl PairGenerator for CallGraphPairs {
    fn generate_pairs(&self, db: &Connection) -> Result<Vec<TrainingPair>> {
        // SELECT from call_graph table
        // Positive: function that calls/is called by anchor
        // Negative: unrelated function
    }
}
```

### 4. Projection Trainer
**Location:** `src/adapters/projections/trainer.rs`

```rust
pub struct ProjectionTrainer {
    embedding_model: Box<dyn EmbeddingModel>,
}

impl ProjectionTrainer {
    pub fn train(
        &self,
        config: &ProjectionConfig,
        pairs: &[TrainingPair],
    ) -> Result<TrainedProjection> {
        // 2-layer MLP: input_dim → hidden → output_dim
        let input_dim = self.embedding_model.dimensions();  // 768
        let hidden_dim = config.layers[1];  // 1024
        let output_dim = config.layers[2];  // 256

        let mut weights1 = random_matrix(hidden_dim, input_dim);
        let mut weights2 = random_matrix(output_dim, hidden_dim);

        for epoch in 0..config.training.epochs {
            for pair in pairs {
                let anchor_base = self.embedding_model.embed(&pair.anchor)?;
                let pos_base = self.embedding_model.embed(&pair.positive)?;
                let neg_base = self.embedding_model.embed(&pair.negative)?;

                let anchor_proj = forward(&weights1, &weights2, &anchor_base);
                let pos_proj = forward(&weights1, &weights2, &pos_base);
                let neg_proj = forward(&weights1, &weights2, &neg_base);

                let loss = triplet_loss(&anchor_proj, &pos_proj, &neg_proj);
                backward(&mut weights1, &mut weights2, loss);
            }
        }

        Ok(TrainedProjection { weights1, weights2 })
    }
}
```

### 5. Oxidize Command
**Location:** `src/commands/oxidize/mod.rs`

```rust
pub fn oxidize(full: bool, only: Option<Vec<String>>) -> Result<()> {
    let recipe = load_recipe(".patina/oxidize.yaml")?;
    let db = open_db(".patina/data/patina.db")?;
    let embedding_model = get_embedding_model(&recipe.embedding_model)?;

    let output_dir = format!(".patina/data/embeddings/{}/projections", recipe.embedding_model);
    fs::create_dir_all(&output_dir)?;

    for (name, config) in &recipe.projections {
        if let Some(ref only) = only {
            if !only.contains(name) { continue; }
        }

        println!("Training projection: {}", name);

        // Generate training pairs
        let pair_generator = get_pair_generator(&config.training)?;
        let pairs = pair_generator.generate_pairs(&db)?;
        println!("  Generated {} training pairs", pairs.len());

        // Train projection
        let trainer = ProjectionTrainer::new(embedding_model.clone());
        let projection = trainer.train(config, &pairs)?;

        // Save weights
        projection.save_onnx(&format!("{}/{}.onnx", output_dir, name))?;

        // Build vector index
        build_index(&db, &embedding_model, &projection, &format!("{}/{}.usearch", output_dir, name))?;

        println!("  Saved {}.onnx and {}.usearch", name, name);
    }

    Ok(())
}
```

**CLI:**
```bash
patina oxidize                    # Train all projections from recipe
patina oxidize --only semantic    # Train specific projection
patina oxidize --full             # Rebuild all from scratch
patina oxidize --dry-run          # Show what would be trained
```

## File Structure

```
.patina/
├── oxidize.yaml                  ← RECIPE (git-tracked)
└── data/                         ← ARTIFACTS (gitignored)
    ├── patina.db                 ← materialized from events
    └── embeddings/
        └── e5-base-v2/
            ├── base.usearch      ← raw embeddings index
            └── projections/
                ├── semantic.onnx
                ├── semantic.usearch
                ├── temporal.onnx
                ├── temporal.usearch
                └── ...
```

## Multi-User Workflow

**Recipe is shared:**
```bash
git pull                          # Get updated recipe
patina materialize                # Rebuild SQLite from events
patina oxidize                    # Build vectors from recipe
```

**Same recipe + same events → equivalent adapters**

Each user builds locally. No binary artifacts in git.

## Version Tracking

```yaml
# .patina/oxidize.yaml
version: 3                        # Bump when recipe changes

# After oxidize, writes:
# .patina/data/oxidize.lock
embedding_model: e5-base-v2
events_hash: sha256:abc123...     # Hash of events at build time
recipe_version: 3
built_at: 2025-11-21T10:00:00Z
projections:
  semantic:
    weights_hash: sha256:def456...
    training_pairs: 15234
    epochs_completed: 10
```

## Hardware Considerations

- **Mac Studio M2 Ultra:** MLX for training acceleration
- **Training time:** 1-4 hours per projection with 10K-50K pairs
- **Inference:** <10ms per embedding (all projections)
- **Storage:** ~4MB per projection (ONNX weights)

## Implementation Status

### Phase 2 MVP ✅ COMPLETE (2025-11-23)

**Implemented:**
- [x] `oxidize.yaml` parser with validation (version, embedding_model, projections)
- [x] `ProjectionConfig` parsing (layers, epochs, batch_size)
- [x] `patina oxidize` command with full pipeline
- [x] SameSessionPairs generator (queries eventlog for session observations)
- [x] 2-layer MLP trainer with triplet loss
- [x] E5-base-v2 embedding integration
- [x] End-to-end training tested (100 pairs from 108 sessions, 10 epochs)

**Tested Configuration:**
```yaml
version: 1
embedding_model: e5-base-v2

projections:
  semantic:
    layers: [768, 1024, 256]
    epochs: 10
    batch_size: 32
```

**Test Results:**
- 108 sessions with 1,015 observations scraped from eventlog
- 100 training triplets generated successfully
- E5 embeddings: 768 dimensions
- Projection trained: 768→1024→256
- Training time: ~10 seconds for 10 epochs

### Pending (Full Phase 2)

- [ ] ONNX export for trained projections
- [ ] USearch indices built for each projection
- [ ] Proper backpropagation (current: simplified gradient approximation)
- [ ] Lock file tracks build state (oxidize.lock)
- [ ] `--only` flag trains subset of projections
- [ ] `--dry-run` shows plan without training
- [ ] Additional pair generators:
  - [ ] CoChangedPairs (temporal projection)
  - [ ] CallGraphPairs (dependency projection)
  - [ ] ASTSimilarPairs (syntactic projection)
- [ ] Multiple embedding models (BGE, nomic)
- [ ] World-model projections (state-encoder, action-encoder)
