//! Navigation state management with git awareness

use super::{Confidence, GitState, Layer, Location, NavigationResponse, WorkspaceHint};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Git-aware navigation map for fast pattern discovery
pub struct GitAwareNavigationMap {
    /// Concept to location mapping
    concepts: HashMap<String, Vec<Location>>,

    /// Document metadata cache
    documents: HashMap<String, DocumentInfo>,

    /// Git state tracking per file
    git_states: HashMap<PathBuf, GitState>,

    /// Active workspace states
    workspace_states: HashMap<String, WorkspaceNavigationState>,

    /// Performance tracking
    last_refresh: Instant,
    cache_generation: u64,
}

/// Information about a tracked document
#[derive(Debug, Clone)]
pub struct DocumentInfo {
    pub id: String,
    pub path: PathBuf,
    pub layer: Layer,
    pub title: String,
    pub summary: String,
    pub concepts: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Navigation state for a workspace
#[derive(Debug, Clone)]
pub struct WorkspaceNavigationState {
    pub workspace_id: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub git_state: GitState,
    pub navigation_confidence: Confidence,
    pub discovered_patterns: Vec<DiscoveredPattern>,
    pub last_activity: Instant,
}

/// A pattern discovered in a workspace
#[derive(Debug, Clone)]
pub struct DiscoveredPattern {
    pub name: String,
    pub pattern_type: String,
    pub file_path: PathBuf,
    pub confidence: f32,
}

impl Default for GitAwareNavigationMap {
    fn default() -> Self {
        Self::new()
    }
}

impl GitAwareNavigationMap {
    /// Create a new navigation map
    pub fn new() -> Self {
        Self {
            concepts: HashMap::new(),
            documents: HashMap::new(),
            git_states: HashMap::new(),
            workspace_states: HashMap::new(),
            last_refresh: Instant::now(),
            cache_generation: 0,
        }
    }

    /// Navigate to find patterns matching a query
    pub fn navigate(&self, query: &str) -> NavigationResponse {
        use std::collections::HashSet;
        let mut locations = Vec::new();
        let mut seen_paths = HashSet::new();

        // Find concepts matching the query
        let concepts = self.extract_concepts(query);

        for concept in &concepts {
            if let Some(concept_locations) = self.concepts.get(concept) {
                for loc in concept_locations {
                    // Skip if we've already added this path
                    if !seen_paths.insert(loc.path.clone()) {
                        continue;
                    }

                    let mut location = loc.clone();

                    // Enrich with git state
                    if let Some(git_state) = self.git_states.get(&location.path) {
                        location.confidence =
                            self.calculate_confidence(location.confidence, git_state);
                        location.git_state = Some(git_state.clone());
                    }

                    locations.push(location);
                }
            }
        }

        // Sort by layer and confidence
        locations.sort_by(|a, b| a.layer.cmp(&b.layer).then(b.confidence.cmp(&a.confidence)));

        // Build workspace hints
        let workspace_hints = self.build_workspace_hints(query);

        NavigationResponse {
            query: query.to_string(),
            locations,
            workspace_hints,
            confidence_explanation: self.explain_confidence(),
        }
    }

    /// Insert or update a document in the navigation map
    pub fn insert_document(&mut self, info: DocumentInfo) {
        // Update concept mappings
        for concept in &info.concepts {
            let location = Location {
                layer: info.layer,
                path: info.path.clone(),
                relevance: format!("Defines {concept}"),
                confidence: Confidence::Medium,
                git_state: None,
            };

            self.concepts
                .entry(concept.clone())
                .or_default()
                .push(location);
        }

        // Store document info
        self.documents.insert(info.id.clone(), info);

        // Bump cache generation
        self.cache_generation += 1;
    }

    /// Update git state for a document
    pub fn update_git_state(&mut self, path: &Path, state: GitState) {
        self.git_states.insert(path.to_path_buf(), state);
        self.cache_generation += 1;
    }

