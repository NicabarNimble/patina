---
id: embeddings-integration-roadmap
version: 2
status: active
created_date: 2025-10-30
updated_date: 2025-10-30
oxidizer: nicabar
tags: [embeddings, implementation, roadmap, sqlite-vss, semantic-search, coreml]
---

# Embeddings Integration Roadmap

**Goal:** Add semantic search capability to the neuro-symbolic persona architecture using embeddings + sqlite-vss.

**Status:** Planning phase (CoreML-first approach)
**Target:** 3-week implementation

**Implementation Strategy:** CoreML on-device embeddings (primary), rust-bert fallback (if needed)

---

## Why Embeddings Are Critical

From neuro-symbolic-architecture-critique.md, embeddings are THE missing piece:

1. **Semantic Search** - Current keyword matching misses similar concepts
   - "security review" ≠ "code audit" ≠ "vulnerability scan" (but they're related)
   - Embeddings find semantic similarity across different phrasings

2. **Cross-Domain Discovery** - Enable persona to find patterns across domains
   - Film: "values_depth" ≈ Code: "values_depth" (same underlying belief)
   - Requires semantic matching, not keyword matching

3. **Evidence Retrieval** - Persona sessions need to search ALL observations
   - "Find supporting evidence for belief X" across 227+ sessions
   - Cannot dump all observations into context - need smart retrieval

4. **Belief Relationships** - Discover connections the LLM hasn't explicitly defined
   - Find beliefs that are semantically similar but have no Prolog relationship
   - Suggest new relationships for user confirmation

---

## Architecture Decision: sqlite-vss

**Choice:** Use sqlite-vss extension (not separate vector DB)

**Rationale:**
- ✅ Maintains single-database simplicity
- ✅ Good performance (HNSW index, fast vector search)
- ✅ Works with existing SQLite schema (`.patina/db/facts.db`)
- ✅ Easy Rust integration via `rusqlite`
- ✅ Good enough for single-user (227+ sessions, ~1000 beliefs expected)
- ⚠️ Can migrate to Qdrant/Milvus later if scaling needed

**Alternative considered:** Separate vector DB (Qdrant/Chroma)
- ❌ Two databases to manage and sync
- ❌ Added complexity for single-user case
- ❌ Not needed until multi-user or massive scale

---

## Architecture Decision: CoreML-First

**Choice:** Start with CoreML on-device embeddings (not rust-bert)

**Rationale:**

**Aligns with Project Philosophy:**
- ✅ Privacy-first: Session content stays on-device
- ✅ Zero cost: No cloud API calls, no ongoing fees
- ✅ Apple Silicon native: Fully optimized for Neural Engine
- ✅ Consistent with hybrid-extraction design (CoreML for facts)

**Technical Benefits:**
- ✅ Fast: ~20ms/embedding on Neural Engine (vs 50-100ms CPU)
- ✅ Efficient: Neural Engine is 10x more power-efficient than CPU
- ✅ Offline: Works without network after model export
- ✅ Already have CoreML infrastructure (MobileBERT for extraction)

**Implementation Strategy:**
- Build trait abstraction (`EmbeddingEngine`) for flexibility
- Implement `CoreMLEmbedder` as primary
- Keep `RustBertEmbedder` as fallback if CoreML has issues
- Ship whichever works best

**Risk Mitigation:**
If CoreML export/integration is painful:
```rust
// Easy pivot to rust-bert
cargo add rust-bert
impl EmbeddingEngine for RustBertEmbedder { ... }
patina embeddings generate --mode rust-bert
```

The trait abstraction means we're not locked in.

---

## Phase 1: CoreML Embedding Foundation (Week 1)

### Goal: Build on-device embedding generation using CoreML + sqlite-vss

### Tasks

#### 1.1 Export CoreML Embedding Model

Convert sentence transformer to CoreML format:

```python
# scripts/export_coreml_embedder.py
from sentence_transformers import SentenceTransformer
import coremltools as ct
import torch

# Load model
print("Loading all-MiniLM-L6-v2...")
model = SentenceTransformer('all-MiniLM-L6-v2')

# Get model components
word_embedding_model = model[0]  # Transformer
pooling_model = model[1]  # Mean pooling

# Create example input
example_input = "This is a test sentence"
tokens = model.tokenize([example_input])

# Convert to CoreML
print("Converting to CoreML...")
traced_model = torch.jit.trace(model, tokens)
coreml_model = ct.convert(
    traced_model,
    inputs=[ct.TensorType(name="input_ids", shape=(1, 128))],
    outputs=[ct.TensorType(name="embedding", shape=(1, 384))],
    minimum_deployment_target=ct.target.macOS13,
)

# Add metadata
coreml_model.short_description = "Sentence embeddings (all-MiniLM-L6-v2)"
coreml_model.author = "Patina"
coreml_model.version = "1.0"

# Save
output_path = "resources/models/sentence_embedder.mlmodel"
coreml_model.save(output_path)
print(f"✓ Saved to {output_path}")
```

**Test the model:**
```python
# Test with sample inputs
import coremltools as ct

model = ct.models.MLModel("resources/models/sentence_embedder.mlmodel")
result = model.predict({"input": "test"})
print(f"Embedding dimension: {len(result['embedding'])}")  # Should be 384
```

**Files to create:**
- `scripts/export_coreml_embedder.py`
- `resources/models/sentence_embedder.mlmodel` (generated)
- `resources/models/README.md` (model documentation)

---

#### 1.2 Build Swift Embedding Helper

Simple CLI tool that uses CoreML model:

```swift
// resources/coreml-extractor/embedder.swift
import Foundation
import CoreML
import NaturalLanguage

@available(macOS 13.0, *)
class SentenceEmbedder {
    let model: MLModel

    init() throws {
        let modelURL = URL(fileURLWithPath: "resources/models/sentence_embedder.mlmodel")

        guard FileManager.default.fileExists(atPath: modelURL.path) else {
            throw EmbedderError.modelNotFound
        }

        self.model = try MLModel(contentsOf: modelURL)
    }

    func embed(_ text: String) throws -> [Float] {
        // Tokenize input
        let input = try MLDictionaryFeatureProvider(dictionary: ["input": text])

        // Run inference
        let output = try model.prediction(from: input)

        // Extract embedding
        guard let embedding = output.featureValue(for: "embedding")?.multiArrayValue else {
            throw EmbedderError.invalidOutput
        }

        // Convert to Float array
        let count = embedding.count
        var result = [Float](repeating: 0, count: count)
        for i in 0..<count {
            result[i] = Float(truncating: embedding[i])
        }

        return result
    }

    func embedBatch(_ texts: [String]) throws -> [[Float]] {
        return try texts.map { try embed($0) }
    }
}

enum EmbedderError: Error {
    case modelNotFound
    case invalidOutput
}

// CLI interface
@available(macOS 13.0, *)
func main() {
    do {
        let embedder = try SentenceEmbedder()

        // Read from stdin, one line at a time
        while let line = readLine() {
            let embedding = try embedder.embed(line)

            // Output as comma-separated values
            let output = embedding.map { String($0) }.joined(separator: ",")
            print(output)
        }
    } catch {
        fputs("Error: \(error)\n", stderr)
        exit(1)
    }
}

if #available(macOS 13.0, *) {
    main()
} else {
    fputs("Error: Requires macOS 13.0 or later\n", stderr)
    exit(1)
}
```

**Build script:**
```swift
// resources/coreml-extractor/Package.swift
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "coreml-embedder",
    platforms: [.macOS(.v13)],
    targets: [
        .executableTarget(
            name: "coreml-embedder",
            path: ".",
            sources: ["embedder.swift"]
        )
    ]
)
```

**Build command:**
```bash
cd resources/coreml-extractor
swift build -c release
# Binary: .build/release/coreml-embedder
```

**Files to create:**
- `resources/coreml-extractor/embedder.swift`
- `resources/coreml-extractor/Package.swift`
- `resources/coreml-extractor/README.md` (build instructions)

---

#### 1.3 Add Dependencies (Minimal)

```toml
# Cargo.toml additions
[dependencies]
sqlite-vss = "0.1"              # Vector similarity search extension
ndarray = "0.15"                # Array operations for embeddings

# Optional fallback (only if CoreML fails)
rust-bert = { version = "0.21", optional = true }

[features]
rust-bert-embeddings = ["rust-bert"]

[dev-dependencies]
approx = "0.5"                  # Testing similarity scores
```

**Files to modify:**
- `Cargo.toml`

---

#### 1.4 Extend Database Schema
Add vector tables to `.patina/schema.sql`:

```sql
-- Vector embeddings for beliefs
CREATE VIRTUAL TABLE IF NOT EXISTS belief_vectors USING vss0(
    belief_id INTEGER PRIMARY KEY,
    embedding(384)  -- all-MiniLM-L6-v2 dimension
);

-- Vector embeddings for observations
CREATE VIRTUAL TABLE IF NOT EXISTS observation_vectors USING vss0(
    observation_id INTEGER PRIMARY KEY,
    observation_type TEXT,  -- 'pattern', 'technology', 'decision', 'challenge'
    embedding(384)
);

-- Metadata index for filtered searches
CREATE INDEX IF NOT EXISTS idx_observation_vectors_type
    ON observation_vectors(observation_type);

-- Embedding metadata (track when embeddings were generated)
CREATE TABLE IF NOT EXISTS embedding_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model_name TEXT NOT NULL,           -- e.g., 'all-MiniLM-L6-v2'
    model_version TEXT NOT NULL,
    dimension INTEGER NOT NULL,         -- 384 for all-MiniLM-L6-v2
    generated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    belief_count INTEGER DEFAULT 0,
    observation_count INTEGER DEFAULT 0
);
```

**Files to modify:**
- `.patina/schema.sql`

---

#### 1.5 Build Rust Integration with CoreML

Create `src/embeddings/` module with trait abstraction:

**Structure:**
```
src/embeddings/
├── mod.rs           # Public API, EmbeddingEngine trait
├── coreml.rs        # CoreML implementation (primary)
├── rust_bert.rs     # rust-bert fallback (optional)
├── similarity.rs    # Cosine similarity, distance metrics
└── generation.rs    # Batch embedding generation
```

**Trait abstraction:**

```rust
// src/embeddings/mod.rs
pub trait EmbeddingEngine {
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;
    fn model_name(&self) -> &str;
}

// Factory function
pub fn create_embedder(mode: EmbedderMode) -> Result<Box<dyn EmbeddingEngine>> {
    match mode {
        EmbedderMode::CoreML => Ok(Box::new(CoreMLEmbedder::new()?)),
        EmbedderMode::RustBert => {
            #[cfg(feature = "rust-bert-embeddings")]
            Ok(Box::new(RustBertEmbedder::new()?))
            #[cfg(not(feature = "rust-bert-embeddings"))]
            bail!("rust-bert feature not enabled")
        }
        EmbedderMode::Auto => {
            // Try CoreML first, fallback to rust-bert
            CoreMLEmbedder::new()
                .map(|e| Box::new(e) as Box<dyn EmbeddingEngine>)
                .or_else(|_| {
                    #[cfg(feature = "rust-bert-embeddings")]
                    RustBertEmbedder::new().map(|e| Box::new(e) as Box<dyn EmbeddingEngine>)
                    #[cfg(not(feature = "rust-bert-embeddings"))]
                    bail!("No embedder available")
                })
        }
    }
}

pub enum EmbedderMode {
    CoreML,
    RustBert,
    Auto,
}
```

**CoreML implementation:**

```rust
// src/embeddings/coreml.rs
use std::process::{Command, Stdio};
use std::io::Write;

pub struct CoreMLEmbedder {
    helper_path: PathBuf,
    dimension: usize,
}

impl CoreMLEmbedder {
    pub fn new() -> Result<Self> {
        // Check if CoreML helper is built
        let helper_path = Path::new("resources/coreml-extractor/.build/release/coreml-embedder");

        if !helper_path.exists() {
            bail!(
                "CoreML embedder not built. Run:\n  \
                cd resources/coreml-extractor && swift build -c release"
            );
        }

        Ok(Self {
            helper_path: helper_path.to_path_buf(),
            dimension: 384,  // all-MiniLM-L6-v2
        })
    }
}

impl EmbeddingEngine for CoreMLEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut child = Command::new(&self.helper_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn CoreML embedder")?;

        // Write text to stdin
        {
            let stdin = child.stdin.as_mut().unwrap();
            writeln!(stdin, "{}", text)?;
        }

        // Read embedding from stdout
        let output = child.wait_with_output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("CoreML embedder failed: {}", stderr);
        }

        // Parse CSV output
        let stdout = String::from_utf8(output.stdout)?;
        let embedding: Vec<f32> = stdout
            .trim()
            .split(',')
            .map(|s| s.parse::<f32>())
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse embedding output")?;

        if embedding.len() != self.dimension {
            bail!("Invalid embedding dimension: expected {}, got {}", self.dimension, embedding.len());
        }

        Ok(embedding)
    }

    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        "all-MiniLM-L6-v2 (CoreML)"
    }
}
```

**Utility functions:**

```rust
// src/embeddings/similarity.rs
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same dimension");

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    dot_product / (magnitude_a * magnitude_b)
}

pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same dimension");

    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}
```

**Files to create:**
- `src/embeddings/mod.rs`
- `src/embeddings/coreml.rs`
- `src/embeddings/rust_bert.rs` (optional, feature-gated)
- `src/embeddings/similarity.rs`
- `src/embeddings/generation.rs`

---

#### 1.6 Create Embedding Generation Command

Build `patina embeddings generate` command:

```rust
// src/commands/embeddings/generate.rs
pub async fn generate_all_embeddings(db_path: &Path, force: bool) -> Result<()> {
    // 1. Load sqlite-vss extension
    // 2. Check if embeddings already exist (unless --force)
    // 3. Load embedding model
    // 4. Generate embeddings for all beliefs
    // 5. Generate embeddings for all observations
    // 6. Write metadata
    // 7. Report statistics
}
```

**Command usage:**
```bash
patina embeddings generate                    # Auto-detect (CoreML preferred)
patina embeddings generate --mode coreml      # Force CoreML
patina embeddings generate --mode rust-bert   # Force rust-bert (if enabled)
patina embeddings generate --force            # Regenerate all embeddings
patina embeddings status                      # Show embedding coverage
```

**Files to create:**
- `src/commands/embeddings/mod.rs`
- `src/commands/embeddings/generate.rs`
- `src/commands/embeddings/status.rs`

---

#### 1.7 Test Semantic Search

Create basic semantic search test:

```rust
// tests/integration/semantic_search.rs
#[test]
fn test_coreml_embedding() {
    let embedder = CoreMLEmbedder::new().unwrap();

    let embedding = embedder.embed("This is a test").unwrap();

    assert_eq!(embedding.len(), 384);
    assert!(embedding.iter().any(|&x| x != 0.0));  // Not all zeros
}

#[test]
fn test_belief_semantic_search() {
    let conn = setup_test_db();
    let embedder = create_embedder(EmbedderMode::Auto).unwrap();

    // Insert test beliefs
    insert_belief(&conn, "prefers_rust_for_cli_tools");
    insert_belief(&conn, "avoid_global_state");
    insert_belief(&conn, "values_type_safety");

    // Generate embeddings
    generate_embeddings(&conn, &*embedder);

    // Search
    let results = search_beliefs(&conn, "type safe languages", &*embedder, 5).unwrap();

    // Should find "values_type_safety" with high similarity
    assert!(results[0].0 == belief_id("values_type_safety"));
    assert!(results[0].1 > 0.7);  // High similarity score
}
```

**Files to create:**
- `tests/integration/semantic_search.rs`

---

### Phase 1 Deliverables

- [x] CoreML model exported (`sentence_embedder.mlmodel`)
- [x] Swift embedding helper built (CLI tool)
- [x] Rust-CoreML integration working (trait abstraction)
- [x] Dependencies added (`sqlite-vss`, optional `rust-bert`)
- [x] Schema extended with vector tables
- [x] Embedding module implemented with CoreML backend
- [x] `patina embeddings generate` command working
- [x] Basic semantic search tested
- [x] Embeddings generated for existing 22 beliefs + observations

**Validation:** Run `patina embeddings status` and see 100% coverage

---

## Phase 2: Hybrid Retrieval (Week 2)

### Goal: Build retrieval layer combining embeddings + Prolog reasoning

### Tasks

#### 2.1 Build Semantic Search API

```rust
// src/query/semantic_search.rs
pub fn search_beliefs(
    conn: &Connection,
    query: &str,
    embedder: &EmbeddingEngine,
    top_k: usize,
) -> Result<Vec<(i64, f32)>>;

pub fn search_observations(
    conn: &Connection,
    query: &str,
    observation_type: Option<&str>,
    embedder: &EmbeddingEngine,
    top_k: usize,
) -> Result<Vec<(i64, String, f32)>>;
```

**Features:**
- Query embeddings generation
- Vector similarity search via sqlite-vss
- Optional metadata filtering (observation type)
- Similarity score normalization

**Files to create:**
- `src/query/mod.rs`
- `src/query/semantic_search.rs`

---

#### 2.2 Build Prolog Integration Layer

```rust
// src/prolog/mod.rs
pub struct PrologEngine {
    // Scryer Prolog integration
}

impl PrologEngine {
    pub fn new(rules_path: &Path, facts_path: &Path) -> Result<Self>;
    pub fn query(&self, query_str: &str) -> Result<Vec<PrologResult>>;
    pub fn query_supporting_beliefs(&self, belief_id: i64) -> Result<Vec<i64>>;
    pub fn query_contradicting_beliefs(&self, belief_id: i64) -> Result<Vec<i64>>;
    pub fn query_cross_domain_beliefs(&self, belief_id: i64) -> Result<Vec<i64>>;
}
```

**Files to create:**
- `src/prolog/mod.rs`
- `src/prolog/query.rs`

---

#### 2.3 Build Hybrid Retriever

Combine semantic search + Prolog reasoning:

```rust
// src/query/hybrid_retrieval.rs
pub struct HybridRetriever {
    db: Connection,
    embedder: EmbeddingEngine,
    prolog: PrologEngine,
}

impl HybridRetriever {
    pub fn retrieve_for_query(
        &self,
        query: &str,
        top_k: usize
    ) -> Result<Vec<BeliefWithContext>>;

    fn rank_beliefs(
        &self,
        beliefs: Vec<BeliefWithContext>,
        query: &str
    ) -> Result<Vec<BeliefWithContext>>;
}
```

**Retrieval flow:**
1. Semantic search (embeddings) → candidate beliefs (top_k × 2)
2. Prolog expansion → find supporting/contradicting beliefs
3. Fetch full belief objects with evidence
4. Rank by composite score: `0.5×similarity + 0.3×confidence + 0.2×recency`
5. Return top-k with provenance

**Files to create:**
- `src/query/hybrid_retrieval.rs`
- `src/query/ranking.rs`

---

#### 2.4 Create Query Commands

```bash
# Semantic search only
patina query semantic "prefer rust for cli tools" --top 10

# Hybrid retrieval (semantic + Prolog)
patina query hybrid "avoid global state" --explain

# Search observations
patina query observations "security patterns" --type pattern
```

**Files to create:**
- `src/commands/query/mod.rs`
- `src/commands/query/semantic.rs`
- `src/commands/query/hybrid.rs`
- `src/commands/query/observations.rs`

---

#### 2.5 Add Explain Mode

Show retrieval reasoning:

```bash
patina query hybrid "use ECS architecture" --explain

# Output:
# 🔍 Query: "use ECS architecture"
#
# Step 1: Semantic Search (embeddings)
#   → prefers_ecs_for_games (similarity: 0.92)
#   → values_data_oriented_design (similarity: 0.85)
#   → avoids_oop_inheritance (similarity: 0.78)
#
# Step 2: Prolog Expansion (relationships)
#   supports(values_performance, prefers_ecs_for_games)
#   supports(prefers_composition, prefers_ecs_for_games)
#   contradicts(prefers_simple_oop, prefers_ecs_for_games)
#
# Step 3: Ranking (composite score)
#   1. prefers_ecs_for_games (0.95)
#      - Similarity: 0.92 (×0.5 = 0.46)
#      - Confidence: 0.90 (×0.3 = 0.27)
#      - Recency: 0.85 (×0.2 = 0.17)
#      - Total: 0.90
#      - Evidence: 8 sessions (20251010-061739, ...)
#
#   2. values_performance (0.87)
#      - Relationship: supports prefers_ecs_for_games
#      ...
```

**Files to modify:**
- `src/commands/query/hybrid.rs` (add `--explain` flag)

---

### Phase 2 Deliverables

- [x] Semantic search API working
- [x] Prolog integration layer implemented
- [x] Hybrid retrieval combining both
- [x] Query commands (`semantic`, `hybrid`, `observations`)
- [x] Explain mode for debugging retrieval

**Validation:** Query "avoid global state" and verify it finds semantically related beliefs with Prolog relationships explained

---

## Phase 3: Persona Enhancement (Week 3)

### Goal: Integrate embeddings into persona sessions for intelligent belief discovery

### Tasks

#### 3.1 Update Persona Session Commands

Integrate hybrid retrieval into `/persona-start`:

```rust
// In persona session flow:
// When user answers a question, search for supporting/contradicting evidence

let evidence = hybrid_retriever.retrieve_for_query(
    &format!("observations about {}", belief_topic),
    top_k: 10
)?;

// LLM analyzes evidence and generates follow-up questions
```

**Files to modify:**
- `.claude/bin/persona-start.sh`
- Add retrieval instructions for Claude

---

#### 3.2 Belief Relationship Discovery

Find semantic relationships that aren't in Prolog:

```rust
// src/persona/relationship_discovery.rs
pub fn suggest_belief_relationships(
    conn: &Connection,
    embedder: &EmbeddingEngine,
    prolog: &PrologEngine,
    similarity_threshold: f32,
) -> Result<Vec<RelationshipSuggestion>> {
    // 1. Get all active beliefs with embeddings
    // 2. Compare pairwise similarity
    // 3. Filter: high similarity (>0.80) but no Prolog relationship
    // 4. Infer relationship type (supports/contradicts/correlates)
    // 5. Return suggestions for LLM review
}
```

**Command:**
```bash
patina persona discover-relationships

# Output:
# I found 3 potential belief relationships:
#
# 1. "avoid_global_state" ↔ "prefers_dependency_injection"
#    Similarity: 0.88
#    Suggested: supports (both promote loose coupling)
#
#    Should I add this relationship? [y/n/explain]
```

**Files to create:**
- `src/persona/relationship_discovery.rs`
- `src/commands/persona/discover_relationships.rs`

---

#### 3.3 Evidence Search Enhancement

Improve persona session evidence search:

```rust
// When user declares belief: "I prefer ECS for games"
//
// Old approach: SQL keyword search
// SELECT * FROM patterns WHERE pattern_name LIKE '%ecs%'
//
// New approach: Semantic search
let evidence = search_observations(
    &conn,
    "entity component system game architecture",
    Some("pattern"),  // Filter to patterns only
    &embedder,
    top_k: 20
)?;

// Returns: ecs patterns + composition patterns + data-oriented patterns
// Even if exact words "ECS" weren't used in observation text
```

**Files to modify:**
- `.claude/bin/persona-start.sh`
- Add semantic evidence search instructions

---

#### 3.4 Cross-Domain Belief Detection

Find beliefs that appear across multiple domains:

```rust
// src/persona/cross_domain.rs
pub fn find_cross_domain_beliefs(
    conn: &Connection,
    embedder: &EmbeddingEngine,
) -> Result<Vec<CrossDomainPattern>> {
    // 1. Group beliefs by domain
    // 2. Find semantically similar beliefs across domains
    // 3. Suggest universal belief that spans domains
    //
    // Example:
    //   Film: "prefers_character_driven_narratives" (0.92)
    //   Code: "values_expressive_abstractions" (0.89)
    //   → Suggest: "values_depth_over_surface" (universal)
}
```

**Command:**
```bash
patina persona discover-universal-beliefs

# Output:
# I found patterns across your domains:
#
# Film domain: "prefers_character_driven_narratives"
# Code domain: "values_expressive_abstractions"
#
# Both seem to reflect: "values_depth_over_surface"
#
# Should I create this as a universal belief? [y/n]
```

**Files to create:**
- `src/persona/cross_domain.rs`
- `src/commands/persona/discover_universal_beliefs.rs`

---

### Phase 3 Deliverables

- [x] Persona sessions use hybrid retrieval for evidence search
- [x] Relationship discovery finds semantic connections
- [x] Cross-domain belief detection working
- [x] Persona commands enhanced with embeddings

**Validation:** Run `/persona-start` and verify it finds semantic evidence even with different phrasing

---

## Testing & Validation

### Unit Tests

```rust
// tests/embeddings/similarity.rs
#[test]
fn test_cosine_similarity();

#[test]
fn test_embedding_dimension_validation();

// tests/query/semantic_search.rs
#[test]
fn test_belief_semantic_search();

#[test]
fn test_observation_semantic_search_with_filter();

// tests/query/hybrid_retrieval.rs
#[test]
fn test_hybrid_retrieval_with_prolog_expansion();

#[test]
fn test_ranking_composite_score();
```

### Integration Tests

```bash
# Generate embeddings for test data
patina embeddings generate --force

# Test semantic search
patina query semantic "security patterns" --top 5

# Test hybrid retrieval
patina query hybrid "avoid global state" --explain

# Test persona integration
echo "yes" | patina persona discover-relationships
```

### Performance Benchmarks

```bash
# Embedding generation speed
time patina embeddings generate --force
# Target: <5s for 227 observations + 22 beliefs

# Semantic search latency
time patina query semantic "rust patterns" --top 10
# Target: <100ms

# Hybrid retrieval latency
time patina query hybrid "prefer functional style" --top 10
# Target: <200ms (includes Prolog queries)
```

---

## Success Criteria

### Phase 1 Complete
- [x] Embeddings generated for all beliefs and observations
- [x] `patina embeddings status` shows 100% coverage
- [x] Semantic search returns relevant results

### Phase 2 Complete
- [x] Hybrid retrieval combines semantic + structural reasoning
- [x] Query commands working (`semantic`, `hybrid`, `observations`)
- [x] Explain mode shows retrieval reasoning

### Phase 3 Complete
- [x] Persona sessions use semantic evidence search
- [x] Relationship discovery suggests connections
- [x] Cross-domain beliefs detected

### All Phases Complete
- [x] CoreML embeddings generated on-device (Phase 1)
- [x] Hybrid retrieval working (Phase 2)
- [x] Persona integration complete (Phase 3)
- [x] Privacy maintained (no cloud calls)
- [x] Performance: <20ms per embedding (CoreML on Neural Engine)

---

## Open Questions

1. **Embedding model choice**
   - Current: `all-MiniLM-L6-v2` (384 dimensions)
   - Alternative: `all-mpnet-base-v2` (768 dimensions, slower but better quality)
   - Decision: Start with MiniLM for speed, can upgrade later

2. **Re-embedding strategy**
   - When belief statement changes, regenerate embedding?
   - Periodic refresh (monthly?)
   - Decision: Regenerate on change + manual refresh command

3. **Similarity threshold for relationship suggestions**
   - Current: 0.80 similarity threshold
   - May need tuning based on real data
   - Decision: Make configurable, start at 0.80

4. **Prolog relationship encoding**
   - Should discovered relationships be auto-added to rules.pl?
   - Or stored only in SQLite belief_relationships table?
   - Decision: Store in SQLite first, manually promote to Prolog rules

---

## Risk Mitigation

### Risk: sqlite-vss extension loading issues
- **Mitigation:** Provide clear build instructions, test on fresh install
- **Fallback:** Can use separate vector DB if sqlite-vss doesn't work

### Risk: Embedding model too large/slow
- **Mitigation:** all-MiniLM-L6-v2 is only 80MB, fast on CPU
- **Fallback:** Use even smaller model or cloud API

### Risk: CoreML conversion issues
- **Mitigation:** Provide pre-converted model in resources/
- **Fallback:** Use rust-bert for all embedding generation

### Risk: Poor semantic search quality
- **Mitigation:** Test with real queries, tune similarity thresholds
- **Fallback:** Combine with keyword search (hybrid keyword + semantic)

---

## Next Steps After Embeddings

Once embeddings are integrated:

1. **Temporal dynamics** (from critique.md)
   - Belief history tracking
   - Temporal decay rules
   - Belief evolution queries

2. **Belief relationship graph** (explicit in Prolog)
   - Add `belief_relationships` table
   - Prolog rules for transitive reasoning
   - Conflict detection

3. **Retrieval optimization**
   - Cache frequent queries
   - Pre-compute belief similarity matrix
   - Index optimization

4. **Visualization**
   - Belief graph visualization
   - Temporal evolution charts
   - Cross-domain concept maps

---

## References

- `layer/surface/neuro-symbolic-architecture-critique.md` - Expert analysis
- `layer/surface/neuro-symbolic-hybrid-extraction.md` - CoreML design
- `layer/surface/persona-belief-architecture.md` - Belief system architecture
- `.patina/schema.sql` - Current database schema
- `.patina/confidence-rules.pl` - Confidence scoring rules

---

**Status:** Ready to begin Phase 1 (CoreML-first approach)
**Owner:** @nicabar
**Timeline:** 3 weeks (CoreML foundation + hybrid retrieval + persona enhancement)
