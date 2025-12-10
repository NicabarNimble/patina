//! Launch command - Open project in AI frontend
//!
//! The launcher is how you open AI-assisted development sessions.
//! Like `code .` for VS Code, but for AI frontends.
//!
//! # Usage
//!
//! ```bash
//! patina launch              # Open in default frontend
//! patina launch claude       # Open in Claude Code
//! patina launch gemini       # Open in Gemini CLI
//! patina launch . claude     # Explicit current dir + frontend
//! patina launch ~/project    # Different project, default frontend
//! ```

mod internal;

use anyhow::Result;

/// Launch options
#[derive(Debug, Clone)]
pub struct LaunchOptions {
    /// Path to project (default: current directory)
    pub path: Option<String>,
    /// Frontend to use (default: from config)
    pub frontend: Option<String>,
    /// Start mothership in background if not running
    pub auto_start_mothership: bool,
    /// Initialize project if needed (prompt user)
    pub auto_init: bool,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            path: None,
            frontend: None,
            auto_start_mothership: true,
            auto_init: true,
        }
    }
}

/// Execute the launch command
pub fn execute(options: LaunchOptions) -> Result<()> {
    internal::launch(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let opts = LaunchOptions::default();
        assert!(opts.path.is_none());
        assert!(opts.frontend.is_none());
        assert!(opts.auto_start_mothership);
        assert!(opts.auto_init);
    }
}
