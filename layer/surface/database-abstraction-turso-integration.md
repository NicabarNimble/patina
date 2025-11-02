---
id: database-abstraction-turso-integration
version: 4
status: active
created_date: 2025-11-01
updated_date: 2025-11-02
oxidizer: nicabar
tags: [turso, database, architecture, distributed, vectors, rust, async, dual-api]
---

# Database Backend Strategy: Turso-First with SQLite Fallback

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
//! src/db/turso.rs
//!
//! Turso backend - asynchronous API
//! Uses Turso as designed (native async, no wrapper)

use turso::{Builder, Connection};
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

// Global runtime for Turso operations (minimal overhead)
static TURSO_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("turso-worker")
        .build()
        .expect("Failed to create Turso runtime")
});

pub struct TursoDatabase {
    conn: Connection,
}

impl TursoDatabase {
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Builder::new_local(path.as_ref().to_str().unwrap())
            .build()
            .await?;
        let conn = db.connect()?;
        Ok(Self { conn })
    }

    pub async fn open_embedded<P: AsRef<Path>>(
        path: P,
        url: &str,
        auth_token: &str,
    ) -> Result<Self> {
        let db = Builder::new_local(path.as_ref().to_str().unwrap())
            .url(url)
            .auth_token(auth_token)
            .build()
            .await?;

        let conn = db.connect()?;

        // Initial sync
        conn.sync().await?;

        Ok(Self { conn })
    }

    pub async fn execute(&self, sql: &str, params: impl IntoParams) -> Result<usize> {
        let result = self.conn.execute(sql, params).await?;
        Ok(result as usize)
    }

    pub async fn vector_search(
        &self,
        table: VectorTable,
        query_vector: &[f32],
        filter: Option<VectorFilter>,
        limit: usize,
    ) -> Result<Vec<VectorMatch>> {
        // Turso native vector syntax (cleaner than sqlite-vec!)
        let sql = format!(
            "SELECT rowid, vector_distance_cos(embedding, vector32(?)) as distance
             FROM {}
             ORDER BY distance LIMIT ?",
            table.table_name()
        );

        let mut stmt = self.conn.prepare(&sql).await?;
        let rows = stmt.query([query_vector, &limit]).await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let row_id: i64 = row.get(0)?;
            let distance: f32 = row.get(1)?;
            results.push(VectorMatch::new(row_id, distance));
        }

        Ok(results)
    }

    pub async fn sync(&self) -> Result<()> {
        self.conn.sync().await?;
        Ok(())
    }
}
```

### SQLite Backend (Fallback, Sync)

```rust
//! src/db/sqlite.rs
//!
//! SQLite backend - synchronous API
//! No async, no tokio, pure sync simplicity

use rusqlite::Connection;
use zerocopy::AsBytes;

pub struct SqliteDatabase {
    conn: Connection,
}

impl SqliteDatabase {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::load_vec_extension(&conn)?;
        Ok(Self { conn })
    }

    fn load_vec_extension(conn: &Connection) -> Result<()> {
        unsafe {
            sqlite_vec::sqlite3_auto_extension(Some(
                std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ())
            ));
            conn.load_extension_enable()?;
        }
        Ok(())
    }

    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize> {
        let count = self.conn.execute(sql, params)?;
        Ok(count)
    }

    pub fn vector_search(
        &self,
        table: VectorTable,
        query_vector: &[f32],
        filter: Option<VectorFilter>,
        limit: usize,
    ) -> Result<Vec<VectorMatch>> {
        // sqlite-vec MATCH syntax
        let sql = format!(
            "SELECT rowid, distance FROM {}
             WHERE embedding MATCH ?
             ORDER BY distance LIMIT ?",
            table.table_name()
        );

        let vector_bytes = query_vector.as_bytes();

        let mut stmt = self.conn.prepare(&sql)?;
        let results = stmt.query_map(
            params![vector_bytes, limit],
            |row| {
                let row_id: i64 = row.get(0)?;
                let distance: f32 = row.get(1)?;
                Ok(VectorMatch::new(row_id, distance))
            }
        )?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }
}
```

---

## Domain Layer: Generic Over Backend

### SemanticSearch with Conditional Implementations

```rust
//! src/query/mod.rs

