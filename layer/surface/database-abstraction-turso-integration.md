---
id: database-abstraction-turso-integration
version: 3
status: partial
created_date: 2025-11-01
updated_date: 2025-11-02
oxidizer: nicabar
tags: [turso, database, architecture, refactoring, sqlite, wrapper, technical-debt]
---

# Database Abstraction & Turso Integration

**Original Goal:** Make database backend a first-class configurable choice, with seamless support for SQLite (sqlite-vec) and Turso (libsql), designed for easy unwinding if needed.

**Actual Accomplishment:** Internal consistency refactor - consolidated three disparate database access patterns into a single wrapper pattern. **No multi-backend support implemented.**

**Status:** Phase 1-4 Complete (Consistency), Phase 5+ Not Started (Actual Abstraction)
**Current Reality:** All modules use `SqliteDatabase` wrapper (concrete type), still tightly coupled to SQLite
**Architecture Limitation:** Rejected traits means no runtime backend selection possible without significant rework

## Implementation Progress

### ✅ Completed - Internal Consistency Refactor
- [x] Phase 1: Created `SqliteDatabase` wrapper (concrete type, not abstraction)
- [x] Phase 2: Refactored semantic_search from free functions to `SemanticSearch` struct
- [x] Phase 3: Refactored embeddings from free functions to `EmbeddingsDatabase` struct
- [x] Phase 4: Refactored scrape/code to use `SqliteDatabase` instead of raw `Connection`
- [x] All modules now follow consistent struct-based pattern
- [x] Removed public API exposure of rusqlite (semantic_search)
- [x] Production validated: SDL (1,409 files, 80K items) and patina (102 files, 2.4K items)

