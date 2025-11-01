---
id: database-abstraction-turso-integration
version: 2
status: active
created_date: 2025-11-01
updated_date: 2025-11-01
oxidizer: nicabar
tags: [turso, database, architecture, refactoring, sqlite, abstraction]
---

# Database Abstraction & Turso Integration

**Goal:** Make database backend a first-class configurable choice, with seamless support for SQLite (sqlite-vec) and Turso (libsql), designed for easy unwinding if needed.

**Status:** Phase 1, 2 & 3 Complete (abstraction + semantic_search + embeddings refactored)
**Target:** Clean abstraction that works across all modules (scrape, embeddings, semantic search, beliefs)
**Philosophy:** Turso-first for pre-alpha, but SQLite fallback is one config change away

## Implementation Progress

### ✅ Completed
- [x] Design document (this file)
- [x] Phase 1: `src/db` module with SqliteDatabase (scrape/code pattern)
- [x] Phase 2: semantic_search refactored to use abstraction
- [x] Phase 3: embeddings commands refactored to use abstraction
- [x] All tests passing (39 lib tests)

### 🚧 In Progress
- [ ] Turso backend implementation
- [ ] Config-based factory function

### 📝 Lessons Learned
1. **Rejected trait-based approach**: Generic methods (`query_row<T, F>`) aren't dyn-safe
2. **Concrete types win**: Following scrape/code pattern (no traits) is simpler and cleaner
3. **Escape hatches work**: `.connection()` method allows gradual migration

---

## Why This Matters

### Current Reality
- **Embeddings + Semantic Search** just migrated from sqlite-vss → sqlite-vec (working great!)
- **Turso** offers native vectors, Rust rewrite of SQLite, optional distributed sync
- **But**: Turso is beta, APIs are in flux, we need easy escape hatch

### Design Goal
> "Design for Turso focus but easy unwinding" - use Turso as primary DB since Patina is pre-alpha, but architect so switching back to SQLite is trivial (one config change).

---

## Current State Analysis

### Module Assessment

| Module | Abstraction | Backend Coupling | Swap Difficulty | Grade |
|--------|-------------|------------------|-----------------|-------|
| **scrape/code** | ✅ Has wrapper | Low (contained) | 🟢 Easy | A |
| **embeddings** | ❌ None | High (exposed) | 🟡 Medium | C |
| **semantic_search** | ❌ None | Very High (public API) | 🔴 Hard | D |
| **beliefs** | ❌ N/A | N/A (not implemented) | 🟢 Easy | N/A |

### Code Usage Patterns

#### 🏆 scrape/code (Reference Design)
```rust
// src/commands/scrape/code/database.rs
pub struct Database {
    conn: Connection,  // ← Encapsulated
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self>;
    pub fn insert_functions(&self, functions: &[FunctionFact]) -> Result<usize>;
    pub fn insert_types(&self, types: &[TypeFact]) -> Result<usize>;
}

// Usage
let db = Database::open(db_path)?;
db.insert_functions(&functions)?;  // Clean!
```

**Why it's good:**
- Owns the connection (not borrowed)
- Domain-specific methods
- Easy to make generic (just change `Connection` field type)

#### ⚠️ embeddings (Needs Wrapper)
```rust
// src/commands/embeddings/mod.rs
pub fn generate(force: bool) -> Result<()> {
    let mut conn = Connection::open(db_path)?;  // ← Direct rusqlite
    generate_belief_embeddings(&mut conn, &mut *embedder)?;
}

fn generate_belief_embeddings(
    conn: &mut Connection,  // ← Exposed in function signature
    embedder: &mut dyn EmbeddingEngine,
) -> Result<usize>
```

**Problems:**
- Functions take `&mut Connection` parameter
- Callers manage connection lifetime
- Hard to swap backend (would change signatures)