use crate::embeddings::EmbeddingEngine;

pub struct SemanticSearch<DB> {
    db: DB,
    embedder: Box<dyn EmbeddingEngine>,
}

// ============================================================================
// TURSO IMPLEMENTATION (Async, Default)
// ============================================================================

#[cfg(feature = "turso")]
mod turso_impl {
    use super::*;
    use crate::db::TursoDatabase;

    impl SemanticSearch<TursoDatabase> {
        pub async fn new(db: TursoDatabase, embedder: Box<dyn EmbeddingEngine>) -> Self {
            Self { db, embedder }
        }

        pub async fn open_default() -> Result<Self> {
            let db = TursoDatabase::open(".patina/db/facts.db").await?;
            let embedder = crate::embeddings::create_embedder()?;
            Ok(Self::new(db, embedder).await)
        }

        pub async fn search_beliefs(
            &mut self,
            query: &str,
            top_k: usize
        ) -> Result<Vec<(i64, f32)>> {
            // Generate embedding (sync - CPU bound, fast)
            let embedding = self.embedder.embed(query)?;

            // Search vectors (async - I/O bound)
            let matches = self.db.vector_search(
                VectorTable::Beliefs,
                &embedding,
                None,
                top_k
            ).await?;

            Ok(matches.into_iter()
                .map(|m| (m.row_id, m.similarity))
                .collect())
        }

        /// Search multiple queries concurrently (scoped, no 'static issues)
        pub async fn search_many<'a>(
            &'a mut self,
            queries: &[&str],
            top_k: usize
        ) -> Result<Vec<Vec<(i64, f32)>>> {
            use futures::future::join_all;

            let searches = queries.iter()
                .map(|q| self.search_beliefs(q, top_k));

            let results = join_all(searches).await;
            results.into_iter().collect()
        }
    }
}

// ============================================================================
// SQLITE IMPLEMENTATION (Sync, Fallback)
// ============================================================================

#[cfg(feature = "sqlite")]
mod sqlite_impl {
    use super::*;
    use crate::db::SqliteDatabase;

    impl SemanticSearch<SqliteDatabase> {
        pub fn new(db: SqliteDatabase, embedder: Box<dyn EmbeddingEngine>) -> Self {
            Self { db, embedder }
        }

        pub fn open_default() -> Result<Self> {
            let db = SqliteDatabase::open(".patina/db/facts.db")?;
            let embedder = crate::embeddings::create_embedder()?;
            Ok(Self::new(db, embedder))
        }

        pub fn search_beliefs(
            &mut self,
            query: &str,
            top_k: usize
        ) -> Result<Vec<(i64, f32)>> {
            // Generate embedding
            let embedding = self.embedder.embed(query)?;

            // Search vectors (synchronous)
            let matches = self.db.vector_search(
                VectorTable::Beliefs,
                &embedding,
                None,
                top_k
            )?;

            Ok(matches.into_iter()
                .map(|m| (m.row_id, m.similarity))
                .collect())
        }
    }
}

// Re-export based on enabled features
#[cfg(feature = "turso")]
pub use turso_impl::*;

