//! Database backend enum for multi-backend support
//!
//! Provides a unified interface over different database backends (SQLite, Turso)
//! using enum dispatch for type-safe, compile-time polymorphism.

use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

use super::sqlite::SqliteDatabase;
use super::vectors::{VectorFilter, VectorMatch, VectorTable};

/// Database backend enum supporting multiple implementations
///
/// Uses enum dispatch for zero-cost abstraction over different backends.
/// Each variant wraps a specific backend implementation.
#[derive(Debug)]
pub enum DatabaseBackend {
    /// SQLite backend with sqlite-vec extension
    Sqlite(SqliteDatabase),
    // TODO: Add Turso variant
    // Turso(TursoDatabase),
}

impl DatabaseBackend {
    /// Open a SQLite database file
    pub fn open_sqlite<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::Sqlite(SqliteDatabase::open(path)?))
    }

    /// Create an in-memory SQLite database for testing
    pub fn open_sqlite_in_memory() -> Result<Self> {
        Ok(Self::Sqlite(SqliteDatabase::open_in_memory()?))
    }

    /// Execute a SQL statement (INSERT, UPDATE, DELETE, etc.)
    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize> {
        match self {
            Self::Sqlite(db) => db.execute(sql, params),
        }
    }

    /// Execute a batch of SQL statements
    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        match self {
            Self::Sqlite(db) => db.execute_batch(sql),
        }
    }

    /// Search for vectors using semantic similarity
    pub fn vector_search(
        &self,
        table: VectorTable,
        query_vector: &[f32],
        filter: Option<VectorFilter>,
        limit: usize,
    ) -> Result<Vec<VectorMatch>> {
        match self {
            Self::Sqlite(db) => db.vector_search(table, query_vector, filter, limit),
        }
    }

    /// Insert a vector into a vector table
    pub fn vector_insert(
        &self,
        table: VectorTable,
        id: i64,
        vector: &[f32],
        metadata: Option<&str>,
    ) -> Result<()> {
        match self {
            Self::Sqlite(db) => db.vector_insert(table, id, vector, metadata),
        }
    }

    /// Get backend name for debugging/logging
    pub fn backend_name(&self) -> &'static str {
        match self {
            Self::Sqlite(db) => db.backend_name(),
        }
    }

    /// Get reference to underlying connection (temporary escape hatch)
    ///
    /// Only available for SQLite backend. Will need refactoring when
    /// adding Turso support.
    pub fn connection(&self) -> Option<&Connection> {
        match self {
            Self::Sqlite(db) => Some(db.connection()),
        }
    }

    /// Get mutable reference to underlying connection (for transactions)
    ///
    /// Only available for SQLite backend. Will need refactoring when
    /// adding Turso support.
    pub fn connection_mut(&mut self) -> Option<&mut Connection> {
        match self {
            Self::Sqlite(db) => Some(db.connection_mut()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_backend_creation() -> Result<()> {
        let db = DatabaseBackend::open_sqlite_in_memory()?;
        assert_eq!(db.backend_name(), "sqlite-vec");
        Ok(())
    }

    #[test]
    fn test_backend_execute() -> Result<()> {
        let db = DatabaseBackend::open_sqlite_in_memory()?;

        // Create table
        db.execute("CREATE TABLE test (id INTEGER, name TEXT)", &[])?;

        // Insert data
        let count = db.execute("INSERT INTO test VALUES (?, ?)", &[&1, &"test"])?;
        assert_eq!(count, 1);

        Ok(())
    }

    #[test]
    fn test_backend_enum_dispatch() -> Result<()> {
        let db = DatabaseBackend::open_sqlite_in_memory()?;

        // Verify enum dispatch works
        match db {
            DatabaseBackend::Sqlite(_) => {
                // Expected
            }
        }

        Ok(())
    }
}