#### 🔴 semantic_search (Worst - Public API Leakage)
```rust
// src/query/semantic_search.rs
pub fn search_beliefs(
    conn: &Connection,  // ← PUBLIC API exposes rusqlite!
    query: &str,
    embedder: &mut dyn EmbeddingEngine,
    top_k: usize,
) -> Result<Vec<BeliefSearchResult>>
```

**Problems:**
- **Public API requires rusqlite::Connection**
- Backend leaks into library interface
- Changing backend breaks all callers
- sqlite-vec specifics exposed ("WHERE embedding match ?")

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

## Module Refactoring Plan

### Phase 1: Create Abstraction Layer
**Goal:** Add `src/db/` module without breaking existing code

```
src/db/
├── mod.rs           # Database trait, factory function
├── config.rs        # Configuration loading
├── sqlite.rs        # SQLite backend
├── turso.rs         # Turso backend (stub initially)
└── vectors.rs       # Vector-specific types
```

**Status:** All existing code still works, new abstraction available

### Phase 2: Refactor scrape/code (Easy)
**Current:**
```rust
// src/commands/scrape/code/database.rs
pub struct Database {
    conn: Connection,  // rusqlite::Connection
}
```

**Target:**
```rust
pub struct CodeDatabase {
    db: Box<dyn crate::db::Database>,  // Generic backend
}

impl CodeDatabase {
    pub fn open_with_config() -> Result<Self> {
        let config = crate::db::load_config()?;
        let db = crate::db::open_database(&config)?;
        Ok(Self { db })
    }

    // All methods stay the same, just use trait instead of Connection
    pub fn insert_functions(&self, functions: &[FunctionFact]) -> Result<usize> {
        let mut stmt = self.db.prepare("...")?;
        // ... same logic
    }
}
```

**Migration:** Update 1 file, call sites unchanged

### Phase 3: Refactor Embeddings (Medium)
**Current:**
```rust
// src/commands/embeddings/mod.rs
pub fn generate(force: bool) -> Result<()> {
    let mut conn = Connection::open(db_path)?;
    generate_belief_embeddings(&mut conn, &mut *embedder)?;
}

fn generate_belief_embeddings(
    conn: &mut Connection,
    embedder: &mut dyn EmbeddingEngine,
) -> Result<usize>
```

**Target:**
```rust
// src/embeddings/database.rs (NEW)
pub struct EmbeddingsDatabase {
    db: Box<dyn crate::db::Database>,
}

impl EmbeddingsDatabase {
    pub fn open_with_config() -> Result<Self> {
        let config = crate::db::load_config()?;
        let db = crate::db::open_database(&config)?;
        Ok(Self { db })
    }

    pub fn generate_belief_embeddings(
        &self,
        embedder: &mut dyn EmbeddingEngine,
    ) -> Result<usize> {
        let mut stmt = self.db.prepare("...")?;
        // ... same logic, uses trait
    }
}

// src/commands/embeddings/mod.rs (UPDATED)
pub fn generate(force: bool) -> Result<()> {
    let db = EmbeddingsDatabase::open_with_config()?;
    let mut embedder = create_embedder()?;

    db.generate_belief_embeddings(&mut *embedder)?;
    db.generate_observation_embeddings(&mut *embedder)?;

    Ok(())
}
```

**Migration:** Create wrapper, update 1 command file

### Phase 4: Refactor Semantic Search (Hard - Public API)
**Current:**
```rust
// src/query/semantic_search.rs (PUBLIC API)
pub fn search_beliefs(
    conn: &Connection,  // ← Leaks rusqlite
    query: &str,
    embedder: &mut dyn EmbeddingEngine,
    top_k: usize,
) -> Result<Vec<BeliefSearchResult>>
```

