use anyhow::{Context, Result};
use rqlite_rs::{prelude::*, query};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{DocumentInfo, Layer};

/// rqlite client wrapper using rqlite-rs
pub struct RqliteClient {
    client: rqlite_rs::RqliteClient,
}

/// Document record for database queries
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DocumentRecord {
    pub id: String,
    pub path: String,
    pub layer: String,
    pub title: String,
    pub summary: String,
    pub metadata: String, // JSON as string
}

/// Concept record for database queries
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ConceptRecord {
    pub concept: String,
    pub document_id: String,
    pub relevance: String,
    pub confidence: f64,
}

impl RqliteClient {
    /// Create a new rqlite client
    pub async fn new(url: &str) -> Result<Self> {
        // Parse URL to extract host
        let host = url
            .trim_start_matches("http://")
            .trim_start_matches("https://");

        // Create client with single host and no retries for now
        let client = RqliteClientBuilder::new()
            .known_host(host)
            .build()
            .context("Failed to create rqlite client")?;

        Ok(Self { client })
    }

    /// Initialize the database schema
    pub async fn initialize_schema(&self) -> Result<()> {
        // Create documents table
        self.client
            .exec(
                "CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                layer TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                last_indexed TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            )
            .await
            .context("Failed to create documents table")?;

        // Create concepts table
        self.client
            .exec(
                "CREATE TABLE IF NOT EXISTS concepts (
                concept TEXT NOT NULL,
                document_id TEXT NOT NULL,
                relevance TEXT NOT NULL DEFAULT '',
                confidence REAL NOT NULL DEFAULT 1.0,
                PRIMARY KEY (concept, document_id),
                FOREIGN KEY (document_id) REFERENCES documents(id)
            )",
            )
            .await
            .context("Failed to create concepts table")?;

        // Create relationships table
        self.client
            .exec(
                "CREATE TABLE IF NOT EXISTS relationships (
                from_doc TEXT NOT NULL,
                to_doc TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                metadata TEXT DEFAULT '{}',
                PRIMARY KEY (from_doc, to_doc, relationship_type),
                FOREIGN KEY (from_doc) REFERENCES documents(id),
                FOREIGN KEY (to_doc) REFERENCES documents(id)
            )",
            )
            .await
            .context("Failed to create relationships table")?;

