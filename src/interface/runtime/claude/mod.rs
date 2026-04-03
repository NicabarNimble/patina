//! Claude interface for Patina
//!
//! Provides Claude-specific integration including:
//! - Session management scripts
//! - Context file generation (.claude/CLAUDE.md)
//! - Custom command definitions

use crate::environment::Environment;
use anyhow::Result;
use std::path::{Path, PathBuf};

mod internal;
use super::InterfaceProvider;

// Re-export version constant for version checking
pub use internal::CLAUDE_INTERFACE_VERSION;

/// Claude interface implementation
pub struct ClaudeInterface;

impl ClaudeInterface {
    /// Create a new Claude interface
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClaudeInterface {
    fn default() -> Self {
        Self::new()
    }
}

impl InterfaceProvider for ClaudeInterface {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn init_project(
        &self,
        project_path: &Path,
        project_name: &str,
        environment: &Environment,
    ) -> Result<()> {
        internal::init_project(project_path, project_name, environment)
    }

    fn post_init(&self, project_path: &Path) -> Result<()> {
        internal::post_init(project_path)
    }

    fn get_custom_commands(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "/session-start [name]",
                "Start session with Git branch creation",
            ),
            ("/session-update", "Update session with Git awareness"),
            ("/session-note [insight]", "Add insight with Git context"),
            ("/session-end", "End session with Git classification"),
            ("/patina-review", "Review recent sessions and git history"),
        ]
    }

    fn get_context_file_path(&self, project_path: &Path) -> PathBuf {
        internal::get_context_file_path(project_path)
    }

    fn check_for_updates(&self, project_path: &Path) -> Result<Option<(String, String)>> {
        internal::check_for_updates(project_path)
    }

    fn update_interface_files(&self, project_path: &Path) -> Result<()> {
        internal::update_interface_files(project_path)
    }

    fn get_sessions_path(&self, project_path: &Path) -> Option<PathBuf> {
        Some(internal::get_sessions_path(project_path))
    }

    fn version(&self) -> &'static str {
        internal::CLAUDE_INTERFACE_VERSION
    }

    fn get_version_changes(&self, version: &str) -> Option<Vec<String>> {
        internal::get_version_changes(version)
    }

    fn get_changelog_since(&self, from_version: &str) -> Vec<String> {
        internal::get_changelog_since(from_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_name() {
        let iface = ClaudeInterface::new();
        assert_eq!(iface.name(), "claude");
    }

    #[test]
    fn test_custom_commands() {
        let iface = ClaudeInterface::new();
        let commands = iface.get_custom_commands();
        assert_eq!(commands.len(), 5);
        assert!(commands.iter().any(|(cmd, _)| cmd.starts_with("/session-")));
        assert!(commands
            .iter()
            .any(|(cmd, _)| cmd.starts_with("/patina-review")));
    }
}