**Target:**
```rust
// src/query/semantic_search.rs (NEW PUBLIC API)
pub struct SemanticSearch {
    db: Box<dyn crate::db::Database>,
    embedder: Box<dyn crate::embeddings::EmbeddingEngine>,
}

impl SemanticSearch {
    pub fn new(
        db: Box<dyn crate::db::Database>,
        embedder: Box<dyn crate::embeddings::EmbeddingEngine>,
    ) -> Self {
        Self { db, embedder }
    }

    pub fn from_config() -> Result<Self> {
        let config = crate::db::load_config()?;
        let db = crate::db::open_database(&config)?;
        let embedder = crate::embeddings::create_embedder()?;
        Ok(Self::new(db, embedder))
    }

    pub fn search_beliefs(
        &mut self,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<BeliefSearchResult>> {
        // Generate query embedding
        let query_embedding = self.embedder.embed(query)?;

        // Use abstracted vector search
        let matches = self.db.vector_search(
            crate::db::VectorTable::Beliefs,
            &query_embedding,
            None,
            top_k,
        )?;

        matches.into_iter()
            .map(|m| (m.row_id, m.similarity))
            .collect()
    }

    pub fn search_observations(
        &mut self,
        query: &str,
        observation_type: Option<&str>,
        top_k: usize,
    ) -> Result<Vec<ObservationSearchResult>> {
        let query_embedding = self.embedder.embed(query)?;

        let filter = observation_type.map(|t| crate::db::VectorFilter {
            field: "observation_type".to_string(),
            operator: crate::db::FilterOp::Equals,
            value: crate::db::SqlValue::Text(t.to_string()),
        });

        let matches = self.db.vector_search(
            crate::db::VectorTable::Observations,
            &query_embedding,
            filter,
            top_k,
        )?;

        // Extract metadata for observation_type
        matches.into_iter()
            .map(|m| {
                let obs_type = m.metadata.get("observation_type")
                    .and_then(|v| v.as_text())
                    .unwrap_or("unknown")
                    .to_string();
                (m.row_id, obs_type, m.similarity)
            })
            .collect()
    }
}

// DEPRECATED: Keep old functions for backward compat (temporary)
#[deprecated(note = "Use SemanticSearch::search_beliefs instead")]
pub fn search_beliefs(
    conn: &Connection,
    query: &str,
    embedder: &mut dyn EmbeddingEngine,
    top_k: usize,
) -> Result<Vec<BeliefSearchResult>> {
    // Wrapper that uses old API
    todo!("Implement backward compat wrapper or remove")
}
```

**Migration:**
- New code uses `SemanticSearch` struct
- Old code can keep using functions (deprecated)
- Eventually remove deprecated functions

### Phase 5: Add Belief System Module (New Code)
**Target:**
```rust
// src/persona/beliefs.rs (NEW)
pub struct BeliefDatabase {
    db: Box<dyn crate::db::Database>,
}

impl BeliefDatabase {
    pub fn from_config() -> Result<Self> {
        let config = crate::db::load_config()?;
        let db = crate::db::open_database(&config)?;
        Ok(Self { db })
    }

    pub fn insert_belief(&self, statement: &str, value: bool) -> Result<i64> {
        self.db.execute(
            "INSERT INTO beliefs (statement, value) VALUES (?, ?)",
            &[&statement, &value],
        )
    }

    pub fn update_confidence(&self, id: i64, confidence: f32) -> Result<()> {
        self.db.execute(
            "UPDATE beliefs SET confidence = ? WHERE id = ?",
            &[&confidence, &id],
        )?;
        Ok(())
    }

    pub fn get_active_beliefs(&self) -> Result<Vec<Belief>> {
        self.db.query_map(
            "SELECT id, statement, value, confidence FROM beliefs WHERE active = TRUE",
            &[],
            |row| {
                Ok(Belief {
                    id: row.get(0)?,
                    statement: row.get(1)?,
                    value: row.get(2)?,
                    confidence: row.get(3)?,
                })
            },
        )
    }
}

#[derive(Debug, Clone)]
pub struct Belief {
    pub id: i64,
    pub statement: String,
    pub value: bool,
    pub confidence: f32,
}
```

