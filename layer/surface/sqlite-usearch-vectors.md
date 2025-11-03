---
id: sqlite-usearch-vectors
version: 1
status: design
created_date: 2025-11-02
updated_date: 2025-11-02
oxidizer: nicabar
tags: [sqlite, usearch, vectors, local-first, storage, architecture, rust, sync]
---

# Local-First Vector Storage: SQLite + USearch

## Status

**Architecture**: ✅ Finalized (local-first dual storage)
**Implementation**: ❌ Not started - design only
**Current Codebase**: Uses `DatabaseBackend` enum (technical debt, must be removed)

**Goal:** Local-first knowledge storage with fast vector search. No servers, no async, no cloud dependencies.

---

## Architectural Decision: Dual Storage (No Abstraction Layer)

### The Storage Split

**SQLite** for structured data:
- Event sourcing log (append-only history)
- Relational queries (joins, filtering)
- Code symbols metadata
- Pattern documents
- Session records

**USearch** for vectors:
- Embedding similarity search (ANN via HNSW)
- 10x faster than FAISS
- Memory-mapped on-disk indices
- Custom distance metrics support

### Why This Split Works

**Principle**: Use the right tool for each job, no abstraction overhead.

```rust
// Direct usage - no wrapper needed
pub struct Storage {
    vectors: usearch::Index,        // Vector operations
    db: rusqlite::Connection,       // Everything else
}
```

**Benefits**:
- ✅ Each library used as designed (no fighting APIs)
- ✅ Zero abstraction overhead (direct function calls)
- ✅ 100% sync APIs (no async infection)
- ✅ Local-first by default (no network dependencies)
- ✅ Tiny footprint (~2 MB total)
- ✅ Battle-tested components (SQLite decades old, USearch production-proven)

---

## Design Pattern: Domain Wrapper Over Storage

The `scrape/code` module demonstrates the correct pattern - domain types own their storage needs:

**Key Principles:**
1. **Domain types are storage-agnostic** - `CodeSymbol`, `Belief`, `Pattern` have no SQLite/USearch coupling
2. **Storage struct owns both backends** - Single wrapper provides domain API
3. **API mirrors domain operations** - Methods named for *what* they do, not *how*
4. **No leaking storage details** - Callers never see `Connection` or `Index` directly

**Example pattern:**
```rust
// Domain types (storage-agnostic)
pub struct Belief {
    id: Uuid,
    content: String,
    embedding: Vec<f32>,
    metadata: BeliefMetadata,
}

// Storage wrapper (owns SQLite + USearch)
pub struct BeliefStorage {
    vectors: usearch::Index,
    db: rusqlite::Connection,
}

impl BeliefStorage {
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Belief>> {
        // 1. Embed query
        let embedding = embed(query)?;

        // 2. Vector search in USearch
        let matches = self.vectors.search(&embedding, limit)?;

        // 3. Hydrate from SQLite
        let ids: Vec<String> = matches.iter().map(|m| m.key.to_string()).collect();
        let beliefs = self.load_by_ids(&ids)?;

        Ok(beliefs)
    }

    pub fn insert(&mut self, belief: &Belief) -> Result<()> {
        // 1. Append event to SQLite (source of truth)
        self.db.execute(
            "INSERT INTO beliefs (id, content, metadata) VALUES (?1, ?2, ?3)",
            params![belief.id, belief.content, serde_json::to_string(&belief.metadata)?],
        )?;

        // 2. Update vector index
        self.vectors.add(belief.id.as_u128() as u64, &belief.embedding)?;

        Ok(())
    }
}
```

This pattern applies to all domain modules: `SemanticSearch`, `EmbeddingsDatabase`, scrape `Database`.

---

## USearch Integration

### Why USearch?

