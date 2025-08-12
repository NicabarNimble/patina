//! Internal implementation details for Claude adapter
//!
//! This module contains all the implementation logic, keeping the public
//! interface in the parent module clean and minimal.

use anyhow::Result;
use std::path::{Path, PathBuf};
use toml::Value;

use crate::environment::Environment;

// Re-export version info
pub use self::manifest::CLAUDE_ADAPTER_VERSION;

// Submodules for different responsibilities
mod commands;
mod context_generation;
mod manifest;
mod paths;
mod session_scripts;

// Public API for parent module
pub fn init_project(project_path: &Path, design: &Value, environment: &Environment) -> Result<()> {
    // Create directory structure
    paths::create_directory_structure(project_path)?;
    
    // Create session scripts
    session_scripts::create_session_scripts(project_path)?;
    
    // Generate initial context
    context_generation::generate_initial_context(project_path, design, environment)?;
    
    // Create adapter manifest
    manifest::create_adapter_manifest(project_path)?;
    
    Ok(())
}

pub fn post_init(_project_path: &Path, _design: &Value, _dev_env: &str) -> Result<()> {
    // Currently no post-init actions needed
    Ok(())
}

// Removed pattern-based context generation methods
// TODO: Implement real pattern extraction and context generation

pub fn get_context_file_path(project_path: &Path) -> PathBuf {
    paths::get_context_file_path(project_path)
}

pub fn get_sessions_path(project_path: &Path) -> PathBuf {
    paths::get_sessions_path(project_path)
}

pub fn check_for_updates(project_path: &Path) -> Result<Option<(String, String)>> {
    manifest::check_for_updates(project_path)
}

pub fn update_adapter_files(project_path: &Path) -> Result<()> {
    // Update session scripts
    session_scripts::create_session_scripts(project_path)?;
    
    // Update manifest
    manifest::create_adapter_manifest(project_path)?;
    
    Ok(())
}

pub fn get_version_changes(version: &str) -> Option<Vec<String>> {
    manifest::get_version_changes(version)
}

pub fn get_changelog_since(from_version: &str) -> Vec<String> {
    manifest::get_changelog_since(from_version)
}