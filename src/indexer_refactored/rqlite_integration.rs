//! rqlite integration for decentralized pattern sharing
//! 
//! This module provides a proper integration with rqlite-rs that supports
//! Patina's vision of decentralized knowledge sharing ("Napster not Spotify").

use anyhow::{Context, Result};
use rqlite_rs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{DocumentInfo, Layer};

/// Configuration for rqlite connection
#[derive(Debug, Clone)]
pub struct RqliteConfig {
    /// Primary node address (e.g., "localhost:4001")
    pub primary_host: String,
    /// Additional peer nodes for resilience
    pub peer_hosts: Vec<String>,
    /// Enable automatic peer discovery
    pub auto_discover: bool,
}

impl Default for RqliteConfig {
    fn default() -> Self {
        Self {
            primary_host: "localhost:4001".to_string(),
            peer_hosts: vec![],
            auto_discover: false,
        }
    }
}

/// rqlite client wrapper with decentralized features
#[derive(Clone)]
pub struct DecentralizedDB {
    client: RqliteClient,
    config: RqliteConfig,
}

/// Document record for database storage
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DocumentRecord {
    pub id: String,
    pub path: String,
    pub layer: String,
    pub title: String,
    pub summary: String,
    pub metadata: String, // JSON as string
}

/// Concept record for database storage
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ConceptRecord {
    pub concept: String,
    pub document_id: String,
    pub relevance: String,
    pub confidence: f64,
}

impl DecentralizedDB {
    /// Create a new decentralized database connection
    pub async fn new(config: RqliteConfig) -> Result<Self> {
        let mut builder = RqliteClientBuilder::new()
            .known_host(&config.primary_host);
            
        // Add peer hosts for resilience
        for peer in &config.peer_hosts {
            builder = builder.known_host(peer);
        }
        
        let client = builder
            .build()
            .context("Failed to build rqlite client")?;
        
        Ok(Self { client, config })
    }
    
