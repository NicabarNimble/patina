pub mod claude_refactored;
pub mod gemini;

use crate::environment::Environment;
use crate::layer::Pattern;
use anyhow::Result;
use std::path::Path;
use toml::Value;

/// Trait for LLM-specific implementations
pub trait LLMAdapter {
    /// Get the name of this LLM adapter
    fn name(&self) -> &'static str;

    /// Initialize LLM-specific files and directories during project creation
    fn init_project(
        &self,
        project_path: &Path,
        design: &Value,
        environment: &Environment,
    ) -> Result<()>;

    /// Called after project initialization to perform additional setup
    /// This is where adapters can create development environment files, etc.
    fn post_init(&self, _project_path: &Path, _design: &Value, _dev_env: &str) -> Result<()> {
        Ok(()) // Default: no-op
    }

    /// Generate LLM-specific context from patterns and environment
    fn generate_context(
        &self,
        project_path: &Path,
        project_name: &str,
        design_content: &str,
        patterns: &[Pattern],
        environment: &Environment,
    ) -> Result<()>;

    /// Update existing context with latest information
    fn update_context(
        &self,
        project_path: &Path,
        project_name: &str,
        design: &Value,
        patterns: &[Pattern],
        environment: &Environment,
    ) -> Result<()>;

    /// Get custom commands for this LLM
    fn get_custom_commands(&self) -> Vec<(&'static str, &'static str)> {
        vec![]
    }

    /// Get the main context file path for this LLM
    fn get_context_file_path(&self, project_path: &Path) -> std::path::PathBuf;

    /// Check if adapter files need updating
    /// Returns Some((current_version, available_version)) if update available
    fn check_for_updates(&self, _project_path: &Path) -> Result<Option<(String, String)>> {
        Ok(None) // Default: no updates
    }

    /// Update adapter files to latest version
    fn update_adapter_files(&self, _project_path: &Path) -> Result<()> {
        Ok(()) // Default: no-op
    }

    /// Get version changes for a specific version
    fn get_version_changes(&self, _version: &str) -> Option<Vec<String>> {
        None // Default: no changes
    }

    /// Get all changes since a given version
    fn get_changelog_since(&self, _from_version: &str) -> Vec<String> {
        Vec::new() // Default: no changelog
    }

    /// Get the sessions directory path for this adapter
    fn get_sessions_path(&self, _project_path: &Path) -> Option<std::path::PathBuf> {
        None // Default: no sessions directory
    }

    /// Get the version of this adapter
    fn version(&self) -> &'static str {
        "0.1.0" // Default version
    }
}

/// Get an LLM adapter by name
pub fn get_adapter(llm_name: &str) -> Box<dyn LLMAdapter> {
    match llm_name.to_lowercase().as_str() {
        "claude" => claude_refactored::create(),
        "gemini" => Box::new(gemini::GeminiAdapter),
        _ => claude_refactored::create(),
    }
}
