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
    Confidence,
    // Git state (needed for confidence scoring)
    GitState,
    Layer,
    Location,
    // Response types
    NavigationResponse,
    // Main indexer
    PatternIndexer,

    WorkspaceHint,
};

// Advanced API - for specialized use cases
pub mod advanced {
    pub use super::internal::{
        // Git modules
        git_detection,
        git_state,
        // Navigation internals
        DocumentInfo,
        GitAwareNavigationMap,
        // State machine
        GitNavigationStateMachine,
        // Database types
        HybridDatabase,
        NavigationCRDT,
        Pattern,
        // Database client
        SqliteClient,
        WorkspaceNavigationState,
        WorkspaceState,
    };
}
