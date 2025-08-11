pub mod adapters;
pub mod config;
pub mod dev_env;
pub mod environment;
pub mod indexer;
pub mod layer;
pub mod session;
pub mod version;
pub mod workspace_client;

// Re-export commonly used types
pub use environment::Environment;
pub use layer::{Layer, Pattern, PatternType};
pub use session::SessionManager;
