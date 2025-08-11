//! Pattern indexing and navigation for the Patina knowledge system.
//!
//! This module provides semantic search and navigation across the layer hierarchy,
//! with git-aware confidence scoring and workspace state tracking.
//!
//! # Example
//! ```no_run
//! use patina::indexer::PatternIndexer;
//!
//! let indexer = PatternIndexer::new()?;
//! indexer.index_directory(&layer_path)?;
//! let results = indexer.navigate("session management");
//! ```

mod internal;

// Re-export only the public API
pub use internal::{
    // Core types
    Confidence, GitState, Layer, Location, NavigationResponse, PatternIndexer, WorkspaceHint,
    // Database types (for now, will minimize later)
    HybridDatabase, NavigationCRDT, Pattern, WorkspaceState,
    // Navigation types
    DocumentInfo, GitAwareNavigationMap, WorkspaceNavigationState,
    // Database client
    SqliteClient,
    // State machine
    GitNavigationStateMachine,
    // Git detection
    git_detection, git_state,
};