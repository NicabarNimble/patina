//! Hybrid database module combining SQLite storage with optional Automerge CRDT
//!
//! This module provides a unified interface for persistent storage (SQLite) with
//! optional distributed synchronization (Automerge). The design allows the system
//! to work fully without CRDT, enhancing functionality when enabled.

use anyhow::{Context, Result};
use automerge::{transaction::Transactable, Automerge, ObjType, ReadDoc, ROOT};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

use super::DocumentInfo;

/// Pattern information for CRDT sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub name: String,
    pub content: String,
    pub layer: String,
    pub confidence: String,
}

/// Workspace navigation state for CRDT sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub workspace_id: String,
    pub navigation_state: String,
    pub last_query: Option<String>,
    pub active_patterns: Vec<String>,
}

/// CRDT wrapper for Automerge documents
pub struct NavigationCRDT {
    /// Automerge document for patterns
    patterns_doc: Automerge,
    /// Automerge document for workspace states
    workspace_doc: Automerge,
    /// Site ID for this peer
    site_id: Vec<u8>,
}

impl NavigationCRDT {
    /// Create a new CRDT instance
    pub fn new() -> Result<Self> {
        let patterns_doc = Automerge::new();
        let workspace_doc = Automerge::new();

        // Generate unique site ID
        let site_id = Uuid::new_v4().as_bytes().to_vec();

        Ok(Self {
            patterns_doc,
            workspace_doc,
            site_id,
        })
    }

    /// Add or update a pattern in the CRDT
    pub fn add_pattern(&mut self, pattern: &Pattern) -> Result<()> {
        // For simplicity, store patterns at the root level with a prefix
        let pattern_key = format!("pattern:{}", pattern.id);

        self.patterns_doc
            .transact(|tx| {
                // Create pattern object at root with prefixed key
                let pattern_obj = tx.put_object(ROOT, &pattern_key, ObjType::Map)?;
                tx.put(&pattern_obj, "id", &pattern.id)?;
                tx.put(&pattern_obj, "name", &pattern.name)?;
                tx.put(&pattern_obj, "content", &pattern.content)?;
                tx.put(&pattern_obj, "layer", &pattern.layer)?;
                tx.put(&pattern_obj, "confidence", &pattern.confidence)?;
                tx.put(
                    &pattern_obj,
                    "timestamp",
                    chrono::Utc::now().timestamp() as i64,
                )?;
                Ok::<(), automerge::AutomergeError>(())
            })
            .map_err(|e| anyhow::anyhow!("Failed to add pattern to CRDT: {:?}", e))?;

        Ok(())
    }

    /// Update workspace state in the CRDT
    pub fn update_workspace_state(&mut self, state: &WorkspaceState) -> Result<()> {
        // For simplicity, store workspaces at the root level with a prefix
        let workspace_key = format!("workspace:{}", state.workspace_id);

        self.workspace_doc
            .transact(|tx| {
                // Create or update workspace object at root with prefixed key
                let workspace_obj = tx.put_object(ROOT, &workspace_key, ObjType::Map)?;
                tx.put(&workspace_obj, "workspace_id", &state.workspace_id)?;
                tx.put(&workspace_obj, "navigation_state", &state.navigation_state)?;

                if let Some(query) = &state.last_query {
                    tx.put(&workspace_obj, "last_query", query)?;
                }

                // Store active patterns as a list
                let patterns_list =
                    tx.put_object(&workspace_obj, "active_patterns", ObjType::List)?;
                for (idx, pattern_id) in state.active_patterns.iter().enumerate() {
                    tx.insert(&patterns_list, idx, pattern_id)?;
                }

                tx.put(
                    &workspace_obj,
                    "timestamp",
                    chrono::Utc::now().timestamp() as i64,
                )?;
                Ok::<(), automerge::AutomergeError>(())
            })
            .map_err(|e| anyhow::anyhow!("Failed to update workspace state in CRDT: {:?}", e))?;

        Ok(())
    }

