//! Pattern indexer for navigation and discovery
//!
//! This module implements the git-aware navigation system that tracks
//! pattern evolution through git states and provides confidence-based
//! search results.

pub mod database;
pub mod git_detection;
pub mod git_state;
pub mod monitoring;
pub mod navigation_state;
pub mod state_machine;

pub use database::RqliteClient;
pub use git_state::{Confidence, GitEvent, GitState};
pub use monitoring::WorkspaceMonitor;
pub use navigation_state::{DocumentInfo, GitAwareNavigationMap, WorkspaceNavigationState};
pub use state_machine::GitNavigationStateMachine;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main pattern indexer that coordinates navigation
pub struct PatternIndexer {
    /// In-memory navigation cache for fast queries
    cache: Arc<RwLock<GitAwareNavigationMap>>,
    /// Database client for persistence
    db: Option<RqliteClient>,
    /// Git state machine for tracking changes
    state_machine: Arc<RwLock<GitNavigationStateMachine>>,
}

impl PatternIndexer {
    /// Create a new pattern indexer without database
    pub fn new() -> Result<Self> {
        let cache = Arc::new(RwLock::new(GitAwareNavigationMap::new()));
        let state_machine = Arc::new(RwLock::new(GitNavigationStateMachine::new()?));

        Ok(Self {
            cache,
            db: None,
            state_machine,
        })
    }

    /// Create a new pattern indexer with database connection
    pub async fn with_database(db_url: &str) -> Result<Self> {
        let db = RqliteClient::new(db_url).await?;

        // Initialize schema
        db.initialize_schema()
            .await
            .context("Failed to initialize database schema")?;

        // Load existing data into cache
        let mut cache = GitAwareNavigationMap::new();
        let state_machine = Arc::new(RwLock::new(GitNavigationStateMachine::new()?));

        // Load documents and concepts from database
        let (documents, _concept_mappings) = db
            .load_cache_data()
            .await
            .context("Failed to load data from database")?;

        // Populate cache
        for doc in documents {
            cache.insert_document(doc);
        }

        let cache = Arc::new(RwLock::new(cache));

        Ok(Self {
            cache,
            db: Some(db),
            state_machine,
        })
    }

    /// Navigate to find patterns matching a query (memory-first)
    pub async fn navigate(&self, query: &str) -> NavigationResponse {
        let cache = self.cache.read().await;
        let mut response = cache.navigate(query);

        // Enrich with git states
        let state_machine = self.state_machine.read().await;
        for location in &mut response.locations {
            if let Some(git_state) = state_machine.get_git_state(&location.path) {
                location.git_state = Some(git_state.clone());
                location.confidence =
                    self.calculate_git_confidence(location.confidence, git_state);
            }
        }

        // Add confidence explanation
        response.confidence_explanation = self.explain_confidence_scoring();

        response
    }

    /// Index a document for navigation
    pub async fn index_document(&self, path: &Path) -> Result<()> {
        // 1. Analyze document
        let doc_info = self.analyze_document(path).await?;

        // 2. Update memory cache
        {
            let mut cache = self.cache.write().await;
            cache.insert_document(doc_info.clone());
        }

        // 3. Persist to database if available
        if let Some(db) = &self.db {
            db.insert_document(&doc_info)
                .await
                .context("Failed to persist document to database")?;
        }

        // 4. Update git state machine
        {
            let mut state_machine = self.state_machine.write().await;
            state_machine.track_document(&doc_info.path);
        }

        Ok(())
    }

    /// Analyze a document to extract metadata and concepts
    async fn analyze_document(&self, path: &Path) -> Result<DocumentInfo> {
        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read document")?;

        // Parse YAML frontmatter if present
        let (metadata, body) = self.parse_frontmatter(&content);

        // Extract concepts from content
        let concepts = self.extract_concepts(&body);

        // Determine layer from path
        let layer = self.determine_layer(path);

        // Extract title and summary
        let title = metadata
            .get("id")
            .or_else(|| metadata.get("title"))
            .cloned()
            .unwrap_or_else(|| {
                path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default()
            });

        let summary = metadata
            .get("summary")
            .cloned()
            .or_else(|| self.extract_summary(&body))
            .unwrap_or_default();

        Ok(DocumentInfo {
            id: self.generate_document_id(path),
            path: path.to_path_buf(),
            layer,
            title,
            summary,
            concepts,
            metadata,
        })
    }

    /// Parse YAML frontmatter from markdown
    fn parse_frontmatter(
        &self,
        content: &str,
    ) -> (std::collections::HashMap<String, String>, String) {
        let mut metadata = std::collections::HashMap::new();
        let mut body = content;

        if let Some(stripped) = content.strip_prefix("---\n") {
            if let Some(end) = stripped.find("\n---\n") {
                let yaml_content = &stripped[..end];
                body = &content[end + 9..];

                // Simple YAML parsing for key-value pairs
                for line in yaml_content.lines() {
                    if let Some((key, value)) = line.split_once(':') {
                        let key = key.trim().to_string();
                        let value = value.trim().trim_matches('"').to_string();
                        metadata.insert(key, value);
                    }
                }
            }
        }

        (metadata, body.to_string())
    }

