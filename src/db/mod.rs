//! Database abstraction for Patina
//!
//! Follows the same pattern as scrape/code/database.rs:
//! - Concrete types, no traits
//! - Simple wrappers around backend connections
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