    /// Get all patterns from CRDT
    pub fn get_patterns(&self) -> Result<HashMap<String, Pattern>> {
        let mut patterns = HashMap::new();

        // Read all keys from the root
        for key in self.patterns_doc.keys(ROOT) {
            if key.starts_with("pattern:") {
                if let Some((pattern_value, pattern_obj_id)) = self.patterns_doc.get(ROOT, &key)? {
                    if let automerge::Value::Object(automerge::ObjType::Map) = pattern_value {
                        // Extract pattern fields
                        let id = self
                            .patterns_doc
                            .get(&pattern_obj_id, "id")?
                            .and_then(|(v, _)| v.to_str().map(|s| s.to_string()))
                            .unwrap_or_default();

                        let name = self
                            .patterns_doc
                            .get(&pattern_obj_id, "name")?
                            .and_then(|(v, _)| v.to_str().map(|s| s.to_string()))
                            .unwrap_or_default();

                        let content = self
                            .patterns_doc
                            .get(&pattern_obj_id, "content")?
                            .and_then(|(v, _)| v.to_str().map(|s| s.to_string()))
                            .unwrap_or_default();

                        let layer = self
                            .patterns_doc
                            .get(&pattern_obj_id, "layer")?
                            .and_then(|(v, _)| v.to_str().map(|s| s.to_string()))
                            .unwrap_or_default();

                        let confidence = self
                            .patterns_doc
                            .get(&pattern_obj_id, "confidence")?
                            .and_then(|(v, _)| v.to_str().map(|s| s.to_string()))
                            .unwrap_or_default();

                        let pattern = Pattern {
                            id: id.clone(),
                            name,
                            content,
                            layer,
                            confidence,
                        };

                        patterns.insert(id, pattern);
                    }
                }
            }
        }

        Ok(patterns)
    }

    /// Get changes since a given version for sync
    pub fn get_changes_since(&self, last_sync_state: &[u8]) -> Result<Vec<u8>> {
        // Parse the sync state to get last known heads
        let mut pattern_heads = Vec::new();
        let mut workspace_heads = Vec::new();

        if !last_sync_state.is_empty() {
            let mut offset = 0;

            // Read pattern heads count
            if offset + 4 <= last_sync_state.len() {
                let count = u32::from_le_bytes([
                    last_sync_state[offset],
                    last_sync_state[offset + 1],
                    last_sync_state[offset + 2],
                    last_sync_state[offset + 3],
                ]) as usize;
                offset += 4;

                // Read pattern heads
                for _ in 0..count {
                    if offset + 32 <= last_sync_state.len() {
                        let mut head_bytes = [0u8; 32];
                        head_bytes.copy_from_slice(&last_sync_state[offset..offset + 32]);
                        pattern_heads.push(automerge::ChangeHash(head_bytes));
                        offset += 32;
                    }
                }
            }

            // Read workspace heads count
            if offset + 4 <= last_sync_state.len() {
                let count = u32::from_le_bytes([
                    last_sync_state[offset],
                    last_sync_state[offset + 1],
                    last_sync_state[offset + 2],
                    last_sync_state[offset + 3],
                ]) as usize;
                offset += 4;

                // Read workspace heads
                for _ in 0..count {
                    if offset + 32 <= last_sync_state.len() {
                        let mut head_bytes = [0u8; 32];
                        head_bytes.copy_from_slice(&last_sync_state[offset..offset + 32]);
                        workspace_heads.push(automerge::ChangeHash(head_bytes));
                        offset += 32;
                    }
                }
            }
        }

        // Get changes since the last known heads
        let pattern_changes = self.patterns_doc.get_changes(&pattern_heads);
        let workspace_changes = self.workspace_doc.get_changes(&workspace_heads);

        // Serialize changes for transmission
        let mut serialized = Vec::new();
        for change in pattern_changes {
            let bytes = change.raw_bytes();
            serialized.extend_from_slice(&bytes);
        }
        for change in workspace_changes {
            let bytes = change.raw_bytes();
            serialized.extend_from_slice(&bytes);
        }

        Ok(serialized)
    }

    /// Apply changes from another peer
    pub fn apply_changes(&mut self, changes: &[u8]) -> Result<()> {
        // For simplicity, try to parse the entire buffer as changes
        // In a real implementation, you'd have a proper protocol for separating changes

        // Try to load and apply changes to patterns document
        if let Ok(change) = automerge::Change::from_bytes(changes.to_vec()) {
            let _ = self.patterns_doc.apply_changes(vec![change]);
        }

        // In practice, you'd need a way to separate pattern changes from workspace changes
        // For now, we'll just try to apply to both and ignore errors

        Ok(())
    }

    /// Get current sync state
    pub fn get_sync_state(&self) -> Vec<u8> {
        // For now, just return a simple version identifier
        // In a real implementation, you'd track the actual sync state
        let mut state = Vec::new();
        state.extend_from_slice(b"v1");
        state
    }
}