### ❌ Not Completed - Actual Multi-Backend Support
- [ ] Turso backend implementation (0% progress)
- [ ] Config system for backend selection (doesn't exist)
- [ ] Factory pattern for runtime backend choice (impossible with current design)
- [ ] Trait or enum abstraction for polymorphism (explicitly rejected)
- [ ] Performance benchmarks of wrapper overhead (not measured)
- [ ] Test failures addressed (negative similarity bug ignored)

### 🚨 Architecture Concerns

**1. Not Actually an Abstraction**
- `SqliteDatabase` is a **concrete wrapper** around `rusqlite::Connection`, not an abstraction
- No polymorphism: Can't have different backend implementations
- No runtime selection: Can't choose backend based on config
- All modules are still tightly coupled to SQLite through the concrete type

**2. The Trait Rejection Problem**
- Rejected traits because "generic methods aren't dyn-safe"
- **Critical Issue:** This prevents actual multi-backend support
- Consequence: No `Box<dyn Database>`, no runtime backend choice
- Alternative approaches not explored: enum dispatch, separate dyn-safe operations, generic types

**3. Escape Hatch Technical Debt**
- Every wrapper provides `.connection()` → `&Connection` escape hatch
- **Problem:** Any code using this becomes SQLite-specific
- **Problem:** Creates two ways to do everything
- **Problem:** No plan for removal or migration path

**4. What Happens When Adding Turso?**
- Current architecture requires one of:
  1. Re-introduce traits (architecture was wrong)
  2. Enum dispatch: `enum Backend { Sqlite(SqliteDatabase), Turso(TursoDatabase) }` (requires refactor)
  3. Make `SqliteDatabase` wrap either rusqlite or libsql (naming confusion, hidden complexity)
- All options require significant rework of the "completed" modules

### 📝 Lessons Learned (Revised)

**What Worked:**
1. ✅ **Consistent pattern**: All modules follow same struct-based approach
2. ✅ **API cleanup**: Removed rusqlite exposure from public APIs
3. ✅ **Execution quality**: Professional commits, testing, documentation
4. ✅ **Production validated**: Works with real codebases

**What Didn't Work:**
1. ❌ **"Concrete types win"**: Post-hoc rationalization of a limitation, not a principle
2. ❌ **Trait rejection**: Premature decision that blocks stated goal
3. ❌ **Escape hatches**: Technical debt that undermines abstraction
4. ❌ **Goal mismatch**: Delivered internal refactor, claimed multi-backend support

**Critical Insight:**
> We optimized for implementation simplicity over architectural requirements. The result is cleaner, more consistent code, but we're not actually closer to Turso integration. We've rearranged the furniture but not opened the door.

**Grade: B-**
- Good refactoring work
- Questionable architecture for stated goals
- Incomplete delivery

### 🔄 Recommended Path Forward

**Commit to Goal: Implement Actual Multi-Backend Support**

The work done so far (consistency refactor) is valuable but incomplete. To deliver on the stated "Turso Integration" goal, we must:

1. Choose abstraction approach (enum dispatch or traits)
2. Implement the abstraction layer
3. Implement Turso backend
4. Add config system and factory pattern
5. Validate with both backends

**Recommended Approach: Enum Dispatch Pattern**
```rust
pub enum DatabaseBackend {
    Sqlite(SqliteDatabase),
    Turso(TursoDatabase),  // Future
}

impl DatabaseBackend {
    pub fn open_from_config(config: &Config) -> Result<Self> {
        match config.backend {
            BackendType::Sqlite => Ok(Self::Sqlite(SqliteDatabase::open(path)?)),
            BackendType::Turso => Ok(Self::Turso(TursoDatabase::open(path)?)),
        }
    }

    // Forward methods to underlying impl
    pub fn execute(&self, sql: &str, params: &[&dyn ToSql]) -> Result<usize> {
        match self {
            Self::Sqlite(db) => db.execute(sql, params),
            Self::Turso(db) => db.execute(sql, params),
        }
    }
}
```

**Advantages:**
- No trait object overhead
- Compile-time dispatch
- Can still have backend-specific methods
- Keeps concrete types but adds flexibility

**Required Changes:**
- Domain wrappers own `DatabaseBackend` instead of `SqliteDatabase`
- Add config system
- Implement `TursoDatabase` with same API as `SqliteDatabase`
- Update 7 files (doable)

**Recommendation:** Choose Option 3 (Enum Dispatch) if Turso support is actually needed, Option 1 (Accept Reality) if not.

### 🐛 Technical Debt to Address

1. **Test Failures**
   - Fix negative similarity bug in semantic_search tests
   - Add tests for wrapper types
   - Verify abstraction actually works

2. **Escape Hatches**
   - Audit all `.connection()` usage
   - Either wrap operations properly or accept SQLite-only
   - Document which operations can't be abstracted

3. **Documentation**
   - Update all "ready for backend swapping" claims
   - Be honest about what was accomplished
   - Document actual vs. claimed goals

4. **Performance**
   - Benchmark wrapper overhead
   - Justify indirection layer cost
   - Profile real-world usage

---

## Why This Matters (Original Context)

### Current Reality
- **Embeddings + Semantic Search** just migrated from sqlite-vss → sqlite-vec (working great!)
- **Turso** offers native vectors, Rust rewrite of SQLite, optional distributed sync
- **But**: Turso is beta, APIs are in flux, we need easy escape hatch

### Design Goal
> "Design for Turso focus but easy unwinding" - use Turso as primary DB since Patina is pre-alpha, but architect so switching back to SQLite is trivial (one config change).

---

## Current State Analysis (Post-Refactor)

### Module Assessment - AFTER Phase 1-4 ✅

| Module | Abstraction | Backend Coupling | Swap Difficulty | Grade |
|--------|-------------|------------------|-----------------|-------|
| **scrape/code** | ✅ Has wrapper (`Database` → `SqliteDatabase`) | Low (contained) | 🟢 Easy | **A** |
| **embeddings** | ✅ Has wrapper (`EmbeddingsDatabase` → `SqliteDatabase`) | Low (contained) | 🟢 Easy | **A** ⬆️ |
| **semantic_search** | ✅ Has wrapper (`SemanticSearch` → `SqliteDatabase`) | Low (contained) | 🟢 Easy | **A** ⬆️ |
| **beliefs** | ❌ N/A | N/A (not implemented) | 🟢 Easy | N/A |

**Result:** All implemented modules are Grade A! Ready for backend swapping.

### Code Usage Patterns - Current Implementation

#### 🏆 scrape/code (Now Uses Abstraction)
```rust
// src/commands/scrape/code/database.rs
pub struct Database {
    db: SqliteDatabase,  // ← Uses wrapper!
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = SqliteDatabase::open(path)?;
        Ok(Self { db })
    }

    pub fn insert_functions(&self, functions: &[FunctionFact]) -> Result<usize> {
        let conn = self.db.connection();
        // ... uses wrapper API
    }
}

// Usage (unchanged)
let db = Database::open(db_path)?;
db.insert_functions(&functions)?;
```

**Why it's excellent:**
- Owns `SqliteDatabase` wrapper (not raw `Connection`)
- Domain-specific methods unchanged
- Backend swap = just change what `SqliteDatabase` wraps
- Public API stable

#### ✅ embeddings (Refactored in Phase 3)
```rust
// src/embeddings/database.rs
pub struct EmbeddingsDatabase {
    db: SqliteDatabase,  // ← Owns wrapper
}

impl EmbeddingsDatabase {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = SqliteDatabase::open(path)?;
        Ok(Self { db })
    }

    pub fn generate_belief_embeddings(
        &self,
        embedder: &mut dyn EmbeddingEngine,
    ) -> Result<usize> {
        // Clean domain-specific API
    }
}

// Usage
let db = EmbeddingsDatabase::open(db_path)?;
db.generate_belief_embeddings(&mut *embedder)?;
```

**Improvements from original:**
- No `&mut Connection` parameters
- Clean encapsulation
- Easy to swap backend

#### ✅ semantic_search (Refactored in Phase 2)
```rust
// src/query/semantic_search.rs
pub struct SemanticSearch {
    db: SqliteDatabase,  // ← Owns wrapper
    embedder: Box<dyn EmbeddingEngine>,
}

impl SemanticSearch {
    pub fn new(db: SqliteDatabase, embedder: Box<dyn EmbeddingEngine>) -> Self {
        Self { db, embedder }
    }

    pub fn search_beliefs(
        &mut self,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<BeliefSearchResult>> {
        // No rusqlite in public API!
    }
}

// Usage
let mut search = SemanticSearch::open_default()?;
let results = search.search_beliefs("query", 10)?;
```

**Improvements from original:**
- Public API has ZERO rusqlite exposure
- Struct-based instead of free functions
- Backend completely abstracted

---

## Target Architecture

### Core Abstraction Layer

```rust
// src/db/mod.rs

/// Generic database operations for Patina
///
/// Design principles:
/// - Sync API (no async leakage)
/// - Covers relational + vector operations
/// - Backend-agnostic
pub trait Database: Send + Sync {
    // --- Basic SQL Operations ---
    fn execute(&self, sql: &str, params: &[&dyn ToSql]) -> Result<usize>;

    fn query_row<T, F>(&self, sql: &str, params: &[&dyn ToSql], f: F) -> Result<T>
        where F: FnOnce(&Row) -> Result<T>;

    fn query_map<T, F>(&self, sql: &str, params: &[&dyn ToSql], f: F) -> Result<Vec<T>>
        where F: FnMut(&Row) -> Result<T>;

    fn prepare<'a>(&'a self, sql: &str) -> Result<Box<dyn Statement + 'a>>;

    // --- Transactions ---
    fn transaction<F, T>(&self, f: F) -> Result<T>
        where F: FnOnce(&dyn Database) -> Result<T>;

    // --- Vector Operations (Abstracted) ---
    fn vector_search(
        &self,
        table: VectorTable,
        query_vector: &[f32],
        filter: Option<VectorFilter>,
        limit: usize,
    ) -> Result<Vec<VectorMatch>>;

    fn vector_insert(
        &self,
        table: VectorTable,
        id: i64,
        vector: &[f32],
        metadata: Option<HashMap<String, SqlValue>>,
    ) -> Result<()>;

    // --- Schema Management ---
    fn initialize_schema(&self, schema_sql: &str) -> Result<()>;

    // --- Backend Info ---
    fn backend_name(&self) -> &'static str;
    fn supports_native_vectors(&self) -> bool;
}

/// Prepared statement abstraction
pub trait Statement {
    fn execute(&mut self, params: &[&dyn ToSql]) -> Result<usize>;
    fn query_map<T, F>(&mut self, params: &[&dyn ToSql], f: F) -> Result<Vec<T>>
        where F: FnMut(&Row) -> Result<T>;
}

/// Row abstraction (for query results)
pub trait Row {
    fn get<T: FromSql>(&self, idx: usize) -> Result<T>;
}
```

### Vector-Specific Types

```rust
// src/db/vectors.rs

pub enum VectorTable {
    Beliefs,
    Observations,
}

pub struct VectorFilter {
    pub field: String,
    pub operator: FilterOp,
    pub value: SqlValue,
}

pub enum FilterOp {
    Equals,
    NotEquals,
    In,
    Like,
}

pub struct VectorMatch {
    pub row_id: i64,
    pub distance: f32,
    pub similarity: f32,  // Computed: 1.0 - distance
    pub metadata: HashMap<String, SqlValue>,
}

pub enum SqlValue {
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
    Null,
}
```

---

## Backend Implementations

### SQLite Backend (sqlite-vec)

```rust
// src/db/sqlite.rs

use rusqlite::{Connection, params};
use sqlite_vec;

pub struct SqliteBackend {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteBackend {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Load sqlite-vec extension
        unsafe {
            sqlite_vec::sqlite3_auto_extension(Some(
                std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ())
            ));
        }
        conn.load_extension_enable()?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;

        unsafe {
            sqlite_vec::sqlite3_auto_extension(Some(
                std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ())
            ));
        }
        conn.load_extension_enable()?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

impl Database for SqliteBackend {
    fn execute(&self, sql: &str, params: &[&dyn ToSql]) -> Result<usize> {
        let conn = self.conn.lock();
        let count = conn.execute(sql, params)?;
        Ok(count)
    }

    fn vector_search(
        &self,
        table: VectorTable,
        query_vector: &[f32],
        filter: Option<VectorFilter>,
        limit: usize,
    ) -> Result<Vec<VectorMatch>> {
        let conn = self.conn.lock();

        // sqlite-vec syntax: "WHERE embedding MATCH ?"
        let table_name = match table {
            VectorTable::Beliefs => "belief_vectors",
            VectorTable::Observations => "observation_vectors",
        };

        let sql = if let Some(f) = filter {
            format!(
                "SELECT rowid, distance FROM {}
                 WHERE embedding MATCH ? AND {} {} ?
                 ORDER BY distance LIMIT ?",
                table_name, f.field, f.operator.to_sql()
            )
        } else {
            format!(
                "SELECT rowid, distance FROM {}
                 WHERE embedding MATCH ?
                 ORDER BY distance LIMIT ?",
                table_name
            )
        };

        // Convert vector to bytes (sqlite-vec format)
        use zerocopy::AsBytes;
        let vector_bytes = query_vector.as_bytes();

        let mut stmt = conn.prepare(&sql)?;
        let results = if let Some(f) = filter {
            stmt.query_map(params![vector_bytes, f.value.to_sql(), limit], |row| {
                Ok(VectorMatch {
                    row_id: row.get(0)?,
                    distance: row.get(1)?,
                    similarity: 1.0 - row.get::<_, f32>(1)?,  // Cosine distance → similarity
                    metadata: HashMap::new(),
                })
            })?
        } else {
            stmt.query_map(params![vector_bytes, limit], |row| {
                Ok(VectorMatch {
                    row_id: row.get(0)?,
                    distance: row.get(1)?,
                    similarity: 1.0 - row.get::<_, f32>(1)?,
                    metadata: HashMap::new(),
                })
            })?
        };

        results.collect::<Result<Vec<_>, _>>()
    }

    fn backend_name(&self) -> &'static str {
        "sqlite-vec"
    }

    fn supports_native_vectors(&self) -> bool {
        true
    }
}
```

### Turso Backend (libsql)

```rust
// src/db/turso.rs

use libsql::{Builder, Connection as LibsqlConnection, Database as LibsqlDatabase};
use futures::executor::block_on;

pub struct TursoBackend {
    client: Arc<TursoClient>,
}

enum TursoClient {
    /// Local-only (no cloud sync, just libsql)
    Local {
        db: LibsqlDatabase,
        conn: Mutex<LibsqlConnection>,
    },

    /// Embedded replica (local file + cloud sync)
    Embedded {
        db: LibsqlDatabase,
        conn: Mutex<LibsqlConnection>,
    },

    /// Remote-only (HTTP to Turso cloud)
    Remote {
        db: LibsqlDatabase,
        conn: Mutex<LibsqlConnection>,
    },
}

impl TursoBackend {
    /// Open local-only database (no cloud, just libsql)
    pub fn open_local(path: impl AsRef<Path>) -> Result<Self> {
        let db = block_on(Builder::new_local(path).build())?;
        let conn = block_on(db.connect())?;

        Ok(Self {
            client: Arc::new(TursoClient::Local {
                db,
                conn: Mutex::new(conn),
            }),
        })
    }

    /// Open embedded replica (local file that syncs to cloud)
    pub fn open_embedded(
        local_path: impl AsRef<Path>,
        url: &str,
        auth_token: &str,
    ) -> Result<Self> {
        let db = block_on(
            Builder::new_local_replica(local_path)
                .url(url)
                .auth_token(auth_token)
                .build()
        )?;

        let conn = block_on(db.connect())?;

        // Initial sync
        block_on(conn.sync())?;

        Ok(Self {
            client: Arc::new(TursoClient::Embedded {
                db,
                conn: Mutex::new(conn),
            }),
        })
    }

    /// Trigger manual sync (for embedded mode)
    pub fn sync(&self) -> Result<()> {
        match &*self.client {
            TursoClient::Embedded { conn, .. } => {
                let conn = conn.lock();
                block_on(conn.sync())?;
                Ok(())
            }
            _ => Ok(()), // No-op for local/remote
        }
    }
}

impl Database for TursoBackend {
    fn execute(&self, sql: &str, params: &[&dyn ToSql]) -> Result<usize> {
        let client = &self.client;
        match &**client {
            TursoClient::Local { conn, .. }
            | TursoClient::Embedded { conn, .. }
            | TursoClient::Remote { conn, .. } => {
                let conn = conn.lock();

                // Convert params to libsql::Value
                let libsql_params = params.iter()
                    .map(|p| convert_to_libsql_value(p))
                    .collect::<Result<Vec<_>>>()?;

                let result = block_on(conn.execute(sql, libsql_params))?;
                Ok(result as usize)
            }
        }
    }

    fn vector_search(
        &self,
        table: VectorTable,
        query_vector: &[f32],
        filter: Option<VectorFilter>,
        limit: usize,
    ) -> Result<Vec<VectorMatch>> {
        // Turso syntax: vector_distance_cos(embedding, vector32(?))
        let table_name = match table {
            VectorTable::Beliefs => "belief_vectors",
            VectorTable::Observations => "observation_vectors",
        };

        let sql = if let Some(f) = filter {
            format!(
                "SELECT rowid, vector_distance_cos(embedding, vector32(?)) as distance
                 FROM {}
                 WHERE {} {} ?
                 ORDER BY distance LIMIT ?",
                table_name, f.field, f.operator.to_sql()
            )
        } else {
            format!(
                "SELECT rowid, vector_distance_cos(embedding, vector32(?)) as distance
                 FROM {}
                 ORDER BY distance LIMIT ?",
                table_name
            )
        };

        // Turso expects vector as JSON array or binary
        let vector_blob = serialize_vector_for_turso(query_vector);

        // Execute query (same pattern as execute())
        let results = self.query_map(&sql, &[&vector_blob, &limit], |row| {
            Ok(VectorMatch {
                row_id: row.get(0)?,
                distance: row.get(1)?,
                similarity: 1.0 - row.get::<_, f32>(1)?,
                metadata: HashMap::new(),
            })
        })?;

        Ok(results)
    }

    fn backend_name(&self) -> &'static str {
        "turso"
    }

    fn supports_native_vectors(&self) -> bool {
        true
    }
}

// Helper to convert params
fn convert_to_libsql_value(value: &dyn ToSql) -> Result<libsql::Value> {
    // Implement conversion logic
    // This is tricky but doable with type reflection
    todo!("Implement param conversion")
}

fn serialize_vector_for_turso(vector: &[f32]) -> Vec<u8> {
    // Turso's vector32() function format
    vector.iter().flat_map(|f| f.to_le_bytes()).collect()
}
```

---

## Configuration System

```toml
# .patina/config.toml

[database]
backend = "turso"  # or "sqlite"

# SQLite configuration
[database.sqlite]
path = ".patina/db/facts.db"

# Turso configuration
[database.turso]
mode = "local"  # or "embedded" or "remote"
path = ".patina/db/facts.db"

# Optional: for embedded/remote modes
# url = "libsql://patina-oxidizer.turso.io"
# auth_token = "..."
# sync_interval_seconds = 300  # Auto-sync every 5 minutes
```

```rust
// src/db/config.rs

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub backend: BackendType,
    pub sqlite: Option<SqliteConfig>,
    pub turso: Option<TursoConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    Sqlite,
    Turso,
}

#[derive(Debug, Deserialize)]
pub struct SqliteConfig {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct TursoConfig {
    pub mode: TursoMode,
    pub path: String,
    pub url: Option<String>,
    pub auth_token: Option<String>,
    pub sync_interval_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TursoMode {
    Local,      // No cloud sync
    Embedded,   // Local file + cloud sync
    Remote,     // Cloud-only
}

/// Load database configuration
pub fn load_config() -> Result<DatabaseConfig> {
    let config_path = ".patina/config.toml";
    let config_str = std::fs::read_to_string(config_path)
        .context("Failed to read config file")?;
    let config: DatabaseConfig = toml::from_str(&config_str)
        .context("Failed to parse config")?;
    Ok(config)
}

/// Factory: Create appropriate backend from config
pub fn open_database(config: &DatabaseConfig) -> Result<Box<dyn Database>> {
    match config.backend {
        BackendType::Sqlite => {
            let cfg = config.sqlite.as_ref()
                .context("Missing sqlite config")?;
            Ok(Box::new(SqliteBackend::open(&cfg.path)?))
        }
        BackendType::Turso => {
            let cfg = config.turso.as_ref()
                .context("Missing turso config")?;
            match cfg.mode {
                TursoMode::Local => {
                    Ok(Box::new(TursoBackend::open_local(&cfg.path)?))
                }
                TursoMode::Embedded => {
                    let url = cfg.url.as_ref()
                        .context("Embedded mode requires url")?;
                    let token = cfg.auth_token.as_ref()
                        .context("Embedded mode requires auth_token")?;
                    Ok(Box::new(TursoBackend::open_embedded(
                        &cfg.path, url, token
                    )?))
                }
                TursoMode::Remote => {
                    // Implement remote-only connection
                    todo!("Remote mode not yet implemented")
                }
            }
        }
    }
}
```

---

## Module Refactoring - Completed Phases

### Phase 1: Create Abstraction Layer ✅ COMPLETE
**Goal:** Add `src/db/` module without breaking existing code

**Implementation:**
```
src/db/
├── mod.rs           # Public exports: SqliteDatabase, vector types
├── sqlite.rs        # SqliteDatabase (concrete wrapper, no traits)
└── vectors.rs       # VectorTable, VectorMatch, VectorFilter types
```

**Key Decisions:**
- No trait abstraction (rejected for dyn-safety issues)
- Concrete `SqliteDatabase` struct following scrape/code pattern
- Simple API: `open()`, `execute()`, `vector_search()`, `connection()`
- Escape hatch: `.connection()` for gradual migration

**Result:** Foundation ready, all existing code unchanged

### Phase 2: Refactor semantic_search ✅ COMPLETE
**Goal:** Fix worst offender - public API exposing rusqlite::Connection

**Before:**
- Free functions with `conn: &Connection` parameter in public API
- Backend leakage breaking encapsulation
- Grade D (very hard to swap)

**After:**
```rust
// src/query/semantic_search.rs
pub struct SemanticSearch {
    db: SqliteDatabase,
    embedder: Box<dyn EmbeddingEngine>,
}

impl SemanticSearch {
    pub fn search_beliefs(&mut self, query: &str, top_k: usize) -> Result<Vec<...>> {
        // Clean API - no rusqlite exposure!
    }
}
```

**Achievements:**
- Public API completely clean (no rusqlite)
- Struct-based design (owns database + embedder)
- Grade A (easy to swap backend)
- All 5 integration tests passing

### Phase 3: Refactor Embeddings ✅ COMPLETE
**Goal:** Clean up free functions taking `&mut Connection`

**Before:**
- Commands called free functions with `&mut Connection` parameter
- Backend exposed in internal signatures
- Grade C (medium swap difficulty)

**After:**
```rust
// src/embeddings/database.rs (NEW)
pub struct EmbeddingsDatabase {
    db: SqliteDatabase,
}

impl EmbeddingsDatabase {
    pub fn generate_belief_embeddings(&self, embedder: &mut dyn EmbeddingEngine) -> Result<usize>
    pub fn generate_observation_embeddings(&self, embedder: &mut dyn EmbeddingEngine) -> Result<usize>
}

// Commands updated
let db = EmbeddingsDatabase::open(db_path)?;
db.generate_belief_embeddings(&mut *embedder)?;
```

**Achievements:**
- Clean domain-specific wrapper
- No `&mut Connection` parameters
- Grade A (easy to swap backend)
- Commands use simple, clean API

### Phase 4: Refactor scrape/code ✅ COMPLETE
**Goal:** Apply abstraction back to the north star pattern

**Before:**
- `Database` struct owned raw `rusqlite::Connection`
- Already had good pattern (Grade A), but not using shared abstraction
- Inconsistent with other modules

**After:**
```rust
// src/commands/scrape/code/database.rs
pub struct Database {
    db: SqliteDatabase,  // Now uses shared wrapper!
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = SqliteDatabase::open(path)?;
        Ok(Self { db })
    }

    pub fn insert_functions(&self, functions: &[FunctionFact]) -> Result<usize> {
        let conn = self.db.connection();
        // ... uses wrapper API
    }
}
```

**Achievements:**
- Full circle: north star now uses the abstraction it inspired
- All 3 modules consistent (scrape, embeddings, semantic_search)
- Grade A maintained, but now with shared infrastructure
- Added `connection_mut()` to SqliteDatabase for transaction support

**Result:** All implemented modules use SqliteDatabase wrapper. Ready for backend swapping!

---

## Schema Translation

Different backends have different vector syntax. We need to translate:

### SQLite (sqlite-vec)
```sql
CREATE VIRTUAL TABLE belief_vectors USING vec0(
    embedding float[384]
);

SELECT rowid, distance
FROM belief_vectors
WHERE embedding MATCH ?
ORDER BY distance LIMIT ?;
```

### Turso (libsql native)
```sql
CREATE TABLE belief_vectors (
    rowid INTEGER PRIMARY KEY,
    embedding BLOB  -- vector32 format
);

SELECT rowid, vector_distance_cos(embedding, vector32(?)) as distance
FROM belief_vectors
ORDER BY distance LIMIT ?;
```

### Translation Layer
```rust
// src/db/schema.rs

pub trait SchemaTranslator {
    fn translate_vector_table(&self, def: &VectorTableDef) -> String;
    fn translate_vector_search(&self, query: &VectorSearchQuery) -> String;
}

pub struct SqliteTranslator;
impl SchemaTranslator for SqliteTranslator {
    fn translate_vector_table(&self, def: &VectorTableDef) -> String {
        format!(
            "CREATE VIRTUAL TABLE {} USING vec0(embedding float[{}])",
            def.name, def.dimensions
        )
    }
}

pub struct TursoTranslator;
impl SchemaTranslator for TursoTranslator {
    fn translate_vector_table(&self, def: &VectorTableDef) -> String {
        format!(
            "CREATE TABLE {} (rowid INTEGER PRIMARY KEY, embedding BLOB)",
            def.name
        )
    }
}
```

---

## Testing Strategy

### Unit Tests (per backend)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_backend() {
        let db = SqliteBackend::open_in_memory().unwrap();
        test_database_operations(&db);
    }

    #[test]
    fn test_turso_backend() {
        let db = TursoBackend::open_in_memory().unwrap();
        test_database_operations(&db);  // SAME TEST!
    }

    // Shared test function
    fn test_database_operations(db: &dyn Database) {
        db.execute("CREATE TABLE test (id INTEGER, name TEXT)", &[]).unwrap();
        db.execute("INSERT INTO test VALUES (?, ?)", &[&1, &"test"]).unwrap();
        let result: String = db.query_row("SELECT name FROM test WHERE id = ?", &[&1], |row| {
            row.get(0)
        }).unwrap();
        assert_eq!(result, "test");
    }
}
```

### Integration Tests
```rust
// tests/database_backends.rs

