//! Database abstraction for Patina
//!
//! Simple SQLite wrapper for basic database operations.
//! Vector storage uses the dedicated `storage` module with USearch.
//!
//! # Example
//! ```no_run
//! use patina::db::SqliteDatabase;
//!
//! let db = SqliteDatabase::open(".patina/local/data/facts.db")?;
//! db.execute("CREATE TABLE test (id INTEGER)", &[])?;
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod sqlite;

pub use sqlite::SqliteDatabase;