    /// Initialize the pattern database schema
    pub async fn initialize_schema(&self) -> Result<()> {
        // Create tables using transactions for atomicity
        let schema_sql = r#"
            BEGIN;
            
            -- Core document registry
            CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                layer TEXT NOT NULL CHECK(layer IN ('core', 'surface', 'dust')),
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                source_node TEXT DEFAULT NULL,
                last_indexed TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            
            -- Concept mappings for navigation
            CREATE TABLE IF NOT EXISTS concepts (
                concept TEXT NOT NULL,
                document_id TEXT NOT NULL,
                relevance TEXT NOT NULL DEFAULT '',
                confidence REAL NOT NULL DEFAULT 1.0,
                PRIMARY KEY (concept, document_id),
                FOREIGN KEY (document_id) REFERENCES documents(id)
            );
            
            -- Document relationships
            CREATE TABLE IF NOT EXISTS relationships (
                from_doc TEXT NOT NULL,
                to_doc TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                metadata TEXT DEFAULT '{}',
                PRIMARY KEY (from_doc, to_doc, relationship_type),
                FOREIGN KEY (from_doc) REFERENCES documents(id),
                FOREIGN KEY (to_doc) REFERENCES documents(id)
            );
            
            -- Git state tracking
            CREATE TABLE IF NOT EXISTS git_states (
                document_id TEXT NOT NULL,
                workspace_id TEXT,
                state TEXT NOT NULL,
                confidence_modifier REAL DEFAULT 1.0,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (document_id, workspace_id),
                FOREIGN KEY (document_id) REFERENCES documents(id)
            );
            
            -- Pattern sharing metadata
            CREATE TABLE IF NOT EXISTS pattern_shares (
                pattern_id TEXT PRIMARY KEY,
                shared_by TEXT NOT NULL,
                shared_with TEXT,
                share_type TEXT CHECK(share_type IN ('public', 'team', 'private')),
                shared_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            
            -- Indexes for performance
            CREATE INDEX IF NOT EXISTS idx_concepts_concept ON concepts(concept);
            CREATE INDEX IF NOT EXISTS idx_documents_layer ON documents(layer);
            CREATE INDEX IF NOT EXISTS idx_git_states_doc ON git_states(document_id);
            CREATE INDEX IF NOT EXISTS idx_shares_by ON pattern_shares(shared_by);
            
            COMMIT;
        "#;
        
        self.client.exec_sql(schema_sql)
            .await
            .context("Failed to initialize schema")?;
        
        Ok(())
    }
    
    /// Insert or update a document
    pub async fn upsert_document(&self, info: &DocumentInfo) -> Result<()> {
        let metadata_json = serde_json::to_string(&info.metadata)?;
        let layer_str = match info.layer {
            Layer::Core => "core",
            Layer::Surface => "surface",
            Layer::Dust => "dust",
        };
        
        // Use parameterized queries for safety
        let sql = r#"
            INSERT OR REPLACE INTO documents 
            (id, path, layer, title, summary, metadata)
            VALUES (:id, :path, :layer, :title, :summary, :metadata)
        "#;
        
        self.client.exec_with_params(
            sql,
            named_params! {
                ":id": &info.id,
                ":path": info.path.to_string_lossy().as_ref(),
                ":layer": layer_str,
                ":title": &info.title,
                ":summary": &info.summary,
                ":metadata": &metadata_json,
            }
        ).await
        .context("Failed to upsert document")?;
        
        // Update concepts
        for concept in &info.concepts {
            self.upsert_concept(concept, &info.id, "", 1.0).await?;
        }
        
        Ok(())
    }
    
    /// Insert or update a concept mapping
    pub async fn upsert_concept(&self, concept: &str, doc_id: &str, relevance: &str, confidence: f64) -> Result<()> {
        let sql = r#"
            INSERT OR REPLACE INTO concepts 
            (concept, document_id, relevance, confidence)
            VALUES (:concept, :doc_id, :relevance, :confidence)
        "#;
        
        self.client.exec_with_params(
            sql,
            named_params! {
                ":concept": concept,
                ":doc_id": doc_id,
                ":relevance": relevance,
                ":confidence": confidence,
            }
        ).await
        .context("Failed to upsert concept")?;
        
        Ok(())
    }
    
    /// Query documents by concept
    pub async fn find_by_concept(&self, concept: &str) -> Result<Vec<DocumentRecord>> {
        let sql = r#"
            SELECT d.* FROM documents d
            JOIN concepts c ON d.id = c.document_id
            WHERE c.concept = :concept
            ORDER BY c.confidence DESC, d.layer
        "#;
        
        let rows = self.client.fetch_with_params(
            sql,
            named_params! { ":concept": concept }
        ).await?;
        
        let documents = rows.into_typed::<DocumentRecord>()?;
        Ok(documents.collect())
    }
    
    /// Get all documents for cache loading
    pub async fn load_all_documents(&self) -> Result<Vec<DocumentInfo>> {
        let sql = "SELECT * FROM documents ORDER BY layer, id";
        let rows = self.client.fetch_sql(sql).await?;
        let records = rows.into_typed::<DocumentRecord>()?;
        
        let mut documents = Vec::new();
        for record in records {
            // Load concepts for this document
            let concept_sql = "SELECT concept FROM concepts WHERE document_id = :doc_id";
            let concept_rows = self.client.fetch_with_params(
                concept_sql,
                named_params! { ":doc_id": &record.id }
            ).await?;
            
            let concepts: Vec<String> = concept_rows
                .into_typed::<(String,)>()?
                .map(|(c,)| c)
                .collect();
            
            // Parse metadata
            let metadata: HashMap<String, String> = serde_json::from_str(&record.metadata)
                .unwrap_or_default();
            
            // Convert layer string to enum
            let layer = match record.layer.as_str() {
                "core" => Layer::Core,
                "surface" => Layer::Surface,
                "dust" => Layer::Dust,
                _ => Layer::Surface,
            };
            
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
        
        Ok(documents)
    }
    
    /// Share a pattern with peers
    pub async fn share_pattern(&self, pattern_id: &str, share_type: &str, node_id: &str) -> Result<()> {
        let sql = r#"
            INSERT INTO pattern_shares 
            (pattern_id, shared_by, share_type)
            VALUES (:pattern_id, :node_id, :share_type)
        "#;
        
        self.client.exec_with_params(
            sql,
            named_params! {
                ":pattern_id": pattern_id,
                ":node_id": node_id,
                ":share_type": share_type,
            }
        ).await?;
        
        Ok(())
    }
    
    /// Join a peer's cluster to share patterns
    pub async fn join_peer(&self, peer_addr: &str) -> Result<()> {
        // This would use rqlite's HTTP API to join the cluster
        // For now, it's a placeholder for the decentralized vision
        eprintln!("TODO: Implement cluster join via rqlite HTTP API");
        eprintln!("Would join peer at: {}", peer_addr);
        Ok(())
    }
    
    /// Get cluster status
    pub async fn cluster_status(&self) -> Result<serde_json::Value> {
        // This would query rqlite's status endpoint
        // For now, return mock status
        Ok(serde_json::json!({
            "node_id": "patina-local",
            "leader": true,
            "peers": self.config.peer_hosts,
        }))
    }
}