**Migration:** New code, use abstraction from day 1

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

## Migration Timeline

### Week 1: Foundation
- [ ] Day 1-2: Create `src/db/` module structure
- [ ] Day 3-4: Implement `SqliteBackend` (wrap existing code)
- [ ] Day 5: Add configuration system
- [ ] **Milestone:** Existing code still works, abstraction ready

### Week 2: Refactoring
- [ ] Day 1: Refactor scrape/code to use abstraction
- [ ] Day 2-3: Refactor embeddings to use abstraction
- [ ] Day 4-5: Refactor semantic_search to use abstraction
- [ ] **Milestone:** All modules use Database trait

### Week 3: Turso Integration
- [ ] Day 1-2: Implement `TursoBackend` (local mode only)
- [ ] Day 3: Add vector operations for Turso
- [ ] Day 4: Test with both backends
- [ ] Day 5: Add belief system module (new code)
- [ ] **Milestone:** Turso working, configurable switch

### Week 4: Polish & Documentation
- [ ] Day 1-2: Add embedded/remote Turso modes
- [ ] Day 3: Performance testing & optimization
- [ ] Day 4: Documentation & examples
- [ ] Day 5: Update roadmap, archive design doc
- [ ] **Milestone:** Production-ready, documented

---

## Unwinding Strategy

If Turso doesn't work out (API changes, bugs, etc.), unwinding is easy:

### Option 1: Config Switch (Instant)
```bash
# Switch back to SQLite
echo 'backend = "sqlite"' > .patina/config.toml
patina scrape code  # Works immediately!
```

**Effort:** 0 code changes, 1 config change

### Option 2: Remove Turso Backend (Clean)
```bash
# Remove Turso backend
rm src/db/turso.rs

# Update mod.rs
# - Remove `pub mod turso;`
# - Remove Turso from config enum
```

**Effort:** Delete 1 file, update 2 lines

### Option 3: Remove Abstraction (Nuclear)
If abstraction proves too heavy:
- Keep `scrape/code/database.rs` design (it's good!)
- Revert embeddings/semantic_search to direct rusqlite
- Remove `src/db/` entirely

**Effort:** 1-2 days (basically reverting refactor commits)

**Key Point:** Because we're refactoring to a clean abstraction (not Turso-specific hacks), unwinding is straightforward.

---

## Success Criteria

### Phase 1 Complete (Abstraction)
- [x] `src/db/` module created
- [x] Database trait defined
- [x] SqliteBackend implemented
- [x] Configuration system working
- [x] All existing tests pass

### Phase 2 Complete (Refactoring)
- [ ] scrape/code uses abstraction
- [ ] embeddings uses abstraction
- [ ] semantic_search uses abstraction
- [ ] All tests pass with SQLite backend

### Phase 3 Complete (Turso Integration)
- [ ] TursoBackend implemented (local mode)
- [ ] Vector operations working with Turso
- [ ] Config switch works seamlessly
- [ ] Tests pass with both backends

### Phase 4 Complete (Production Ready)
- [ ] Embedded/remote Turso modes added
- [ ] Performance benchmarks completed
- [ ] Documentation written
- [ ] Belief system module added
- [ ] Roadmap updated

---

## Open Questions

1. **Turso Stability**
   - Turso is beta - what's the plan if APIs break?
   - Answer: Easy unwinding via config switch

2. **Performance Differences**
   - Will Turso native vectors be faster than sqlite-vec?
   - Need benchmarks with real data

3. **Sync Strategy (Embedded Mode)**
   - Auto-sync on every write? Periodic? Manual?
   - Start with manual, add auto-sync later

4. **Schema Migrations**
   - How to handle schema changes across backends?
   - Version schema, test migrations with both

5. **Transaction Semantics**
   - Do Turso transactions work identically to SQLite?
   - Test thoroughly, document differences

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
