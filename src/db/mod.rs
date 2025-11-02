//! Database abstraction for Patina
//!
//! Multi-backend support using enum dispatch:
//! - `DatabaseBackend` enum for runtime backend selection
//! - `DatabaseConfig` for configuration-based backend selection
//! - `SqliteDatabase` for direct SQLite access (legacy)
//! - Simple wrappers around backend connections
//! - Domain-specific operations in domain modules
//!
//! # Example - Direct backend usage
//! ```no_run
//! use patina::db::DatabaseBackend;
//!
//! let db = DatabaseBackend::open_sqlite(".patina/db/facts.db")?;
//! db.execute("CREATE TABLE test (id INTEGER)", &[])?;
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Example - Config-based usage
//! ```no_run
//! use patina::db::DatabaseConfig;
//!
//! let config = DatabaseConfig::load_from_file(".patina/config.toml")?;
//! let db = config.open()?;
//! db.execute("CREATE TABLE test (id INTEGER)", &[])?;
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod backend;
pub mod config;
pub mod sqlite;
pub mod vectors;

pub use backend::DatabaseBackend;
pub use config::{BackendType, DatabaseConfig, SqliteConfig, TursoConfig, TursoMode};
pub use sqlite::SqliteDatabase;
pub use vectors::{VectorFilter, VectorMatch, VectorTable};
