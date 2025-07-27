pub mod brain;
pub mod scaffold;
pub mod session;
pub mod environment;
pub mod adapters;
pub mod version;
pub mod dev_env;

// Re-export commonly used types
pub use brain::{Brain, Pattern, PatternType};
pub use scaffold::Scaffold;
pub use session::{Session, SessionManager, SessionPattern};
pub use environment::Environment;