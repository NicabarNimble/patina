pub mod adapters;
pub mod dev_env;
pub mod environment;
pub mod indexer;
pub mod indexer_refactored;
pub mod layer;
pub mod session;
pub mod version;
pub mod workspace_client;
pub mod workspace_client_refactored;

// Re-export commonly used types
pub use environment::Environment;
pub use layer::{Layer, Pattern, PatternType};
pub use session::SessionManager;
