---
id: embeddings-integration-roadmap
version: 1
status: active
created_date: 2025-10-30
updated_date: 2025-10-30
oxidizer: nicabar
tags: [embeddings, implementation, roadmap, sqlite-vss, semantic-search]
---

# Embeddings Integration Roadmap

**Goal:** Add semantic search capability to the neuro-symbolic persona architecture using embeddings + sqlite-vss.

**Status:** Planning phase
**Target:** 4-week implementation

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

## Phase 1: Foundation (Week 1)

### Goal: Add embedding infrastructure and generate embeddings for existing data

### Tasks

#### 1.1 Add Dependencies
```toml
# Cargo.toml additions
[dependencies]
sqlite-vss = "0.1"              # Vector similarity search extension
rust-bert = "0.21"              # Sentence embeddings (all-MiniLM-L6-v2)
ndarray = "0.15"                # Array operations for embeddings

[dev-dependencies]
approx = "0.5"                  # Testing similarity scores
```

**Files to modify:**
- `Cargo.toml`

---

#### 1.2 Extend Database Schema
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

#### 1.3 Build Embedding Module

Create `src/embeddings/` module:

**Structure:**
```
src/embeddings/
├── mod.rs           # Public API, EmbeddingEngine struct
├── models.rs        # Model loading and management
├── similarity.rs    # Cosine similarity, distance metrics
└── generation.rs    # Batch embedding generation
```

**Key components:**

```rust
// src/embeddings/mod.rs
pub struct EmbeddingEngine {
    model: SentenceEmbeddingsModel,
    dimension: usize,
}

impl EmbeddingEngine {
    pub fn new() -> Result<Self>;
    pub fn embed_belief(&self, statement: &str, why: &str) -> Result<Vec<f32>>;
    pub fn embed_observation(&self, obs: &Observation) -> Result<Vec<f32>>;
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

// src/embeddings/similarity.rs
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32;
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32;
```

**Files to create:**
- `src/embeddings/mod.rs`
- `src/embeddings/models.rs`
- `src/embeddings/similarity.rs`
- `src/embeddings/generation.rs`

---

#### 1.4 Create Embedding Generation Command

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
patina embeddings generate           # Generate for new items only
patina embeddings generate --force   # Regenerate all embeddings
patina embeddings status             # Show embedding coverage
```

**Files to create:**
- `src/commands/embeddings/mod.rs`
- `src/commands/embeddings/generate.rs`
- `src/commands/embeddings/status.rs`

---

#### 1.5 Test Semantic Search

Create basic semantic search test:

```rust
// tests/integration/semantic_search.rs
#[test]
fn test_belief_semantic_search() {
    let conn = setup_test_db();
    let embedder = EmbeddingEngine::new().unwrap();

    // Insert test beliefs
    insert_belief(&conn, "prefers_rust_for_cli_tools");
    insert_belief(&conn, "avoid_global_state");
    insert_belief(&conn, "values_type_safety");

    // Generate embeddings
    generate_embeddings(&conn, &embedder);

    // Search
    let results = search_beliefs(&conn, "type safe languages", &embedder, 5).unwrap();

    // Should find "values_type_safety" with high similarity
    assert!(results[0].0 == belief_id("values_type_safety"));
    assert!(results[0].1 > 0.7);  // High similarity score
}
```

**Files to create:**
- `tests/integration/semantic_search.rs`

---

### Phase 1 Deliverables

- [x] Dependencies added (`sqlite-vss`, `rust-bert`)
- [x] Schema extended with vector tables
- [x] Embedding module implemented
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

## Phase 4: On-Device Embeddings (Week 4)

### Goal: Generate embeddings on-device via CoreML (maintain privacy, zero cost)

### Tasks

#### 4.1 Export CoreML Embedding Model

```python
# scripts/export_coreml_embedder.py
from sentence_transformers import SentenceTransformer
import coremltools as ct

# Load model
model = SentenceTransformer('all-MiniLM-L6-v2')

# Convert to CoreML
coreml_model = ct.convert(
    model.to_torchscript(),
    inputs=[ct.TensorType(name="text", shape=(1, 512))],
    outputs=[ct.TensorType(name="embedding", shape=(1, 384))]
)