    /// Extract concepts from document content
    fn extract_concepts(&self, content: &str) -> Vec<String> {
        let mut concepts = Vec::new();

        // Extract from headings
        for line in content.lines() {
            if let Some(stripped) = line.strip_prefix("# ") {
                let heading = stripped.trim();
                concepts.push(heading.to_lowercase());
                // Also add individual words from heading
                for word in heading.split_whitespace() {
                    if word.len() > 3 {
                        concepts.push(word.to_lowercase());
                    }
                }
            } else if let Some(stripped) = line.strip_prefix("## ") {
                let heading = stripped.trim();
                concepts.push(heading.to_lowercase());
                // Also add individual words from subheading
                for word in heading.split_whitespace() {
                    if word.len() > 3 {
                        concepts.push(word.to_lowercase());
                    }
                }
            }
        }

        // Extract from tags if present
        if let Some(tag_line) = content.lines().find(|l| l.starts_with("tags:")) {
            let tags = tag_line[5..]
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']');
            for tag in tags.split(',') {
                concepts.push(tag.trim().to_lowercase());
            }
        }

        // Extract from id field in frontmatter
        if let Some(id_line) = content.lines().find(|l| l.starts_with("id:")) {
            let id = id_line[3..].trim();
            concepts.push(id.to_lowercase());
            // Also split on hyphens
            for part in id.split('-') {
                if part.len() > 3 {
                    concepts.push(part.to_lowercase());
                }
            }
        }

        concepts.dedup();
        concepts
    }

    /// Extract summary from content
    fn extract_summary(&self, content: &str) -> Option<String> {
        // Take first paragraph after any headings
        let mut in_summary = false;
        let mut summary = String::new();

        for line in content.lines() {
            if line.starts_with('#') {
                in_summary = true;
                continue;
            }

            if in_summary && !line.trim().is_empty() {
                summary.push_str(line);
                summary.push(' ');

                if summary.len() > 200 {
                    break;
                }
            } else if in_summary && line.trim().is_empty() && !summary.is_empty() {
                break;
            }
        }

        if summary.is_empty() {
            None
        } else {
            Some(summary.trim().to_string())
        }
    }

    /// Determine layer from path
    fn determine_layer(&self, path: &Path) -> Layer {
        let path_str = path.to_string_lossy();

        if path_str.contains("/core/") {
            Layer::Core
        } else if path_str.contains("/dust/") || path_str.contains("/archived/") {
            Layer::Dust
        } else {
            Layer::Surface
        }
    }

    /// Generate unique document ID
    fn generate_document_id(&self, path: &Path) -> String {
        // Use path hash for deterministic IDs
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        format!("doc-{:x}", hasher.finish())
    }

    /// Calculate confidence based on git state
    fn calculate_git_confidence(&self, base: Confidence, git_state: &GitState) -> Confidence {
        match git_state {
            GitState::Merged { .. } => Confidence::Verified,
            GitState::Pushed { .. } => Confidence::High,
            GitState::Committed { .. } => Confidence::Medium,
            GitState::Staged { .. } => Confidence::Low,
            GitState::Modified { .. } => Confidence::Low,
            GitState::Untracked { .. } => Confidence::Experimental,
            _ => base,
        }
    }

    /// Explain confidence scoring
    fn explain_confidence_scoring(&self) -> String {
        "Confidence scores reflect git state: \
         Merged=Verified, Pushed=High, Committed=Medium, \
         Staged/Modified=Low, Untracked=Experimental"
            .to_string()
    }

    /// Process git event from workspace
    pub async fn process_git_event(&self, event: GitEvent) -> Result<()> {
        let mut state_machine = self.state_machine.write().await;
        state_machine.process_git_event(event).await?;

        // Update cache if needed
        // TODO: Implement cache updates based on git events

        Ok(())
    }
}

/// Response from a navigation query
#[derive(Debug, Clone)]
pub struct NavigationResponse {
    pub query: String,
    pub locations: Vec<Location>,
    pub workspace_hints: Vec<WorkspaceHint>,
    pub confidence_explanation: String,
}

/// A location in the pattern hierarchy
#[derive(Debug, Clone)]
pub struct Location {
    pub layer: Layer,
    pub path: PathBuf,
    pub relevance: String,
    pub confidence: Confidence,
    pub git_state: Option<GitState>,
}

/// Pattern storage layers
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Layer {
    Core,
    Surface,
    Dust,
}

/// Hint about an active workspace exploration
#[derive(Debug, Clone)]
pub struct WorkspaceHint {
    pub workspace_id: String,
    pub branch: String,
    pub relevance: String,
}
