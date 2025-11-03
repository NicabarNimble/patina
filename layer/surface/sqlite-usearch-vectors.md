---
id: sqlite-usearch-vectors
version: 2
status: active
created_date: 2025-11-02
updated_date: 2025-11-03
oxidizer: nicabar
tags: [sqlite, usearch, vectors, local-first, storage, architecture, rust, sync]
---

# Local-First Vector Storage: SQLite + USearch

## Status

**Architecture**: ✅ Finalized (local-first dual storage)
**Implementation**: 🚧 In progress - Phase 2 complete (BeliefStorage implemented)
**Current Codebase**:
- ✅ Phase 1 complete: DatabaseBackend enum removed, direct SqliteDatabase usage
- ✅ Phase 2 complete: BeliefStorage with SQLite + USearch hybrid storage
- ⏳ Phase 3 pending: Migrate remaining domain types

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

**Current state (Phase 2 complete):**

```
src/
├── storage/              # ✅ NEW: Dual storage layer
│   ├── mod.rs           # ✅ Storage module exports
│   ├── types.rs         # ✅ Domain types (Belief, BeliefMetadata, SearchResult)
│   └── beliefs.rs       # ✅ BeliefStorage (SQLite + USearch)
│
├── db/                   # ✅ Kept for Phase 3 compatibility
│   ├── sqlite.rs        # ✅ SqliteDatabase wrapper (kept)
│   ├── vectors.rs       # ⏳ Legacy vector ops (to be removed Phase 4)
│   └── mod.rs           # ✅ Simplified exports
│
├── query/
│   ├── mod.rs
│   └── semantic_search.rs  # ⏳ Uses SqliteDatabase (Phase 3: migrate to BeliefStorage)
│
├── embeddings/
│   ├── mod.rs
│   ├── engine.rs        # EmbeddingEngine trait (sync)
│   ├── onnx.rs          # ONNX embedder
│   └── database.rs      # ⏳ Uses SqliteDatabase (Phase 3: migrate to new pattern)
│
└── commands/
    └── scrape/
        └── code/
            └── database.rs  # ⏳ Uses SqliteDatabase (Phase 3: migrate to new pattern)
```

**Future (Phase 3+):**
- Add `storage/patterns.rs` for PatternStorage
- Add `storage/observations.rs` for ObservationStorage
- Add `storage/symbols.rs` for CodeSymbolStorage
- Migrate SemanticSearch to use BeliefStorage
- Remove `src/db/vectors.rs` (sqlite-vec)

---

## Storage Implementation

### Dual Storage Wrapper

**Status**: ✅ Implemented in `src/storage/beliefs.rs`

```rust
//! src/storage/beliefs.rs

use rusqlite::{Connection, params};
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};
use std::path::PathBuf;

pub struct BeliefStorage {
    vectors: Index,
    db: Connection,
    index_path: PathBuf,
}

impl BeliefStorage {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let base = path.as_ref();
        std::fs::create_dir_all(base)?;

        // Open SQLite (event log + metadata)
        let db = Connection::open(base.join("beliefs.db"))?;
        Self::init_schema(&db)?;

        // Open USearch (vector index)
        let mut options = IndexOptions::default();
        options.dimensions = 384;
        options.metric = MetricKind::Cos;
        options.quantization = ScalarKind::F32;

        let mut index = Index::new(&options)?;
        index.reserve(1000)?;  // Reserve initial capacity

        let index_path = base.join("beliefs.usearch");
        if index_path.exists() {
            index.view(index_path.to_str().unwrap())?;  // Memory-map existing
        }

        Ok(Self { vectors: index, db, index_path })
    }

    fn init_schema(db: &Connection) -> Result<()> {
        db.execute(
            "CREATE TABLE IF NOT EXISTS beliefs (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT UNIQUE NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                created_at TEXT NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    pub fn search(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<Belief>> {
        // Vector search in USearch
        let matches = self.vectors.search(query_embedding, limit)?;

        // Hydrate from SQLite using rowid
        let mut beliefs = Vec::new();
        for rowid in matches.keys {
            if let Some(belief) = self.load_by_rowid(rowid as i64)? {
                beliefs.push(belief);
            }
        }

        Ok(beliefs)
    }

    pub fn insert(&mut self, belief: &Belief) -> Result<()> {
        // SQLite (source of truth) - atomically insert and retrieve rowid
        let rowid: i64 = self.db.query_row(
            "INSERT INTO beliefs (id, content, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4)
             RETURNING rowid",
            params![
                belief.id.to_string(),
                &belief.content,
                serde_json::to_string(&belief.metadata)?,
                belief.metadata.created_at.unwrap_or_else(chrono::Utc::now).to_rfc3339(),
            ],
            |row| row.get(0),
        )?;

        // USearch (vector index) - use rowid as key
        self.vectors.add(rowid as u64, &belief.embedding)?;

        Ok(())
    }

    fn load_by_rowid(&self, rowid: i64) -> Result<Option<Belief>> {
        let result = self.db.query_row(
            "SELECT id, content, metadata FROM beliefs WHERE rowid = ?1",
            params![rowid],
            |row| {
                let metadata_str: String = row.get(2)?;
                Ok(Belief {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    content: row.get(1)?,
                    embedding: vec![], // Don't load embeddings in results
                    metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
                })
            },
        );

        match result {
            Ok(belief) => Ok(Some(belief)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save_index(&self) -> Result<()> {
        self.vectors.save(self.index_path.to_str().unwrap())?;
        Ok(())
    }
}
```

