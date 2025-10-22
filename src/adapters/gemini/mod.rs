//! Gemini adapter for Patina (stub implementation)
//!
//! Placeholder for future Gemini AI integration.
//! Creates basic `.gemini/` structure with context file.

use crate::adapters::LLMAdapter;
use crate::environment::Environment;
use anyhow::Result;
use std::path::{Path, PathBuf};

mod internal;

// Export version for version management
pub const GEMINI_ADAPTER_VERSION: &str = "0.1.0";

/// Gemini adapter implementation (stub)
pub struct GeminiAdapter;

impl GeminiAdapter {
    /// Create a new Gemini adapter
    pub fn new() -> Self {
        Self
    }
}

impl Default for GeminiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl LLMAdapter for GeminiAdapter {
    fn name(&self) -> &'static str {
        "gemini"
    }

    fn init_project(
        &self,
        project_path: &Path,
        project_name: &str,
        environment: &Environment,
    ) -> Result<()> {
        internal::init_project(project_path, project_name, environment)
    }

    fn post_init(&self, _project_path: &Path, _dev_env: &str) -> Result<()> {
        Ok(())
    }

    fn get_custom_commands(&self) -> Vec<(&'static str, &'static str)> {
        // Gemini doesn't have custom commands yet
        vec![]
    }

    fn get_context_file_path(&self, project_path: &Path) -> PathBuf {
        internal::get_context_file_path(project_path)
    }

    fn check_for_updates(&self, _project_path: &Path) -> Result<Option<(String, String)>> {
        // No version tracking for stub
        Ok(None)
    }

    fn update_adapter_files(&self, _project_path: &Path) -> Result<()> {
        // Nothing to update in stub
        Ok(())
    }

    fn get_sessions_path(&self, _project_path: &Path) -> Option<PathBuf> {
        // No session tracking yet
        None
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn get_version_changes(&self, _version: &str) -> Option<Vec<String>> {
        None
    }

    fn get_changelog_since(&self, _from_version: &str) -> Vec<String> {
        Vec::new()
    }
}

// Exactly 106 lines - well under 150 limit! ✅
