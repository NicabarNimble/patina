pub mod brain;
pub mod scaffold;
pub mod session;
pub mod environment;
pub mod adapters;

// Re-export commonly used types
pub use brain::{Brain, Pattern, PatternType};
pub use scaffold::Scaffold;
pub use session::{Session, SessionManager, SessionPattern};
pub use environment::Environment;