        // Create git_states table
        self.client
            .exec(
                "CREATE TABLE IF NOT EXISTS git_states (
                document_id TEXT NOT NULL,
                workspace_id TEXT,
                state TEXT NOT NULL,
                confidence_modifier REAL DEFAULT 1.0,
                metadata TEXT DEFAULT '{}',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (document_id, workspace_id),
                FOREIGN KEY (document_id) REFERENCES documents(id)
            )",
            )
            .await
            .context("Failed to create git_states table")?;

        // Create state_transitions table
        self.client
            .exec(
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
            )
            .await
            .context("Failed to create state_transitions table")?;

        // Create indexes
        let indexes = vec![
            "CREATE INDEX IF NOT EXISTS idx_concepts_concept ON concepts(concept)",
            "CREATE INDEX IF NOT EXISTS idx_concepts_doc ON concepts(document_id)",
            "CREATE INDEX IF NOT EXISTS idx_documents_layer ON documents(layer)",
            "CREATE INDEX IF NOT EXISTS idx_git_states_workspace ON git_states(workspace_id)",
            "CREATE INDEX IF NOT EXISTS idx_git_states_document ON git_states(document_id)",
        ];

        for index_sql in indexes {
            self.client
                .exec(index_sql)
                .await
                .with_context(|| format!("Failed to create index: {index_sql}"))?;
        }

        Ok(())
    }

    /// Insert or update a document
    pub async fn insert_document(&self, info: &DocumentInfo) -> Result<()> {
        let metadata_json = serde_json::to_string(&info.metadata)?;
        let layer_str = match info.layer {
            Layer::Core => "core",
            Layer::Surface => "surface",
            Layer::Dust => "dust",
        };

        // Use parameterized query for safety
        let query = query!(
            "INSERT OR REPLACE INTO documents (id, path, layer, title, summary, metadata)
             VALUES (?, ?, ?, ?, ?, ?)",
            info.id.clone(),
            info.path.to_string_lossy().to_string(),
            layer_str,
            info.title.clone(),
            info.summary.clone(),
            metadata_json
        )?;

        self.client
            .exec(query)
            .await
            .context("Failed to insert document")?;

        // Insert concepts
        for concept in &info.concepts {
            self.insert_concept(concept, &info.id, "", 1.0).await?;
        }

        Ok(())
    }

    /// Insert or update a concept
    pub async fn insert_concept(
        &self,
        concept: &str,
        document_id: &str,
        relevance: &str,
        confidence: f64,
    ) -> Result<()> {
        let query = query!(
            "INSERT OR REPLACE INTO concepts (concept, document_id, relevance, confidence)
             VALUES (?, ?, ?, ?)",
            concept.to_string(),
            document_id.to_string(),
            relevance.to_string(),
            confidence
        )?;

        self.client
            .exec(query)
            .await
            .context("Failed to insert concept")?;

        Ok(())
    }

    /// Load all documents
    pub async fn load_all_documents(&self) -> Result<Vec<DocumentRecord>> {
        let query = query!("SELECT * FROM documents ORDER BY layer, id")?;

        let rows = self
            .client
            .fetch(query)
            .await
            .context("Failed to load documents")?;

        let documents: Vec<DocumentRecord> = rows
            .into_typed()
            .context("Failed to parse document records")?;

        Ok(documents)
    }

    /// Load all concepts
    pub async fn load_all_concepts(&self) -> Result<Vec<ConceptRecord>> {
        let query = query!("SELECT * FROM concepts ORDER BY concept, document_id")?;

        let rows = self
            .client
            .fetch(query)
            .await
            .context("Failed to load concepts")?;

        let concepts: Vec<ConceptRecord> = rows
            .into_typed()
            .context("Failed to parse concept records")?;

        Ok(concepts)
    }

    /// Find documents by concept
    pub async fn find_documents_by_concept(&self, concept: &str) -> Result<Vec<DocumentRecord>> {
        let query = query!(
            "SELECT d.* FROM documents d
             JOIN concepts c ON d.id = c.document_id
             WHERE c.concept = ?
             ORDER BY c.confidence DESC, d.layer",
            concept.to_string()
        )?;

        let rows = self.client.fetch(query).await?;
        let documents: Vec<DocumentRecord> = rows.into_typed()?;

        Ok(documents)
    }

    /// Update git state for a document
    pub async fn update_git_state(
        &self,
        document_id: &str,
        workspace_id: Option<&str>,
        state: &str,
        confidence_modifier: f64,
    ) -> Result<()> {
        let workspace = workspace_id.unwrap_or("").to_string();
        let metadata = "{}".to_string(); // Default empty JSON

        let query = query!(
            "INSERT OR REPLACE INTO git_states 
             (document_id, workspace_id, state, confidence_modifier, metadata)
             VALUES (?, ?, ?, ?, ?)",
            document_id.to_string(),
            workspace,
            state.to_string(),
            confidence_modifier,
            metadata
        )?;

        self.client
            .exec(query)
            .await
            .context("Failed to update git state")?;

        Ok(())
    }

    /// Record a state transition
    pub async fn record_state_transition(
        &self,
        workspace_id: &str,
        document_id: Option<&str>,
        from_state: Option<&str>,
        to_state: &str,
        reason: Option<&str>,
    ) -> Result<()> {
        let doc_id = document_id.unwrap_or("").to_string();
        let from = from_state.unwrap_or("").to_string();
        let reason_str = reason.unwrap_or("").to_string();
        let metadata = "{}".to_string(); // Default empty JSON

        let query = query!(
            "INSERT INTO state_transitions 
             (workspace_id, document_id, from_state, to_state, transition_reason, metadata)
             VALUES (?, ?, ?, ?, ?, ?)",
            workspace_id.to_string(),
            doc_id,
            from,
            to_state.to_string(),
            reason_str,
            metadata
        )?;

        self.client
            .exec(query)
            .await
            .context("Failed to record state transition")?;

        Ok(())
    }

    /// Load cache data (documents and concept mappings)
    pub async fn load_cache_data(
        &self,
    ) -> Result<(
        Vec<DocumentInfo>,
        HashMap<String, Vec<(String, String, f64)>>,
    )> {
        // Load all documents
        let doc_records = self.load_all_documents().await?;

        let mut documents = Vec::new();
        let mut concept_map: HashMap<String, Vec<(String, String, f64)>> = HashMap::new();

        // Convert records to DocumentInfo
        for record in doc_records {
            // Parse layer
            let layer = match record.layer.as_str() {
                "core" => Layer::Core,
                "surface" => Layer::Surface,
                "dust" => Layer::Dust,
                _ => Layer::Surface,
            };

            // Parse metadata
            let metadata: HashMap<String, String> =
                serde_json::from_str(&record.metadata).unwrap_or_default();

            // Load concepts for this document
            let concept_query = query!(
                "SELECT concept FROM concepts WHERE document_id = ?",
                record.id.clone()
            )?;

            let concept_rows = self.client.fetch(concept_query).await?;
            let concept_results: Vec<(String,)> = concept_rows.into_typed()?;
            let concepts: Vec<String> = concept_results.into_iter().map(|(c,)| c).collect();

            documents.push(DocumentInfo {
                id: record.id,
                path: record.path.into(),
                layer,
                title: record.title,
                summary: record.summary,
                concepts,
                metadata,
            });
        }

        // Load all concept mappings
        let concept_records = self.load_all_concepts().await?;

        for record in concept_records {
            let entry = concept_map.entry(record.concept).or_default();
            entry.push((record.document_id, record.relevance, record.confidence));
        }

        Ok((documents, concept_map))
    }
}