    /// Add or update a workspace state
    pub fn update_workspace_state(&mut self, state: WorkspaceNavigationState) {
        self.workspace_states
            .insert(state.workspace_id.clone(), state);
    }

    /// Extract concepts from a query
    fn extract_concepts(&self, query: &str) -> Vec<String> {
        // Simple tokenization for now
        query
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .filter(|w| {
                !matches!(
                    *w,
                    "how" | "what" | "where" | "when" | "why" | "the" | "and" | "for"
                )
            })
            .map(String::from)
            .collect()
    }

    /// Calculate confidence based on git state
    fn calculate_confidence(&self, _base: Confidence, git_state: &GitState) -> Confidence {
        // Use the git state's inherent confidence
        git_state.confidence()
    }

    /// Build workspace hints for active explorations
    fn build_workspace_hints(&self, query: &str) -> Vec<WorkspaceHint> {
        let mut hints = Vec::new();

        for (ws_id, state) in &self.workspace_states {
            // Check if workspace is relevant to query
            let is_relevant = state
                .discovered_patterns
                .iter()
                .any(|p| p.name.to_lowercase().contains(&query.to_lowercase()));

            if is_relevant {
                hints.push(WorkspaceHint {
                    workspace_id: ws_id.clone(),
                    branch: state.branch.clone(),
                    relevance: format!("Active exploration of {query}"),
                });
            }
        }

        hints
    }

    /// Explain how confidence scoring works
    fn explain_confidence(&self) -> String {
        "Confidence scoring: Verified (merged to main) > High (pushed/PR) > Medium (committed) > Low (modified) > Experimental (untracked)".to_string()
    }

    /// Get statistics about the navigation map
    pub fn stats(&self) -> NavigationStats {
        NavigationStats {
            total_concepts: self.concepts.len(),
            total_documents: self.documents.len(),
            total_git_states: self.git_states.len(),
            active_workspaces: self.workspace_states.len(),
            cache_generation: self.cache_generation,
            last_refresh: self.last_refresh,
        }
    }
}

/// Statistics about the navigation map
#[derive(Debug)]
pub struct NavigationStats {
    pub total_concepts: usize,
    pub total_documents: usize,
    pub total_git_states: usize,
    pub active_workspaces: usize,
    pub cache_generation: u64,
    pub last_refresh: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_map_creation() {
        let map = GitAwareNavigationMap::new();
        let stats = map.stats();
        assert_eq!(stats.total_concepts, 0);
        assert_eq!(stats.total_documents, 0);
    }

    #[test]
    fn test_document_insertion() {
        let mut map = GitAwareNavigationMap::new();

        let doc = DocumentInfo {
            id: "test-doc".to_string(),
            path: PathBuf::from("test.md"),
            layer: Layer::Core,
            title: "Test Document".to_string(),
            summary: "A test document".to_string(),
            concepts: vec!["testing".to_string(), "example".to_string()],
            metadata: HashMap::new(),
        };

        map.insert_document(doc);

        let stats = map.stats();
        assert_eq!(stats.total_documents, 1);
        assert_eq!(stats.total_concepts, 2);
    }

    #[test]
    fn test_navigation_query() {
        let mut map = GitAwareNavigationMap::new();

        // Insert a test document
        let doc = DocumentInfo {
            id: "auth-doc".to_string(),
            path: PathBuf::from("auth.md"),
            layer: Layer::Core,
            title: "Authentication".to_string(),
            summary: "JWT authentication pattern".to_string(),
            concepts: vec!["jwt".to_string(), "authentication".to_string()],
            metadata: HashMap::new(),
        };

        map.insert_document(doc);

        // Query for it
        let response = map.navigate("how to implement JWT authentication");
        assert_eq!(response.locations.len(), 2); // Should find both concepts
        assert_eq!(response.query, "how to implement JWT authentication");
    }
}
