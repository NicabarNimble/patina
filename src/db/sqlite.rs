//! SQLite database wrapper with sqlite-vec support
//!
//! Follows the same pattern as src/commands/scrape/code/database.rs

use anyhow::{Context, Result};
use rusqlite::{ffi::sqlite3_auto_extension, params, Connection};
use sqlite_vec::sqlite3_vec_init;
use std::path::Path;
use zerocopy::AsBytes;

use super::vectors::{VectorFilter, VectorMatch, VectorTable};

/// SQLite database with sqlite-vec extension
pub struct SqliteDatabase {
    conn: Connection,
}

impl SqliteDatabase {
    /// Open or create a SQLite database file
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path).context("Failed to open SQLite database")?;

        // Load sqlite-vec extension
        Self::load_vec_extension(&conn)?;

        Ok(Self { conn })
    }

    /// Create an in-memory database for testing
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("Failed to create in-memory database")?;

        Self::load_vec_extension(&conn)?;

        Ok(Self { conn })
    }

    /// Load sqlite-vec extension
    fn load_vec_extension(conn: &Connection) -> Result<()> {
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
            conn.load_extension_enable()
                .context("Failed to enable extension loading")?;
        }
        Ok(())
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

    /// Search for vectors using semantic similarity
    pub fn vector_search(
        &self,
        table: VectorTable,
        query_vector: &[f32],
        filter: Option<VectorFilter>,
        limit: usize,
    ) -> Result<Vec<VectorMatch>> {
        let table_name = table.table_name();

        // Build SQL for sqlite-vec
        let sql = if let Some(ref f) = filter {
            format!(
                "SELECT rowid, distance FROM {}
                 WHERE embedding MATCH ? AND {} = ?
                 ORDER BY distance LIMIT ?",
                table_name, f.field
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
        let vector_bytes = query_vector.as_bytes();

        // Execute query
        let mut stmt = self.conn.prepare(&sql)?;

        let results = if let Some(f) = filter {
            stmt.query_map(params![vector_bytes, &f.value, limit], |row| {
                let row_id: i64 = row.get(0)?;
                let distance: f32 = row.get(1)?;
                Ok(VectorMatch::new(row_id, distance))
            })?
            .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![vector_bytes, limit], |row| {
                let row_id: i64 = row.get(0)?;
                let distance: f32 = row.get(1)?;
                Ok(VectorMatch::new(row_id, distance))
            })?
            .collect::<Result<Vec<_>, _>>()?
        };

        Ok(results)
    }

    /// Insert a vector into a vector table
    pub fn vector_insert(
        &self,
        table: VectorTable,
        id: i64,
        vector: &[f32],
        metadata: Option<&str>, // Simple string metadata for now
    ) -> Result<()> {
        let table_name = table.table_name();
        let vector_bytes = vector.as_bytes();

        let sql = if metadata.is_some() {
            format!(
                "INSERT INTO {} (rowid, embedding, observation_type) VALUES (?, ?, ?)",
                table_name
            )
        } else {
            format!("INSERT INTO {} (rowid, embedding) VALUES (?, ?)", table_name)
        };

        if let Some(meta) = metadata {
            self.conn.execute(&sql, params![id, vector_bytes, meta])?;
        } else {
            self.conn.execute(&sql, params![id, vector_bytes])?;
        }

        Ok(())
    }

    /// Get backend name
    pub fn backend_name(&self) -> &'static str {
        "sqlite-vec"
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
        assert_eq!(db.backend_name(), "sqlite-vec");
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
        let name: String = db
            .connection()
            .query_row("SELECT name FROM test WHERE id = ?", [1], |row| row.get(0))?;
        assert_eq!(name, "test");

        Ok(())
    }
}
