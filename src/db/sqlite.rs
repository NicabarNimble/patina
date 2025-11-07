//! SQLite database wrapper
//!
//! Simple wrapper around rusqlite::Connection for basic database operations.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

/// SQLite database wrapper
#[derive(Debug)]
pub struct SqliteDatabase {
    conn: Connection,
}

impl SqliteDatabase {
    /// Open or create a SQLite database file
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path).context("Failed to open SQLite database")?;
        Ok(Self { conn })
    }

    /// Create an in-memory database for testing
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("Failed to create in-memory database")?;
        Ok(Self { conn })
    }

    /// Execute a SQL statement (INSERT, UPDATE, DELETE, etc.)
    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize> {
        let count = self
            .conn
            .execute(sql, params)
            .with_context(|| format!("Failed to execute: {}", sql))?;
        Ok(count)
    }

    /// Execute a batch of SQL statements
    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        self.conn
            .execute_batch(sql)
            .context("Failed to execute batch")?;
        Ok(())
    }

    /// Get backend name
    pub fn backend_name(&self) -> &'static str {
        "sqlite"
    }

    /// Get reference to underlying connection (temporary escape hatch)
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Get mutable reference to underlying connection (for transactions)
    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_database_creation() -> Result<()> {
        let db = SqliteDatabase::open_in_memory()?;
        assert_eq!(db.backend_name(), "sqlite");
        Ok(())
    }

    #[test]
    fn test_basic_operations() -> Result<()> {
        let db = SqliteDatabase::open_in_memory()?;

        // Create table
        db.execute("CREATE TABLE test (id INTEGER, name TEXT)", &[])?;

        // Insert data
        let count = db.execute("INSERT INTO test VALUES (?, ?)", &[&1, &"test"])?;
        assert_eq!(count, 1);

        // Query
        let name: String =
            db.connection()
                .query_row("SELECT name FROM test WHERE id = ?", [1], |row| row.get(0))?;
        assert_eq!(name, "test");

        Ok(())
    }
}