**Key Implementation Details:**

- **rowid as USearch key**: SQLite's auto-incrementing rowid provides stable i64 keys for USearch (avoids UUID→u64 truncation)
- **RETURNING clause**: Atomic insert with `RETURNING rowid` retrieves the new rowid in a single statement
- **Immutable beliefs**: Insert-only (no upsert) maintains consistency between SQLite and USearch indices
- **Memory-mapped indices**: `index.view()` loads existing indices without copying to RAM
- **Dual storage pattern**: SQLite is source of truth, USearch provides fast ANN search
- **Storage separation**: `beliefs.db` for metadata, `beliefs.usearch` for vectors

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

### Phase 1: Remove DatabaseBackend Enum ✅ COMPLETE
**Completed:** 2025-11-03

- ✅ Deleted `src/db/backend.rs`
- ✅ Deleted `src/db/config.rs` (DatabaseConfig system)
- ✅ Removed `DatabaseBackend` from SemanticSearch
- ✅ Removed `DatabaseBackend` from EmbeddingsDatabase
- ✅ Removed `DatabaseBackend` from scrape Database
- ✅ Updated examples and tests to use SqliteDatabase directly
- ✅ Updated mod.rs exports

**Commits:** 9 focused commits, all tests passing (39→39 passing)

### Phase 2: Add USearch Integration ✅ COMPLETE
**Completed:** 2025-11-03

- ✅ Added `usearch = "2.21"` dependency
- ✅ Created `src/storage/` module structure
- ✅ Implemented `BeliefStorage` with SQLite + USearch dual storage
- ✅ Implemented rowid-based key scheme (avoids UUID truncation)
- ✅ Added 4 comprehensive unit tests (creation, roundtrip, ranking, direct USearch)
- ✅ Memory-mapped indices working

**Commits:** 3 focused commits, tests passing (39→46 passing)

**Key Discoveries:**
- Using SQLite's auto-incrementing rowid as USearch key solves UUID→u64 truncation
- RETURNING clause provides atomic insert with rowid retrieval (cleaner than last_insert_rowid())

### Phase 3: Migrate Domain Types ⏳ PENDING
- Update `SemanticSearch` to use BeliefStorage
- Update `EmbeddingsDatabase` to use new storage pattern
- Update scrape `Database` to use new storage pattern
- Create storage wrappers for patterns, observations, code symbols

### Phase 4: Remove Old Abstractions ⏳ PENDING
- Remove sqlite-vec dependency
- Remove old vector tables setup
- Direct SQLite + USearch usage everywhere

### Phase 5: Testing & Documentation ⏳ PENDING
- Integration tests for SemanticSearch with new storage
- Performance benchmarks (USearch vs old sqlite-vec)
- Update API documentation

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

- [x] **DatabaseBackend enum removed** - ✅ Completed Phase 1 (2025-11-03)
- [x] **USearch integrated** - ✅ Completed Phase 2 (2025-11-03), memory-mapped indices working
- [x] **SQLite preserved** - ✅ SqliteDatabase kept, event log pattern in BeliefStorage
- [ ] **Domain types migrated** - ⏳ Pending Phase 3 (`SemanticSearch`, `EmbeddingsDatabase`, scrape `Database`)
- [x] **100% sync codebase** - ✅ No async, all storage operations are sync
- [x] **Tests pass** - ✅ 46 tests passing (4 new storage tests added)
- [ ] **Performance validated** - ⏳ Pending benchmarks (USearch vs sqlite-vec)
- [x] **Documentation complete** - ✅ Design doc updated with implementation reality

---

## Migration from Current Codebase

**Original state (Nov 2):**
- `DatabaseBackend` enum with SQLite variant
- sqlite-vec extension for vector search
- Domain wrappers use `DatabaseBackend`

**Current state (Nov 3 - Phase 2 complete):**
- ✅ `DatabaseBackend` enum removed
- ✅ New `src/storage/` module with BeliefStorage
- ✅ USearch integrated alongside sqlite-vec
- ⏳ Domain wrappers still use `SqliteDatabase` (legacy)
- ⏳ sqlite-vec still present (to be removed Phase 4)

**Migration path (completed/in-progress):**

1. ✅ **Remove DatabaseBackend abstraction** - Completed Phase 1 (9 commits)
2. ✅ **Add USearch alongside** - Completed Phase 2 (2 commits), BeliefStorage working
3. ⏳ **Migrate domain types one-by-one** - Phase 3 pending (SemanticSearch first, then others)
4. ⏳ **Remove old abstractions** - Phase 4 pending (delete sqlite-vec, old vector tables)
5. ⏳ **Performance validation** - Phase 5 pending (benchmarks)

**Git strategy (followed):**
- ✅ Each phase committed separately with clear messages
- ✅ Working code at each commit (46 tests passing)
- ✅ No breaking changes to existing functionality yet

---

## References

- USearch: https://github.com/unum-cloud/usearch
- USearch Rust docs: https://docs.rs/usearch/latest/usearch/
- SQLite: https://www.sqlite.org/
- rusqlite: https://docs.rs/rusqlite/latest/rusqlite/
- Memista (USearch + SQLite example): https://github.com/sokratis-xyz/memista
