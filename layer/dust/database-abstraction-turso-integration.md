---
id: database-abstraction-turso-integration
version: 7
status: design
created_date: 2025-11-01
updated_date: 2025-11-02
oxidizer: nicabar
tags: [turso, database, architecture, distributed, vectors, rust, async, dual-api, scrape-code-pattern]
---

# Database Backend Strategy: Turso-First with SQLite Fallback

## ⚠️ Implementation Status

**Architecture**: ✅ Finalized (dual API strategy with feature flags)
**Implementation**: ❌ Not started - design only
**Current Codebase**: Uses `DatabaseBackend` enum (technical debt from early exploration, must be removed)

**Key Decision**: The enum approach currently in codebase contradicts this design. It was explored and rejected in session 20251102-081406 after architectural review revealed sync wrapper over async is an anti-pattern. This document defines the correct path forward: separate sync (SQLite) and async (Turso) implementations via feature flags.

**Goal:** Enable distributed development knowledge through Turso (async), with SQLite (sync) as a simple fallback option for users who don't need distributed features.

**Why Turso as Default:**
- 🌐 **Distributed sync** - Knowledge across machines/teams (Patina's core vision)
- 🔍 **Native vectors** - First-class `vector_distance_cos()` (cleaner than sqlite-vec extension)
- 🦀 **Rust-native** - Written in Rust, clean FFI, modern ecosystem
- 🔄 **SQLite compatible** - Easy migration path
- ⚡ **Async-first** - Designed for concurrent I/O, network sync

**Why SQLite as Fallback:**
- 📦 **Zero dependencies** - No network, no tokio, pure sync
- 🎯 **Simplicity** - For users who just want local-only search
- 🔒 **Battle-tested** - Decades of reliability

---

## Architectural Decision: Dual API (No Wrapper)

### The Problem with Sync Wrapper Over Async Backend

**Initial approach considered**: Wrap Turso's async API in sync wrapper using `block_on()`.

**Critical realization**: This is fighting Turso's design, not using it as intended.

**Issues with wrapper approach**:
1. ❌ Runtime overhead (`block_on()` on every call)
2. ❌ Deadlock risks (nested async contexts)
3. ❌ Lost concurrency benefits (defeats async's purpose)
4. ❌ Resource waste (global runtime or per-instance runtime overhead)
5. ❌ Architectural smell (forcing sync over async = impedance mismatch)

### Solution: Separate Sync and Async APIs

**Key insight**: Don't make Patina's core care about sync vs async. Provide **two clean implementations** and let users choose via feature flags.

```rust
// Turso backend (default) - Pure async, no wrapper
#[cfg(feature = "turso")]
impl SemanticSearch<TursoDatabase> {
    pub async fn search_beliefs(&mut self, query: &str) -> Result<Vec<...>> {
        // Direct async/await - Turso used as designed
    }
}

// SQLite backend (fallback) - Pure sync, no tokio
#[cfg(feature = "sqlite")]
impl SemanticSearch<SqliteDatabase> {
    pub fn search_beliefs(&mut self, query: &str) -> Result<Vec<...>> {
        // Direct sync call - rusqlite used as designed
    }
}
```

**Benefits**:
- ✅ Use each backend **as designed** (no fighting the ecosystem)
- ✅ Zero runtime overhead (no wrapper layer)
- ✅ No deadlock risks (no nested async)
- ✅ Clean separation (Turso = async, SQLite = sync)
- ✅ User chooses complexity level (features flags)

---

## Design Pattern: Domain Wrapper Over Backend

The `scrape/code` module demonstrates the correct pattern for database abstraction:

**Key Principles:**
1. **Domain types are backend-agnostic** - `CodeSymbol`, `FunctionFact` work with any backend
2. **Database struct owns the backend** - No leaking of `Connection` or async runtime
3. **API mirrors domain operations** - Methods named for *what* they do, not *how*
4. **Same API shape across backends** - Only difference is `async` vs sync

**Example pattern:**
```rust
// Domain types (backend-agnostic)
pub struct CodeSymbol { path: String, name: String, kind: String, ... }

// Backend-specific implementations
#[cfg(feature = "sqlite")]
pub struct Database { db: SqliteDatabase }

#[cfg(feature = "sqlite")]
impl Database {
    pub fn insert_symbols(&mut self, symbols: &[CodeSymbol]) -> Result<()>  // sync
}

#[cfg(feature = "turso")]
pub struct Database { db: TursoDatabase }

#[cfg(feature = "turso")]
impl Database {
    pub async fn insert_symbols(&mut self, symbols: &[CodeSymbol]) -> Result<()>  // async
}
```

This pattern applies to all domain modules: `SemanticSearch`, `EmbeddingsDatabase`, scrape `Database`.

---

## Implementation Strategy

### Feature Flags

```toml
[features]
default = ["turso"]

# Primary backend (async, distributed)
turso = ["dep:turso", "dep:tokio", "dep:once_cell"]

# Fallback backend (sync, local-only)
sqlite = ["dep:rusqlite", "dep:sqlite-vec"]
```

**Usage**:
```bash
# Default: Turso (async, distributed)
cargo build

# Fallback: SQLite (sync, simple)
cargo build --no-default-features --features sqlite

# Both enabled (advanced use case - TBD if needed)
cargo build --features sqlite,turso
```

### Dependencies

```toml
[dependencies]
# Core (always present)
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"

# Turso backend (default)
turso = { version = "0.2", optional = true }
tokio = { version = "1.0", features = ["rt-multi-thread"], optional = true }
once_cell = { version = "1.19", optional = true }

# SQLite backend (optional fallback)
rusqlite = { version = "0.32", optional = true }
sqlite-vec = { version = "0.1", optional = true }
```

---

## Module Structure

```
src/
├── db/
│   ├── mod.rs           # Conditional exports based on features
│   ├── types.rs         # Shared types (VectorTable, VectorMatch, etc.)
│   ├── config.rs        # Config system (backend-agnostic)
│   ├── turso.rs         # Async Turso implementation (default)
│   └── sqlite.rs        # Sync SQLite implementation (fallback)
│
├── query/
│   ├── mod.rs           # Exports based on features
│   ├── types.rs         # Shared result types
│   ├── turso.rs         # async impl SemanticSearch<TursoDatabase>
│   └── sqlite.rs        # sync impl SemanticSearch<SqliteDatabase>
│
├── embeddings/
│   ├── mod.rs
│   ├── engine.rs        # Backend-agnostic EmbeddingEngine trait
│   ├── turso.rs         # async impl EmbeddingsDatabase<TursoDatabase>
│   └── sqlite.rs        # sync impl EmbeddingsDatabase<SqliteDatabase>
│
└── commands/
    ├── scrape/
    │   ├── turso.rs     # async impl for Turso
    │   └── sqlite.rs    # sync impl for SQLite
    └── embeddings/
        ├── turso.rs     # async impl
        └── sqlite.rs    # sync impl
```

---

## Backend Implementations

### Turso Backend (Default, Async)

```rust
//! src/db/turso.rs - Async API, uses Turso as designed

use turso::{Builder, Connection};

pub struct TursoDatabase {
    conn: Connection,
}

impl TursoDatabase {
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Builder::new_local(path.as_ref().to_str().unwrap())
            .build()
            .await?;
        Ok(Self { conn: db.connect()? })
    }

    pub async fn open_embedded(path: P, url: &str, auth_token: &str) -> Result<Self> {
        let db = Builder::new_local(path)
            .url(url)
            .auth_token(auth_token)
            .build()
            .await?;
        let conn = db.connect()?;
        conn.sync().await?;  // Initial sync
        Ok(Self { conn })
    }

    pub async fn vector_search(...) -> Result<Vec<VectorMatch>> {
        // Turso native vector syntax (cleaner than sqlite-vec!)
        let sql = "SELECT rowid, vector_distance_cos(embedding, vector32(?)) as distance
                   FROM beliefs ORDER BY distance LIMIT ?";
        // ... async query execution
    }

    pub async fn sync(&self) -> Result<()> {
        self.conn.sync().await  // Distributed sync!
    }
}
```

### SQLite Backend (Fallback, Sync)

```rust
//! src/db/sqlite.rs - Sync API, no async/tokio

use rusqlite::Connection;

pub struct SqliteDatabase {
    conn: Connection,
}

impl SqliteDatabase {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::load_vec_extension(&conn)?;
        Ok(Self { conn })
    }

    pub fn vector_search(...) -> Result<Vec<VectorMatch>> {
        // sqlite-vec MATCH syntax
        let sql = "SELECT rowid, distance FROM beliefs
                   WHERE embedding MATCH ? ORDER BY distance LIMIT ?";
        // ... sync query execution
    }
}
```

---

## Domain Layer: Generic Over Backend

**Pattern**: Generic struct with conditional implementations based on feature flags.

```rust
//! src/query/mod.rs

pub struct SemanticSearch<DB> {
    db: DB,
    embedder: Box<dyn EmbeddingEngine>,
}

// ============================================================================
// TURSO IMPLEMENTATION (Async, Default)
// ============================================================================

#[cfg(feature = "turso")]
impl SemanticSearch<TursoDatabase> {
    pub async fn search_beliefs(&mut self, query: &str, top_k: usize) -> Result<Vec<...>> {
        let embedding = self.embedder.embed(query)?;  // sync
        self.db.vector_search(VectorTable::Beliefs, &embedding, None, top_k).await  // async
    }

    /// Scoped concurrency - no 'static lifetime issues
    pub async fn search_many(&mut self, queries: &[&str], top_k: usize) -> Result<Vec<...>> {
        use futures::future::join_all;
        let searches = queries.iter().map(|q| self.search_beliefs(q, top_k));
        join_all(searches).await.into_iter().collect()
    }
}

// ============================================================================
// SQLITE IMPLEMENTATION (Sync, Fallback)
// ============================================================================

#[cfg(feature = "sqlite")]
impl SemanticSearch<SqliteDatabase> {
    pub fn search_beliefs(&mut self, query: &str, top_k: usize) -> Result<Vec<...>> {
        let embedding = self.embedder.embed(query)?;
        self.db.vector_search(VectorTable::Beliefs, &embedding, None, top_k)  // sync
    }
}
```

**Key difference**: Only `async` vs sync - same API shape, same domain logic.

---

## User Experience

### Default: Turso (Async, Distributed)

```rust
// Cargo.toml: patina = "0.1"  (turso is default)
// main.rs
#[tokio::main]
async fn main() -> Result<()> {
    let db = TursoDatabase::open(".patina/db/facts.db").await?;
    let mut search = SemanticSearch::new(db, embedder).await;
    let results = search.search_beliefs("rust cli tools", 10).await?;
    // ✅ Concurrent queries, distributed sync, native vectors
}
```

### Fallback: SQLite (Sync, Simple)

```rust
// Cargo.toml: patina = { version = "0.1", default-features = false, features = ["sqlite"] }
// main.rs
fn main() -> Result<()> {
    let db = SqliteDatabase::open(".patina/db/facts.db")?;
    let mut search = SemanticSearch::new(db, embedder);
    let results = search.search_beliefs("rust cli tools", 10)?;
    // ✅ Zero async, no tokio, smaller binary, local-only
}
```

---

## Protecting Against "No Boilerplate" Async Concerns

### For Turso Users Who Opt Into Async

**Problem**: Tokio can force `'static` lifetimes when spawning tasks.

**Solution**: Use scoped concurrency patterns (futures that complete before scope ends).

#### ❌ BAD - Spawned Tasks (Requires 'static)

```rust
pub async fn search_multiple(&self, queries: &[&str]) -> Result<Vec<...>> {
    for query in queries {
        tokio::spawn(async move {
            self.search(query).await  // ❌ 'static lifetime required
        });
    }
}
```

#### ✅ GOOD - Scoped Concurrency (No 'static)

```rust
pub async fn search_multiple(&self, queries: &[&str]) -> Result<Vec<...>> {
    use futures::future::join_all;

    // Futures complete before function returns - no 'static needed
    let searches = queries.iter().map(|q| self.search(q));
    let results = join_all(searches).await;

    Ok(results)
}
```

#### ✅ GOOD - Explicit Scoping

```rust
pub async fn search_multiple(&self, queries: &[&str]) -> Result<Vec<...>> {
    use tokio::task;

    tokio::task::scope(|scope| async {
        for query in queries {
            scope.spawn(async move {
                self.search(query).await  // ✅ Scope ensures completion
            });
        }
    }).await
}
```

### Documentation Strategy

```rust
//! # Async Best Practices (Turso Backend)
//!
//! When using Turso's async API, follow these patterns to avoid
//! 'static lifetime issues:
//!
//! ## 1. Use `join!` or `join_all` for Concurrent Operations
//!
//! ```rust
//! let (beliefs, patterns) = tokio::join!(
//!     search.search_beliefs("rust", 10),
//!     search.search_observations("patterns", None, 10),
//! );
//! ```
//!
//! ## 2. Avoid `tokio::spawn` with Borrowed Data
//!
//! Use scoped tasks or join patterns instead.
//!
//! ## 3. Prefer Futures Over Spawned Tasks
//!
//! Futures complete before scope ends - no 'static required.
//!
//! See: No Boilerplate's "Async in Rust" for reasoning.
```

---

## Feature Flag Strategy

**Decision**: Mutually exclusive features (one backend per build)

```rust
// src/lib.rs - Compile-time enforcement
#[cfg(all(feature = "turso", feature = "sqlite"))]
compile_error!("Features 'turso' and 'sqlite' are mutually exclusive. Choose one.");
```

**Rationale**: Simpler mental model, smaller binaries, clearer intent. Can relax later if use cases emerge (e.g., migration tool).

---

## Implementation Phases

### Phase 1: Remove Enum & Add Feature Flags
- Remove `src/db/backend.rs` entirely
- Add feature flags: `default = ["turso"]`, `turso`, `sqlite` (mutually exclusive)
- Make dependencies optional, wrap `SqliteDatabase` in `#[cfg(feature = "sqlite")]`

### Phase 2: Implement Turso Backend
- Create `src/db/turso.rs` with async API (`TursoDatabase`)
- Native vector support using `vector_distance_cos()`
- Embedded replica mode support

### Phase 3: Refactor Domain Types to Generic
- Make domain types generic: `SemanticSearch<DB>`, `EmbeddingsDatabase<DB>`, scrape `Database<DB>`
- Conditional impls: async for turso, sync for sqlite
- Remove all `DatabaseBackend` enum usage

### Phase 4: Update Commands & CLI
- Conditional compilation in command modules
- async handlers for turso, sync handlers for sqlite

### Phase 5: Testing & Documentation
- CI matrix for both features
- Async best practices guide
- Feature selection documentation

---

## Testing Strategy

### CI Matrix

```yaml
test:
  strategy:
    matrix:
      backend: [turso, sqlite]
  steps:
    - name: Test ${{ matrix.backend }}
      run: |
        cargo test --no-default-features --features ${{ matrix.backend }}
```

### Backend-Specific Tests

```rust
// tests/turso_integration.rs
#![cfg(feature = "turso")]

#[tokio::test]
async fn test_turso_vector_search() {
    let db = TursoDatabase::open(":memory:").await.unwrap();
    // ... test async API
}

// tests/sqlite_integration.rs
#![cfg(feature = "sqlite")]

#[test]
fn test_sqlite_vector_search() {
    let db = SqliteDatabase::open(":memory:").unwrap();
    // ... test sync API
}
```

---

## Success Criteria

**Implementation complete when:**

- [ ] **DatabaseBackend enum removed** - No runtime dispatch, feature flags only
- [ ] **Turso backend functional** - `TursoDatabase` with async API, native `vector_distance_cos()`, embedded replica mode
- [ ] **SQLite backend preserved** - Wrapped in `#[cfg(feature = "sqlite")]`, no tokio dependency
- [ ] **Domain types generic** - `SemanticSearch<DB>`, `EmbeddingsDatabase<DB>`, scrape `Database<DB>` with conditional async/sync impls
- [ ] **Feature flags enforced** - Mutually exclusive, default=turso, both build independently
- [ ] **Tests pass for both** - CI matrix tests turso and sqlite features separately
- [ ] **Documentation complete** - Feature selection guide, async best practices, migration examples

---

## References

- Turso crate: https://crates.io/crates/turso
- Our scraped Turso codebase: `layer/dust/repos/turso/`
- No Boilerplate async critique: Informs our scoped concurrency approach
- tokio runtime docs: Global runtime pattern for libraries
