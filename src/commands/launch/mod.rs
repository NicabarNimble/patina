//! Launch command - Open project in AI interface
//!
//! The launcher is how you open AI-assisted development sessions.

pub(crate) mod internal;

use anyhow::Result;

/// Launch options
#[derive(Debug, Clone)]
pub struct LaunchOptions {
    /// Path to project (default: current directory)
    pub path: Option<String>,
    /// Interface to use (default: from config)
    pub interface: Option<String>,
    /// Start mother in background if not running
    #[allow(dead_code)]
    pub auto_start_mother: bool,
    /// Initialize project if needed (prompt user)
    pub auto_init: bool,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            path: None,
            interface: None,
            auto_start_mother: true,
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
        assert!(opts.interface.is_none());
        assert!(opts.auto_start_mother);
        assert!(opts.auto_init);
    }
}