#[test]
fn test_scrape_with_sqlite() {
    set_test_config("sqlite");
    let result = run_scrape_test();
    assert!(result.is_ok());
}

#[test]
fn test_scrape_with_turso() {
    set_test_config("turso");
    let result = run_scrape_test();
    assert!(result.is_ok());
}

fn run_scrape_test() -> Result<()> {
    // Same test, different backend based on config
    let db = CodeDatabase::open_with_config()?;
    db.insert_functions(&test_functions)?;
    Ok(())
}
```

---

## Migration Timeline (Revised)

### ✅ Phase 1 (Complete): Consistency Refactor
**Actual Time:** ~14 hours across 6 commits
- ✅ Created `src/db/` module with `SqliteDatabase` wrapper
- ✅ Refactored all modules to use consistent pattern
- ✅ Removed public API exposure of rusqlite
- ✅ Production validated
- **Milestone Achieved:** Consistent code structure across modules

### ⚠️ Phase 2 (Incomplete): Actual Abstraction
**Status:** Blocked by architecture decision
**Options:**
1. Accept SQLite-only, remove abstraction claims
2. Implement enum dispatch pattern (recommended if Turso needed)
3. Re-introduce traits with careful API design

**If pursuing Option 2 (Enum Dispatch):**
- [ ] Day 1: Design `DatabaseBackend` enum and API
- [ ] Day 2: Implement enum with `SqliteDatabase` variant
- [ ] Day 3: Add config system and factory
- [ ] Day 4-5: Refactor domain wrappers to use enum
- **Milestone:** Actual abstraction in place, still SQLite-only

### 🔮 Phase 3 (Not Started): Turso Integration
**Dependencies:** Phase 2 must be complete with actual abstraction
- [ ] Day 1-2: Research libsql API and vector support
- [ ] Day 3-4: Implement `TursoDatabase` with same API as `SqliteDatabase`
- [ ] Day 5: Add Turso variant to `DatabaseBackend` enum
- [ ] Day 6-7: Test with both backends, fix incompatibilities
- **Milestone:** Both backends working

### 🔮 Phase 4 (Not Started): Production Ready
**Dependencies:** Phase 3 complete with both backends working
- [ ] Day 1: Performance benchmarks (SQLite vs Turso)
- [ ] Day 2: Fix test failures (negative similarity bug)
- [ ] Day 3: Remove or document escape hatches
- [ ] Day 4: Add embedded/remote Turso modes
- [ ] Day 5: Documentation and examples
- **Milestone:** Production-ready multi-backend support

**Total Estimated Time (if pursuing Turso):** 3-4 weeks additional work
**Total Actual Progress on Turso Goal:** ~20% (consistency refactor only)

---

## Unwinding Strategy

### Current State
The consistency refactor provides a foundation for multi-backend support, but the work is incomplete. Rolling back would lose valuable API cleanup and consistency improvements.

### Future State (If Turso Added)
**If enum dispatch is implemented:**

**Option 1: Config Switch (Instant)**
```bash
# Switch back to SQLite
echo 'backend = "sqlite"' > .patina/config.toml
patina scrape code  # Works immediately!
```
**Effort:** 0 code changes, 1 config change

**Option 2: Remove Turso Backend (Clean)**
```bash
# Remove Turso backend
rm src/db/turso.rs

