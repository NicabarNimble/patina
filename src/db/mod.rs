//! Database abstraction for Patina
//!
//! Multi-backend support using enum dispatch:
//! - `DatabaseBackend` enum for runtime backend selection
//! - `SqliteDatabase` for direct SQLite access (legacy)
//! - Simple wrappers around backend connections
//! - Domain-specific operations in domain modules
//!
//! # Example
//! ```no_run
//! use patina::db::DatabaseBackend;
//!
//! let db = DatabaseBackend::open_sqlite(".patina/db/facts.db")?;
//! db.execute("CREATE TABLE test (id INTEGER)", &[])?;
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod backend;
pub mod sqlite;
pub mod vectors;

pub use backend::DatabaseBackend;
pub use sqlite::SqliteDatabase;
pub use vectors::{VectorFilter, VectorMatch, VectorTable};
