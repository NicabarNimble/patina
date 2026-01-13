//! Project module - Unified project configuration
//!
//! Manages `.patina/config.toml` for project-specific settings including
//! project metadata, dev environment, allowed frontends, and embeddings.
//!
//! Supports automatic migration from legacy `config.json` format.
//!
//! # Example
//!
//! ```no_run
//! use patina::project;
//! use std::path::Path;
//!
//! let path = Path::new(".");
//!
//! // Check if this is a patina project
//! if project::is_patina_project(path) {
//!     // Load config (with automatic migration if needed)
//!     let mut config = project::load_with_migration(path)?;
//!     println!("Project: {}", config.project.name);
//!     println!("Allowed frontends: {:?}", config.frontends.allowed);
//!
//!     // Modify and save
//!     config.frontends.allowed.push("gemini".to_string());
//!     project::save(path, &config)?;
//! }
//! # Ok::<(), anyhow::Error>(())
//! ```

mod internal;

use anyhow::Result;
use std::path::{Path, PathBuf};

// Re-export config types
pub use internal::{
    CiSection, DevSection, EmbeddingsSection, EnvironmentSection, FrontendsSection, ProjectConfig,
    ProjectSection, RetrievalSection, SearchSection, UpstreamSection,
};

/// Check if a directory is a patina project (has .patina/)
pub fn is_patina_project(path: &Path) -> bool {
    internal::is_patina_project(path)
}

/// Check if legacy config.json exists and needs migration
pub fn has_legacy_config(path: &Path) -> bool {
    internal::has_legacy_config(path)
}

/// Load project config from `.patina/config.toml`
///
/// Returns default config if file doesn't exist.
/// Does NOT automatically migrate from config.json.
pub fn load(project_path: &Path) -> Result<ProjectConfig> {
    internal::load(project_path)
}

/// Load project config with automatic migration from config.json
///
/// If legacy config.json exists, merges it into config.toml and removes json.
pub fn load_with_migration(project_path: &Path) -> Result<ProjectConfig> {
    internal::load_with_migration(project_path)
}

/// Migrate from legacy config.json to unified config.toml
///
/// Returns true if migration was performed.
pub fn migrate(project_path: &Path) -> Result<bool> {
    internal::migrate_legacy_config(project_path)
}

/// Save project config to `.patina/config.toml`
///
/// Creates `.patina/` directory if it doesn't exist.
pub fn save(project_path: &Path, config: &ProjectConfig) -> Result<()> {
    internal::save(project_path, config)
}

/// Get the .patina directory path for a project
pub fn patina_dir(project_path: &Path) -> PathBuf {
    internal::patina_dir(project_path)
}

/// Get the backups directory path for a project
pub fn backups_dir(project_path: &Path) -> PathBuf {
    internal::backups_dir(project_path)
}

/// Backup a file before modifying it
///
/// Returns the backup path if successful, None if file didn't exist.
/// Backups are stored in `.patina/backups/` with timestamp suffix.
pub fn backup_file(project_path: &Path, file_path: &Path) -> Result<Option<PathBuf>> {
    internal::backup_file(project_path, file_path)
}

/// Create a unique project identifier if it doesn't exist
///
/// Returns the UID (8 hex characters, created once, never modified).
/// Used for stable project identity across different machines.
pub fn create_uid_if_missing(project_path: &Path) -> Result<String> {
    internal::create_uid_if_missing(project_path)
}

/// Get the UID for a project (returns None if not initialized)
pub fn get_uid(project_path: &Path) -> Option<String> {
    internal::get_uid(project_path)
}

/// Get the UID file path for a project
pub fn uid_path(project_path: &Path) -> PathBuf {
    internal::uid_path(project_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patina_dir() {
        let path = patina_dir(Path::new("/some/project"));
        assert!(path.ends_with(".patina"));
    }
}
