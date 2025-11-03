//! Database abstraction for Patina
//!
//! Direct SQLite usage with sqlite-vec extension:
//! - `SqliteDatabase` for SQLite database operations
//! - Simple wrappers around SQLite connections
//! - Domain-specific operations in domain modules
//!
//! # Example
//! ```no_run
//! use patina::db::SqliteDatabase;
//!
//! let db = SqliteDatabase::open(".patina/db/facts.db")?;
//! db.execute("CREATE TABLE test (id INTEGER)", &[])?;
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod sqlite;
pub mod vectors;

pub use sqlite::SqliteDatabase;
pub use vectors::{VectorFilter, VectorMatch, VectorTable};