#[cfg(feature = "sqlite")]
pub use sqlite_impl::*;
```

---

## User Experience

### Default: Turso User (Async, Distributed)

```toml
# Cargo.toml
[dependencies]
patina = "0.1"  # Turso is default
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }
```

```rust
// main.rs
use patina::query::SemanticSearch;
use patina::db::TursoDatabase;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Open database (async)
    let db = TursoDatabase::open(".patina/db/facts.db").await?;

    // Create search engine
    let embedder = patina::embeddings::create_embedder()?;
    let mut search = SemanticSearch::new(db, embedder).await;

    // Search (async - can run concurrently!)
    let results = search.search_beliefs("rust cli tools", 10).await?;

    for (belief_id, similarity) in results {
        println!("Belief {}: {:.3}", belief_id, similarity);
    }

    Ok(())
}
```

**Benefits**:
- ✅ True async/await (concurrent queries possible)
- ✅ Distributed sync (embedded replica mode)
- ✅ Native vectors (cleaner API)
- ✅ Rust-native database

**Cost**: User must understand async/await basics

### Fallback: SQLite User (Sync, Simple)

```toml
# Cargo.toml
[dependencies]
patina = { version = "0.1", default-features = false, features = ["sqlite"] }
```

```rust
// main.rs
use patina::query::SemanticSearch;
use patina::db::SqliteDatabase;

fn main() -> anyhow::Result<()> {
    // Open database (sync)
    let db = SqliteDatabase::open(".patina/db/facts.db")?;

    // Create search engine
    let embedder = patina::embeddings::create_embedder()?;
    let mut search = SemanticSearch::new(db, embedder);

    // Search (sync)
    let results = search.search_beliefs("rust cli tools", 10)?;

    for (belief_id, similarity) in results {
        println!("Belief {}: {:.3}", belief_id, similarity);
    }

    Ok(())
}
```

**Benefits**:
- ✅ Zero async complexity
- ✅ No tokio dependency
- ✅ Smaller binary
- ✅ Pure sync simplicity

**Trade-offs**:
- ⚠️ Local-only (no distributed sync)
- ⚠️ Sequential operations (no concurrency)

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

## Question: Do We Need Both Features Enabled?

### Current Design
```toml
[features]
default = ["turso"]
turso = ["dep:turso", "dep:tokio"]
sqlite = ["dep:rusqlite", "dep:sqlite-vec"]
```

**Possible to enable both**:
```bash
cargo build --features sqlite,turso
```

### Use Cases for Both

**Potential use case**: Migration tool?
```rust
// Migrate from SQLite to Turso
async fn migrate() -> Result<()> {
    let sqlite = SqliteDatabase::open("old.db")?;
    let turso = TursoDatabase::open("new.db").await?;

    // Copy data...
}
```

**Counter-argument**: This is a one-time operation, doesn't justify permanent support.

### Recommendation: Mutually Exclusive (Initially)

**Proposed**:
```toml
[features]
default = ["turso"]
turso = ["dep:turso", "dep:tokio"]
sqlite = ["dep:rusqlite", "dep:sqlite-vec"]
```

**Enforce at compile time**:
```rust
// src/lib.rs
#[cfg(all(feature = "turso", feature = "sqlite"))]
compile_error!("Features 'turso' and 'sqlite' are mutually exclusive. Choose one.");