# Update DatabaseBackend enum to remove Turso variant
```
**Effort:** Delete 1 file, update enum, 2-3 hours

**Option 3: Keep Both (Recommended)**
- Having multiple backends is good for resilience
- Users can choose what works best
- No need to remove working code

**Key Point:** Easy unwinding requires actual abstraction (enum or traits). Current concrete type approach makes unwinding harder.

---

## Success Criteria (Revised)

### ✅ Achieved: Consistency Refactor
- [x] `src/db/` module created with `SqliteDatabase` wrapper
- [x] scrape/code uses wrapper
- [x] embeddings uses wrapper via `EmbeddingsDatabase`
- [x] semantic_search uses wrapper via `SemanticSearch`
- [x] Public API cleanup (removed rusqlite exposure)
- [x] Production validated with real codebases
- [x] All modules follow consistent struct-based pattern

### ❌ Not Achieved: Multi-Backend Support
- [ ] Database trait/enum defined (rejected traits, no enum yet)
- [ ] Configuration system (doesn't exist)
- [ ] Factory pattern for backend selection (impossible with current design)
- [ ] Turso backend implementation (not started)
- [ ] Backend switching working (not possible)
- [ ] Tests with multiple backends (only SQLite)

### 🎯 Revised Goals (If Pursuing Turso)

**Phase 2: Actual Abstraction**
- [ ] Choose abstraction approach (enum dispatch recommended)
- [ ] Implement `DatabaseBackend` enum or trait system
- [ ] Add config system for backend selection
- [ ] Refactor domain wrappers to use abstraction
- [ ] All tests pass with abstracted SQLite backend

**Phase 3: Turso Integration**
- [ ] Research libsql API and vector support
- [ ] Implement `TursoDatabase` matching SQLite API
- [ ] Add Turso variant to backend abstraction
- [ ] Config switch works seamlessly
- [ ] Tests pass with both SQLite and Turso backends

**Phase 4: Production Ready**
- [ ] Fix ignored test failures (negative similarity bug)
- [ ] Performance benchmarks (SQLite vs Turso overhead)
- [ ] Remove or document escape hatches
- [ ] Embedded/remote Turso modes
- [ ] Documentation and migration guide

---

## Open Questions (Updated)

### Architecture Decisions (Critical)

1. **What problem does Turso solve?**
   - Current state: SQLite with sqlite-vec works well for local-only use
   - Turso benefits: Distributed sync, cloud hosting, remote access, replication
   - Question: Do we need these features? When?
   - **Decision needed:** Clarify use cases requiring Turso before implementing

2. **If yes, which abstraction approach?**
   - **Option A:** Enum dispatch (`enum DatabaseBackend { Sqlite, Turso }`)
     - Pros: Type-safe, compile-time dispatch, keeps concrete types
     - Cons: Requires refactoring all domain wrappers
   - **Option B:** Traits with careful API design
     - Pros: True polymorphism, extensible
     - Cons: Complexity with generic methods, dyn-safety issues
   - **Decision needed:** Choose before implementing Turso

3. **What about the escape hatches?**
   - `.connection()` is used throughout codebase
   - This makes code SQLite-specific
   - Options:
     1. Wrap all operations (time-consuming)
     2. Accept SQLite-only for these operations
     3. Document which operations can't be abstracted
   - **Decision needed:** Audit usage, create migration plan

### Technical Questions

4. **Performance cost of indirection?**
   - Added wrapper layer: `Domain → SqliteDatabase → Connection`
   - Question: What's the overhead for 80K+ item operations?
   - **Action:** Benchmark before and after refactor

5. **Test failures we ignored?**
   - Negative similarity bug in semantic_search (concerning)
   - 3 ONNX tests failing (unrelated but should fix)
   - **Action:** Investigate and fix before claiming production-ready

### Turso-Specific (If Pursuing)

6. **Turso API compatibility?**
   - Does libsql API match rusqlite closely enough?
   - What operations need backend-specific code?
   - **Action:** Prototype TursoDatabase to discover incompatibilities

7. **Vector support in Turso?**
   - Turso has native vector support
   - Is it compatible with sqlite-vec API?
   - **Action:** Research before committing to Turso

8. **Sync strategy for embedded mode?**
   - Auto-sync on every write? Periodic? Manual?
   - Performance implications?
   - **Decision:** Start manual, add auto later if needed

### Process Questions

9. **Was this the right problem to solve?**
   - Spent ~14 hours on internal consistency
   - Could have spent that time on actual features
   - **Reflection:** Did we optimize the wrong thing?

10. **What's the completion timeline?**
    - Current state: 20% progress on Turso integration (consistency only)
    - Remaining work: 3-4 weeks to implement actual multi-backend support
    - Includes: enum dispatch, Turso backend, config system, testing
    - **Decision needed:** Commit resources to complete the stated goal

---

## References

- [Turso GitHub](https://github.com/tursodatabase/libsql)
- [sqlite-vec docs](https://github.com/asg017/sqlite-vec)
- `layer/sessions/20251031-131418.md` - Turso exploration findings
- `layer/surface/embeddings-integration-roadmap.md` - Phase 2 context
- `src/commands/scrape/code/database.rs` - Reference design

---

**Status:** Design Complete, Ready for Implementation
**Owner:** @nicabar
**Next Step:** Review with team, then implement Phase 1 (abstraction layer)
