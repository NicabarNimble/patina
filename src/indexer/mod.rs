//! Pattern indexing and navigation for the Patina knowledge system.
//!
//! This module provides semantic search and navigation across the layer hierarchy,
//! with git-aware confidence scoring and workspace state tracking.
//!
//! # Example
//! ```no_run
//! use patina::indexer::{PatternIndexer, Layer};
//!
//! let indexer = PatternIndexer::new()?;
//! indexer.index_directory(&layer_path)?;
//! let results = indexer.navigate("session management");
//!
//! // Filter by layer
//! let core_results = results.locations.iter()
//!     .filter(|loc| loc.layer == Layer::Core)
//!     .collect::<Vec<_>>();
//! ```

mod internal;

// Core public API - minimal surface
pub use internal::{
    // Main indexer
    PatternIndexer,
    
    // Response types
    NavigationResponse, Location, Layer, Confidence, WorkspaceHint,
    
    // Git state (needed for confidence scoring)
    GitState,
};

// Advanced API - for specialized use cases
pub mod advanced {
    pub use super::internal::{
        // Database types
        HybridDatabase, NavigationCRDT, Pattern, WorkspaceState,
        // Navigation internals
        DocumentInfo, GitAwareNavigationMap, WorkspaceNavigationState,
        // Database client
        SqliteClient,
        // State machine
        GitNavigationStateMachine,
        // Git modules
        git_detection, git_state,
    };
}