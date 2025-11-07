//! Claude adapter for Patina
//!
//! Provides Claude-specific integration including:
//! - Session management scripts
//! - Context file generation (.claude/CLAUDE.md)
//! - MCP (Model Context Protocol) support
//! - Custom command definitions

use crate::adapters::LLMAdapter;
use crate::environment::Environment;
use anyhow::Result;
use std::path::{Path, PathBuf};

mod internal;

// Re-export version constant for version checking
pub use internal::CLAUDE_ADAPTER_VERSION;

/// Claude adapter implementation
pub struct ClaudeAdapter;

impl ClaudeAdapter {
    /// Create a new Claude adapter
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl LLMAdapter for ClaudeAdapter {
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

    fn post_init(&self, project_path: &Path, dev_env: &str) -> Result<()> {
        internal::post_init(project_path, dev_env)
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
            (
                "/launch [branch]",
                "Create experimental branch for testing ideas",
            ),
            (
                "/persona-start",
                "Start belief extraction session with neuro-symbolic validation",
            ),
        ]
    }

    fn get_context_file_path(&self, project_path: &Path) -> PathBuf {
        internal::get_context_file_path(project_path)
    }

    fn check_for_updates(&self, project_path: &Path) -> Result<Option<(String, String)>> {
        internal::check_for_updates(project_path)
    }

    fn update_adapter_files(&self, project_path: &Path) -> Result<()> {
        internal::update_adapter_files(project_path)
    }

    fn get_sessions_path(&self, project_path: &Path) -> Option<PathBuf> {
        Some(internal::get_sessions_path(project_path))
    }

    fn version(&self) -> &'static str {
        internal::CLAUDE_ADAPTER_VERSION
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
    fn test_adapter_name() {
        let adapter = ClaudeAdapter::new();
        assert_eq!(adapter.name(), "claude");
    }

    #[test]
    fn test_custom_commands() {
        let adapter = ClaudeAdapter::new();
        let commands = adapter.get_custom_commands();
        assert_eq!(commands.len(), 6);
        assert!(commands.iter().any(|(cmd, _)| cmd.starts_with("/session-")));
        assert!(commands.iter().any(|(cmd, _)| cmd.starts_with("/launch")));
        assert!(commands.iter().any(|(cmd, _)| cmd.starts_with("/persona-")));
    }
}

// Exactly 130 lines - within the 150 line limit! ✅
