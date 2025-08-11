//! SQLite database client for pattern storage

use anyhow::{Context, Result};
use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::DocumentInfo;

/// SQLite client wrapper for embedded database
pub struct SqliteClient {
    conn: Arc<Mutex<Connection>>,
}

/// Document record for database queries
#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentRecord {
    pub id: String,
    pub path: String,
    pub layer: String,
    pub title: String,
    pub summary: String,
    pub metadata: String, // JSON as string
}

/// Concept record for database queries
#[derive(Debug, Serialize, Deserialize)]
pub struct ConceptRecord {
    pub concept: String,
    pub document_id: String,
    pub relevance: String,
    pub confidence: f64,
}

impl SqliteClient {
    /// Create or open SQLite database
    pub fn new(db_path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory for database: {parent:?}"))?;
        }

        // Open connection
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open SQLite database at {db_path:?}"))?;

        // Configure for better performance and reliability
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA temp_store = MEMORY;
            PRAGMA mmap_size = 30000000000;
            ",
        )
        .context("Failed to configure SQLite pragmas")?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Initialize the database schema
    pub fn initialize_schema(&self) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        // Create documents table
        tx.execute(
            "CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                layer TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                last_indexed TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .context("Failed to create documents table")?;

        // Create concepts table
        tx.execute(
            "CREATE TABLE IF NOT EXISTS concepts (
                concept TEXT NOT NULL,
                document_id TEXT NOT NULL,
                relevance TEXT NOT NULL DEFAULT '',
                confidence REAL NOT NULL DEFAULT 1.0,
                PRIMARY KEY (concept, document_id),
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            )",
            [],
        )
        .context("Failed to create concepts table")?;

        // Create relationships table
        tx.execute(
            "CREATE TABLE IF NOT EXISTS relationships (
                from_doc TEXT NOT NULL,
                to_doc TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                metadata TEXT DEFAULT '{}',
                PRIMARY KEY (from_doc, to_doc, relationship_type),
                FOREIGN KEY (from_doc) REFERENCES documents(id) ON DELETE CASCADE,
                FOREIGN KEY (to_doc) REFERENCES documents(id) ON DELETE CASCADE
            )",
            [],
        )
        .context("Failed to create relationships table")?;

        // Create git_states table
        tx.execute(
            "CREATE TABLE IF NOT EXISTS git_states (
                document_id TEXT NOT NULL,
                workspace_id TEXT,
                state TEXT NOT NULL,
                confidence_modifier REAL DEFAULT 1.0,
                metadata TEXT DEFAULT '{}',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (document_id, workspace_id),
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            )",
            [],
        )
        .context("Failed to create git_states table")?;

        // Create state_transitions table
        tx.execute(
            "CREATE TABLE IF NOT EXISTS state_transitions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace_id TEXT NOT NULL,
                document_id TEXT,
                from_state TEXT,
                to_state TEXT NOT NULL,
                transition_reason TEXT,
                metadata TEXT DEFAULT '{}',
                occurred_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .context("Failed to create state_transitions table")?;

        // Create indexes for performance
        let indexes = vec![
            "CREATE INDEX IF NOT EXISTS idx_concepts_concept ON concepts(concept)",
            "CREATE INDEX IF NOT EXISTS idx_concepts_doc ON concepts(document_id)",
            "CREATE INDEX IF NOT EXISTS idx_documents_layer ON documents(layer)",
            "CREATE INDEX IF NOT EXISTS idx_git_states_workspace ON git_states(workspace_id)",
            "CREATE INDEX IF NOT EXISTS idx_git_states_document ON git_states(document_id)",
            "CREATE INDEX IF NOT EXISTS idx_documents_path ON documents(path)",
        ];

        for index_sql in indexes {
            tx.execute(index_sql, [])
                .with_context(|| format!("Failed to create index: {index_sql}"))?;
        }

        tx.commit().context("Failed to commit schema creation")?;
        Ok(())
    }

    /// Store a document in the database
    pub fn store_document(&self, doc: &DocumentInfo) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        // Serialize metadata
        let metadata_json = serde_json::to_string(&doc.metadata)?;

        // Insert or update document
        tx.execute(
            "INSERT OR REPLACE INTO documents (id, path, layer, title, summary, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &doc.id,
                doc.path.to_string_lossy(),
                format!("{:?}", doc.layer),
                &doc.title,
                &doc.summary,
                &metadata_json
            ],
        )?;

        // Clear existing concepts for this document
        tx.execute(
            "DELETE FROM concepts WHERE document_id = ?1",
            params![&doc.id],
        )?;

        // Insert new concepts
        for concept in &doc.concepts {
            tx.execute(
                "INSERT INTO concepts (concept, document_id, relevance, confidence)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    concept.to_lowercase(),
                    &doc.id,
                    format!("Defines {concept}"),
                    1.0
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Get all documents from the database
    pub fn get_all_documents(&self) -> Result<Vec<DocumentRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, path, layer, title, summary, metadata FROM documents")?;

        let docs = stmt
            .query_map([], |row| {
                Ok(DocumentRecord {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    layer: row.get(2)?,
                    title: row.get(3)?,
                    summary: row.get(4)?,
                    metadata: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(docs)
    }

    /// Get all concepts from the database
    pub fn get_all_concepts(&self) -> Result<Vec<ConceptRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT concept, document_id, relevance, confidence FROM concepts")?;

        let concepts = stmt
            .query_map([], |row| {
                Ok(ConceptRecord {
                    concept: row.get(0)?,
                    document_id: row.get(1)?,
                    relevance: row.get(2)?,
                    confidence: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(concepts)
    }

    /// Store a git state transition
    pub fn store_state_transition(
        &self,
        workspace_id: &str,
        document_id: Option<&str>,
        from_state: Option<&str>,
        to_state: &str,
        reason: Option<&str>,
        metadata: &serde_json::Value,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let metadata_json = serde_json::to_string(metadata)?;

        conn.execute(
            "INSERT INTO state_transitions 
             (workspace_id, document_id, from_state, to_state, transition_reason, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                workspace_id,
                document_id,
                from_state,
                to_state,
                reason,
                metadata_json
            ],
        )?;

        Ok(())
    }

    /// Get document by path
    pub fn get_document_by_path(&self, path: &Path) -> Result<Option<DocumentRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, path, layer, title, summary, metadata 
             FROM documents 
             WHERE path = ?1",
        )?;

        let mut docs = stmt.query_map(params![path.to_string_lossy()], |row| {
            Ok(DocumentRecord {
                id: row.get(0)?,
                path: row.get(1)?,
                layer: row.get(2)?,
                title: row.get(3)?,
                summary: row.get(4)?,
                metadata: row.get(5)?,
            })
        })?;

        Ok(docs.next().transpose()?)
    }

    /// Perform a database transaction
    pub fn transaction<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Transaction) -> Result<R>,
    {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let result = f(&tx)?;
        tx.commit()?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::Layer;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_sqlite_client_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let _client = SqliteClient::new(&db_path).unwrap();
        assert!(db_path.exists());
    }

    #[test]
    fn test_schema_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let client = SqliteClient::new(&db_path).unwrap();
        client.initialize_schema().unwrap();

        // Try to initialize again - should not fail
        client.initialize_schema().unwrap();
    }

    #[test]
    fn test_document_storage() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let client = SqliteClient::new(&db_path).unwrap();
        client.initialize_schema().unwrap();

        let doc = DocumentInfo {
            id: "test-doc".to_string(),
            path: PathBuf::from("/test/path.md"),
            layer: Layer::Core,
            title: "Test Document".to_string(),
            summary: "A test document".to_string(),
            concepts: vec!["test".to_string(), "document".to_string()],
            metadata: std::collections::HashMap::new(),
        };

        client.store_document(&doc).unwrap();

        // Verify storage
        let stored = client.get_document_by_path(&doc.path).unwrap();
        assert!(stored.is_some());
        let stored = stored.unwrap();
        assert_eq!(stored.id, doc.id);
        assert_eq!(stored.title, doc.title);
    }
}