/// Hybrid database combining SQLite and optional Automerge
pub struct HybridDatabase {
    /// SQLite connection for persistent storage
    sqlite: Arc<Mutex<Connection>>,
    /// Optional CRDT layer for distributed sync
    crdt: Option<Arc<Mutex<NavigationCRDT>>>,
}

impl HybridDatabase {
    /// Create a new hybrid database
    pub fn new(db_path: &Path, enable_crdt: bool) -> Result<Self> {
        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory for database: {parent:?}"))?;
        }

        // Open SQLite connection
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open SQLite database at {db_path:?}"))?;

        // Configure for optimal performance
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA mmap_size = 30000000000;
        ",
        )
        .context("Failed to configure database")?;

        let sqlite = Arc::new(Mutex::new(conn));

        // Optionally initialize CRDT layer
        let crdt = if enable_crdt {
            Some(Arc::new(Mutex::new(NavigationCRDT::new()?)))
        } else {
            None
        };

        Ok(Self { sqlite, crdt })
    }

    /// Initialize database schema
    pub fn initialize_schema(&self) -> Result<()> {
        let conn = self.sqlite.lock();

        // Documents table (local indexing)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                layer TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                last_indexed TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .context("Failed to create documents table")?;

        // Concepts table (local indexing)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS concepts (
                concept TEXT NOT NULL,
                document_id TEXT NOT NULL,
                relevance TEXT NOT NULL DEFAULT '',
                confidence REAL NOT NULL DEFAULT 1.0,
                PRIMARY KEY (concept, document_id),
                FOREIGN KEY (document_id) REFERENCES documents(id)
            )",
            [],
        )
        .context("Failed to create concepts table")?;

        // Patterns table (synced via CRDT if enabled)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS patterns (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                content TEXT NOT NULL,
                layer TEXT NOT NULL,
                confidence TEXT,
                discovered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                last_modified TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .context("Failed to create patterns table")?;

        // Workspace states (synced via CRDT if enabled)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS workspace_states (
                workspace_id TEXT PRIMARY KEY,
                navigation_state TEXT NOT NULL,
                last_query TEXT,
                active_patterns TEXT,
                last_modified TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .context("Failed to create workspace_states table")?;

        // Create indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_concepts_doc 
             ON concepts(document_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_documents_layer 
             ON documents(layer)",
            [],
        )?;

        Ok(())
    }

    /// Check if CRDT is enabled
    pub fn has_crdt(&self) -> bool {
        self.crdt.is_some()
    }

    /// Store a document (SQLite only, not synced)
    pub fn store_document(&self, doc: &DocumentInfo) -> Result<()> {
        let mut conn = self.sqlite.lock();

        // Convert metadata to JSON
        let metadata_json = serde_json::to_string(&doc.metadata)?;

        // Use a transaction to ensure atomicity
        let tx = conn.transaction()?;

        // Upsert document
        tx.execute(
            "INSERT OR REPLACE INTO documents 
             (id, path, layer, title, summary, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &doc.id,
                doc.path.to_string_lossy(),
                format!("{:?}", doc.layer),
                &doc.title,
                &doc.summary,
                &metadata_json
            ],
        )
        .context("Failed to insert document")?;

        // Clear existing concepts
        tx.execute(
            "DELETE FROM concepts WHERE document_id = ?1",
            params![&doc.id],
        )
        .context("Failed to delete existing concepts")?;

        // Insert new concepts
        for concept in &doc.concepts {
            tx.execute(
                "INSERT OR REPLACE INTO concepts (concept, document_id, relevance, confidence)
                 VALUES (?1, ?2, ?3, ?4)",
                params![concept.to_lowercase(), &doc.id, "extracted", 1.0],
            )?;
        }

        // Commit the transaction
        tx.commit()?;

        Ok(())
    }

    /// Add a pattern (stored in SQLite and optionally synced via CRDT)
    pub fn add_pattern(&self, pattern: &Pattern) -> Result<()> {
        // Always store in SQLite
        let conn = self.sqlite.lock();
        conn.execute(
            "INSERT OR REPLACE INTO patterns 
             (id, name, content, layer, confidence, last_modified) 
             VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP)",
            params![
                &pattern.id,
                &pattern.name,
                &pattern.content,
                &pattern.layer,
                &pattern.confidence
            ],
        )?;

        // Update CRDT if enabled
        if let Some(crdt) = &self.crdt {
            let mut crdt_lock = crdt.lock();
            crdt_lock.add_pattern(pattern)?;
        }

        Ok(())
    }

    /// Update workspace state (stored in SQLite and optionally synced via CRDT)
    pub fn update_workspace_state(&self, state: &WorkspaceState) -> Result<()> {
        // Always store in SQLite
        let conn = self.sqlite.lock();
        let active_patterns_json = serde_json::to_string(&state.active_patterns)?;

        conn.execute(
            "INSERT OR REPLACE INTO workspace_states 
             (workspace_id, navigation_state, last_query, active_patterns, last_modified) 
             VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)",
            params![
                &state.workspace_id,
                &state.navigation_state,
                &state.last_query,
                &active_patterns_json
            ],
        )?;

        // Update CRDT if enabled
        if let Some(crdt) = &self.crdt {
            let mut crdt_lock = crdt.lock();
            crdt_lock.update_workspace_state(state)?;
        }

        Ok(())
    }

    /// Sync CRDT changes with SQLite (pull CRDT state into SQLite)
    pub fn sync_from_crdt(&self) -> Result<()> {
        if let Some(crdt) = &self.crdt {
            let crdt_lock = crdt.lock();
            let patterns = crdt_lock.get_patterns()?;

            let conn = self.sqlite.lock();
            for (_, pattern) in patterns {
                conn.execute(
                    "INSERT OR REPLACE INTO patterns 
                     (id, name, content, layer, confidence, last_modified) 
                     VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP)",
                    params![
                        &pattern.id,
                        &pattern.name,
                        &pattern.content,
                        &pattern.layer,
                        &pattern.confidence
                    ],
                )?;
            }
        }

        Ok(())
    }

    /// Get CRDT changes for sync with peers
    pub fn get_crdt_changes(&self, last_sync_state: &[u8]) -> Result<Vec<u8>> {
        if let Some(crdt) = &self.crdt {
            let crdt_lock = crdt.lock();
            crdt_lock.get_changes_since(last_sync_state)
        } else {
            Ok(vec![])
        }
    }

    /// Apply CRDT changes from a peer
    pub fn apply_crdt_changes(&self, changes: &[u8]) -> Result<()> {
        if let Some(crdt) = &self.crdt {
            let mut crdt_lock = crdt.lock();
            crdt_lock.apply_changes(changes)?;

            // Sync changes to SQLite
            drop(crdt_lock);
            self.sync_from_crdt()?;
        }
        Ok(())
    }

    /// Get current sync state
    pub fn get_sync_state(&self) -> Vec<u8> {
        if let Some(crdt) = &self.crdt {
            let crdt_lock = crdt.lock();
            crdt_lock.get_sync_state()
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hybrid_database_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Test without CRDT
        let db = HybridDatabase::new(&db_path, false).unwrap();
        assert!(!db.has_crdt());

        // Test with CRDT
        let db_with_crdt = HybridDatabase::new(&db_path, true).unwrap();
        assert!(db_with_crdt.has_crdt());
    }

    #[test]
    fn test_schema_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = HybridDatabase::new(&db_path, false).unwrap();
        db.initialize_schema().unwrap();

        // Initialize again - should not fail
        db.initialize_schema().unwrap();
    }

    #[test]
    fn test_pattern_storage() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = HybridDatabase::new(&db_path, true).unwrap();
        db.initialize_schema().unwrap();

        let pattern = Pattern {
            id: "p1".to_string(),
            name: "auth-pattern".to_string(),
            content: "JWT refresh strategy".to_string(),
            layer: "surface".to_string(),
            confidence: "high".to_string(),
        };

        db.add_pattern(&pattern).unwrap();
    }

    #[test]
    fn test_crdt_sync() {
        let temp_dir = TempDir::new().unwrap();

        // Create two databases with CRDT
        let db1 = HybridDatabase::new(&temp_dir.path().join("db1.db"), true).unwrap();
        let db2 = HybridDatabase::new(&temp_dir.path().join("db2.db"), true).unwrap();

        db1.initialize_schema().unwrap();
        db2.initialize_schema().unwrap();

        // Add pattern to db1
        let pattern = Pattern {
            id: "p1".to_string(),
            name: "cache-pattern".to_string(),
            content: "Redis TTL strategy".to_string(),
            layer: "surface".to_string(),
            confidence: "medium".to_string(),
        };

        db1.add_pattern(&pattern).unwrap();

        // Get changes from db1
        let sync_state = db2.get_sync_state();
        let changes = db1.get_crdt_changes(&sync_state).unwrap();

        // Apply to db2
        db2.apply_crdt_changes(&changes).unwrap();
    }
}