# Save
coreml_model.save("resources/models/sentence_embedder.mlmodel")
```

**Files to create:**
- `scripts/export_coreml_embedder.py`
- `resources/models/sentence_embedder.mlmodel` (generated)

---

#### 4.2 Build Swift Embedding Helper

```swift
// resources/coreml-extractor/embedder.swift
import CoreML
import NaturalLanguage

class SentenceEmbedder {
    let model: SentenceEmbedderModel

    init() throws {
        let modelURL = Bundle.main.url(
            forResource: "sentence_embedder",
            withExtension: "mlmodel"
        )!
        self.model = try SentenceEmbedderModel(contentsOf: modelURL)
    }

    func embed(_ text: String) throws -> [Float] {
        let input = SentenceEmbedderInput(text: text)
        let output = try model.prediction(input: input)
        return output.embedding
    }

    func embedBatch(_ texts: [String]) throws -> [[Float]] {
        return try texts.map { try embed($0) }
    }
}

// Command-line interface
func main() {
    let embedder = try! SentenceEmbedder()

    for line in stdin {
        let embedding = try! embedder.embed(line)
        print(embedding.map { String($0) }.joined(separator: ","))
    }
}
```

**Files to create:**
- `resources/coreml-extractor/embedder.swift`
- `resources/coreml-extractor/Package.swift` (Swift package manifest)

---

#### 4.3 Integrate CoreML Embedder with Rust

```rust
// src/embeddings/coreml.rs
pub struct CoreMLEmbedder {
    helper_path: PathBuf,
}

impl CoreMLEmbedder {
    pub fn new() -> Result<Self> {
        let helper_path = Path::new("target/release/coreml-embedder");
        if !helper_path.exists() {
            bail!("CoreML embedder not built. Run: cd resources/coreml-extractor && swift build -c release");
        }
        Ok(Self { helper_path: helper_path.to_path_buf() })
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let output = Command::new(&self.helper_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        // Write text to stdin
        output.stdin.unwrap().write_all(text.as_bytes())?;

        // Read embedding from stdout
        let embedding = parse_embedding(&output.stdout)?;
        Ok(embedding)
    }
}
```

**Files to create:**
- `src/embeddings/coreml.rs`

---

#### 4.4 Update Extraction Pipeline

```rust
// src/commands/session/extract.rs
pub fn extract_session_with_embeddings(
    session_path: &Path,
    embedder: &CoreMLEmbedder,
) -> Result<SessionData> {
    // 1. Extract facts (existing)
    let facts = extract_session_facts(session_path)?;

    // 2. Generate embeddings on-device (new)
    let embeddings = embedder.embed_batch(&facts.to_text_batch())?;

    // 3. Insert both into database
    insert_observations_with_embeddings(&conn, &facts, &embeddings)?;

    Ok(SessionData { facts, embeddings })
}
```

**Files to modify:**
- `src/commands/session/extract.rs`

---

#### 4.5 Add Embedding Mode Selection

```bash
# Use Rust-BERT (CPU/GPU)
patina embeddings generate --mode rust-bert

# Use CoreML (Neural Engine, on-device)
patina embeddings generate --mode coreml

# Auto-detect (prefer CoreML on macOS)
patina embeddings generate
```

**Files to modify:**
- `src/commands/embeddings/generate.rs` (add `--mode` flag)

---

### Phase 4 Deliverables

- [x] CoreML embedding model exported
- [x] Swift embedding helper built
- [x] Rust integration with CoreML embedder
- [x] Extraction pipeline generates embeddings on-device
- [x] Mode selection (rust-bert vs coreml)

**Validation:** Generate embeddings using CoreML and verify performance (~20ms/embedding on Neural Engine)

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

### Phase 4 Complete
- [x] CoreML embeddings generated on-device
- [x] Privacy maintained (no cloud calls)
- [x] Performance: <20ms per embedding

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

**Status:** Ready to begin Phase 1
**Owner:** @nicabar
**Timeline:** 4 weeks (1 week per phase)