**Performance:**
- 10x faster than FAISS (industry standard)
- 20x faster exact search than naive methods
- Handles 100M vectors (far beyond Patina's needs)

**Local-First Perfect:**
- Single C++11 header (<1 MB compiled)
- Memory-mapped on-disk indices (no load into RAM)
- Sync API (no async, no tokio)
- Zero network dependencies
- Serialization built-in (`save()`, `load()`, `view()`)

**Unique Capabilities:**
- User-defined distance metrics (custom similarity functions)
- Fuzzy semantic joins (find similar items across datasets)
- Hardware-agnostic quantization (f16, i8 for smaller indices)
- Built-in clustering

**Production Ready:**
- Company-backed (Unum Cloud)
- Active development (v2.21.2)
- Native Rust bindings (not FFI wrapper)

### Basic Usage

```rust
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

// Initialize index
let mut options = IndexOptions::default();
options.dimensions = 384;  // nomic-embed-text dimensionality
options.metric = MetricKind::Cos;  // Cosine similarity
options.quantization = ScalarKind::F32;

let mut index = Index::new(&options)?;

// Add vectors
index.add(belief_id as u64, &embedding)?;

// Search
let results = index.search(&query_embedding, 10)?;  // Top 10 matches

// Persistence
index.save("beliefs.usearch")?;  // Save to disk
index.view("beliefs.usearch")?;  // Memory-map (no load)
```

---

## Module Structure

```
src/
├── storage/
│   ├── mod.rs           # Storage abstraction exports
│   ├── types.rs         # Shared types (VectorMatch, etc.)
│   ├── beliefs.rs       # BeliefStorage (USearch + SQLite)
│   ├── patterns.rs      # PatternStorage (USearch + SQLite)
│   └── symbols.rs       # SymbolStorage (USearch + SQLite)
│
├── query/
│   ├── mod.rs           # SemanticSearch wrapper
│   └── types.rs         # Query result types
│
├── embeddings/
│   ├── mod.rs
│   ├── engine.rs        # EmbeddingEngine trait (sync)
│   └── database.rs      # EmbeddingsDatabase (stores generated embeddings)
│
└── commands/
    ├── scrape/
    │   └── code.rs      # Uses SymbolStorage
    └── search/
        └── semantic.rs  # Uses BeliefStorage
```

---

## Storage Implementation

### Dual Storage Wrapper

```rust
//! src/storage/beliefs.rs

use rusqlite::{Connection, params};
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

pub struct BeliefStorage {
    vectors: Index,
    db: Connection,
}

impl BeliefStorage {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let base = path.as_ref();

        // Open SQLite (event log + metadata)
        let db = Connection::open(base.join("beliefs.db"))?;
        Self::init_schema(&db)?;

        // Open USearch (vector index)
        let mut options = IndexOptions::default();
        options.dimensions = 384;
        options.metric = MetricKind::Cos;
        options.quantization = ScalarKind::F32;

        let index = Index::new(&options)?;
        let index_path = base.join("beliefs.usearch");
        if index_path.exists() {
            index.view(&index_path)?;  // Memory-map existing
        }

        Ok(Self { vectors: index, db })
    }

    pub fn search(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<Belief>> {
        // Vector search
        let matches = self.vectors.search(query_embedding, limit)?;

        // Hydrate from SQLite
        let ids: Vec<String> = matches.iter()
            .map(|m| m.key.to_string())
            .collect();

        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("SELECT * FROM beliefs WHERE id IN ({})", placeholders);

        let mut stmt = self.db.prepare(&query)?;
        let beliefs = stmt.query_map(
            rusqlite::params_from_iter(ids.iter()),
            |row| Ok(Belief {
                id: row.get(0)?,
                content: row.get(1)?,
                metadata: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                embedding: vec![], // Don't load embeddings in results
            })
        )?.collect::<Result<Vec<_>, _>>()?;

        Ok(beliefs)
    }

    pub fn insert(&mut self, belief: &Belief) -> Result<()> {
        // SQLite (source of truth)
        self.db.execute(
            "INSERT INTO beliefs (id, content, metadata, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                belief.id.to_string(),
                &belief.content,
                serde_json::to_string(&belief.metadata)?,
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;

        // USearch (vector index)
        self.vectors.add(belief.id.as_u128() as u64, &belief.embedding)?;

        Ok(())
    }

    pub fn save_index(&self) -> Result<()> {
        self.vectors.save("beliefs.usearch")?;
        Ok(())
    }
}
```

---

## Domain Layer: SemanticSearch

**Pattern**: Wrapper around storage, provides domain operations.

```rust
//! src/query/mod.rs

pub struct SemanticSearch {
    storage: BeliefStorage,
    embedder: Box<dyn EmbeddingEngine>,
}

impl SemanticSearch {
    pub fn new(storage_path: &Path, embedder: Box<dyn EmbeddingEngine>) -> Result<Self> {
        let storage = BeliefStorage::open(storage_path)?;
        Ok(Self { storage, embedder })
    }

    pub fn search_beliefs(&mut self, query: &str, top_k: usize) -> Result<Vec<Belief>> {
        // Embed query (sync)
        let embedding = self.embedder.embed(query)?;

        // Vector search (sync, local)
        self.storage.search(&embedding, top_k)
    }

    pub fn add_belief(&mut self, content: &str) -> Result<()> {
        let embedding = self.embedder.embed(content)?;

        let belief = Belief {
            id: Uuid::new_v4(),
            content: content.to_string(),
            embedding,
            metadata: BeliefMetadata::default(),
        };

        self.storage.insert(&belief)?;
        self.storage.save_index()?;  // Persist vector index

        Ok(())
    }
}
```

---

## Dependencies

```toml
[dependencies]
# Core
anyhow = "1.0"
uuid = { version = "1.0", features = ["v4"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"

# Storage
rusqlite = { version = "0.32", features = ["bundled"] }
usearch = "2.21"

# Embeddings (separate concern)
# ... embedding engine dependencies
```

**Total compiled size:** ~2-3 MB (SQLite ~1 MB, USearch <1 MB, overhead)

---

## Async Containment

**Core principle**: No async in Patina.

- ✅ SQLite is sync (`rusqlite` crate)
- ✅ USearch is sync (C++ bindings, no async)
- ✅ All domain logic is sync
- ✅ All commands are sync
- ✅ Embedding engines are sync

---

## Implementation Phases

### Phase 1: Remove DatabaseBackend Enum
- Delete `src/db/backend.rs` entirely
- Remove all `DatabaseBackend` usage from domain wrappers
- Keep existing `SqliteDatabase` for now

### Phase 2: Add USearch Integration
- Add `usearch = "2.21"` dependency
- Create `src/storage/mod.rs` with `BeliefStorage` wrapper
- Implement vector search alongside SQLite queries

### Phase 3: Migrate Domain Types
- Update `SemanticSearch` to use new storage
- Update `EmbeddingsDatabase` to use new storage
- Update scrape `Database` to use new storage

### Phase 4: Remove Old Abstractions
- Remove old `src/db/` wrapper code
- Direct SQLite + USearch usage everywhere

### Phase 5: Testing & Documentation
- Unit tests for storage layer
- Integration tests for search
- Performance benchmarks (USearch vs old sqlite-vec)

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_belief_storage_roundtrip() {
    let temp = TempDir::new().unwrap();
    let mut storage = BeliefStorage::open(temp.path()).unwrap();

    let belief = Belief {
        id: Uuid::new_v4(),
        content: "Rust ownership prevents memory bugs".to_string(),
        embedding: vec![0.1; 384],
        metadata: BeliefMetadata::default(),
    };

    storage.insert(&belief).unwrap();
    storage.save_index().unwrap();

    let results = storage.search(&belief.embedding, 1).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].content, belief.content);
}
```

### Integration Tests

```rust
#[test]
fn test_semantic_search_workflow() {
    let temp = TempDir::new().unwrap();
    let embedder = Box::new(MockEmbedder::new());
    let mut search = SemanticSearch::new(temp.path(), embedder).unwrap();

    search.add_belief("Rust prevents data races").unwrap();
    search.add_belief("Python is dynamically typed").unwrap();

    let results = search.search_beliefs("memory safety", 1).unwrap();
    assert!(results[0].content.contains("Rust"));
}
```

---

## Success Criteria

**Implementation complete when:**

- [ ] **DatabaseBackend enum removed** - No abstraction layer, direct library usage
- [ ] **USearch integrated** - Vector search via `usearch` crate, memory-mapped indices
- [ ] **SQLite preserved** - Event log, metadata, relational queries
- [ ] **Domain types migrated** - `SemanticSearch`, `EmbeddingsDatabase`, scrape `Database` use new storage
- [ ] **100% sync codebase** - No async, no tokio, no `.await`
- [ ] **Tests pass** - Unit tests for storage, integration tests for workflows
- [ ] **Performance validated** - USearch faster than old sqlite-vec baseline
- [ ] **Documentation complete** - Architecture guide, API docs, examples

---

## Migration from Current Codebase

**Current state:**
- `DatabaseBackend` enum with SQLite variant
- sqlite-vec extension for vector search
- Domain wrappers use `DatabaseBackend`

**Migration path:**

1. **Keep existing code working** - Don't break current functionality
2. **Add USearch alongside** - New storage layer in parallel
3. **Migrate domain types one-by-one** - SemanticSearch first, then others
4. **Remove old abstractions** - Once migration complete, delete `DatabaseBackend`
5. **Remove sqlite-vec dependency** - USearch replaces it

**Git strategy:**
- Commit each phase separately
- Keep working code at each commit
- Tag when migration complete

---

## References

- USearch: https://github.com/unum-cloud/usearch
- USearch Rust docs: https://docs.rs/usearch/latest/usearch/
- SQLite: https://www.sqlite.org/
- rusqlite: https://docs.rs/rusqlite/latest/rusqlite/
- Memista (USearch + SQLite example): https://github.com/sokratis-xyz/memista
