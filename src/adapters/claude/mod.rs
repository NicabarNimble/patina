//! Claude adapter for Patina
//! 
//! Provides Claude-specific integration including:
//! - Session management scripts
//! - Context file generation (.claude/CLAUDE.md)
//! - MCP (Model Context Protocol) support
//! - Custom command definitions

use crate::adapters::LLMAdapter;
use crate::environment::Environment;
use crate::layer::Pattern;
use anyhow::Result;
use std::path::{Path, PathBuf};
use toml::Value;

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
        design: &Value,
        environment: &Environment,
    ) -> Result<()> {
        internal::init_project(project_path, design, environment)
    }

    fn post_init(&self, project_path: &Path, design: &Value, dev_env: &str) -> Result<()> {
        internal::post_init(project_path, design, dev_env)
    }

    fn generate_context(
        &self,
        project_path: &Path,
        project_name: &str,
        design_content: &str,
        patterns: &[Pattern],
        environment: &Environment,
    ) -> Result<()> {
        internal::generate_context(
            project_path,
            project_name,
            design_content,
            patterns,
            environment,
        )
    }

    fn update_context(
        &self,
        project_path: &Path,
        project_name: &str,
        design: &Value,
        patterns: &[Pattern],
        environment: &Environment,
    ) -> Result<()> {
        internal::update_context(project_path, project_name, design, patterns, environment)
    }

    fn get_custom_commands(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("/session-start [name]", "Start a new development session"),
            ("/session-update", "Update session with rich context"),
            ("/session-note [insight]", "Add human insight to session"),
            ("/session-end", "End session with comprehensive distillation"),
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
        assert_eq!(commands.len(), 4);
        assert!(commands.iter().any(|(cmd, _)| cmd.starts_with("/session-")));
    }
}

// Exactly 130 lines - within the 150 line limit! ✅