#[cfg(not(any(feature = "turso", feature = "sqlite")))]
compile_error!("Must enable either 'turso' or 'sqlite' feature.");
```

**Rationale**:
1. Simpler mental model (one backend at a time)
2. Smaller binaries (don't bundle both)
3. Clearer user intent (distributed vs local-only)
4. Can relax later if use cases emerge

**If we need migration**: Provide separate `patina-migrate` tool.

---

## Implementation Progress

### ✅ Completed - Foundation Work
- [x] DatabaseBackend enum with SQLite variant
- [x] All domain wrappers use enum (scrape, embeddings, semantic_search)
- [x] Config system for backend selection
- [x] Research into Turso API and vector support
- [x] Architectural decision: Dual API (no wrapper)
- [x] 95 tests passing (SQLite only currently)

### 🎯 Current Phase - Dual API Implementation

**Next Steps**:

1. **Create Feature Flags** ✅ (design complete)
   - Add `turso` feature to Cargo.toml
   - Make `sqlite` optional (was default)
   - Add compile-time checks for mutual exclusivity

2. **Implement Turso Backend**
   - Create `src/db/turso.rs` with async API
   - Native vector support (`vector_distance_cos`)
   - Embedded replica support (local + cloud sync)

3. **Refactor Domain Types to Generic**
   - `SemanticSearch<DB>` with conditional impls
   - `EmbeddingsDatabase<DB>` with conditional impls
   - Keep shared types in separate modules

4. **Update Commands**
   - Conditional compilation for sync/async variants
   - Keep CLI simple (users choose via features)

5. **Documentation**
   - Async best practices guide
   - Migration guide (SQLite → Turso)
   - Feature selection guide

6. **Testing Strategy**
   - Test both features in CI (separate jobs)
   - Integration tests for each backend
   - No cross-backend tests (mutually exclusive)

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

## Migration Timeline

### Phase 1: Feature Flags & Conditional Compilation (Week 1)
- [ ] Add turso dependencies (optional)
- [ ] Add feature flags to Cargo.toml
- [ ] Add compile-time checks (mutual exclusivity)
- [ ] Refactor existing code to use `#[cfg(feature = "sqlite")]`

### Phase 2: Turso Backend Implementation (Week 2)
- [ ] Implement `TursoDatabase` with async API
- [ ] Native vector search (`vector_distance_cos`)
- [ ] Embedded replica support
- [ ] Basic tests passing

### Phase 3: Domain Layer Refactor (Week 3)
- [ ] Make `SemanticSearch<DB>` generic
- [ ] Implement async variant for Turso
- [ ] Make `EmbeddingsDatabase<DB>` generic
- [ ] Update commands for conditional compilation

### Phase 4: Testing & Documentation (Week 4)
- [ ] CI matrix for both backends
- [ ] Integration tests for Turso
- [ ] Async best practices guide
- [ ] Migration documentation

---

## Success Criteria

### Turso Backend (Default)
- [x] Feature flag working
- [ ] Async API clean and idiomatic
- [ ] Vector search using native `vector_distance_cos()`
- [ ] Embedded replica mode functional
- [ ] Background sync working
- [ ] All tests passing

### SQLite Backend (Fallback)
- [x] Feature flag working
- [x] Sync API clean and simple
- [x] Vector search using sqlite-vec
- [x] No tokio dependency when enabled
- [x] All tests passing

### User Experience
- [ ] Clear documentation on choosing backend
- [ ] Async best practices documented
- [ ] Migration guide (SQLite → Turso)
- [ ] Example projects for both backends

---

## Open Questions

### 1. Should Both Features Be Allowed Together?

**Current stance**: No, mutually exclusive.

**Reasoning**:
- Simpler (one backend per build)
- Smaller binaries
- Clearer intent

**Revisit if**: Migration tool or hybrid use cases emerge.

### 2. How to Handle Vector Syntax Differences?

**sqlite-vec**:
```sql
WHERE embedding MATCH ?
```

**Turso**:
```sql
WHERE vector_distance_cos(embedding, vector32(?)) < threshold
```

**Solution**: Backend-specific SQL generation (already isolated in implementations).

### 3. Should Default Be Configurable?

**Current**: `default = ["turso"]` (hard-coded)

**Alternative**: Let users explicitly choose:
```toml
# User must choose
[dependencies]
patina = { version = "0.1", default-features = false, features = ["turso"] }
```

**Decision**: Keep `turso` as default for now (aligns with vision).

---

## References

- Turso crate: https://crates.io/crates/turso
- Our scraped Turso codebase: `layer/dust/repos/turso/`
- No Boilerplate async critique: Informs our scoped concurrency approach
- tokio runtime docs: Global runtime pattern for libraries

---

**Status**: Architecture finalized, ready for implementation
**Owner**: @nicabar
**Next Action**: Begin Phase 1 (Feature flags & conditional compilation)
