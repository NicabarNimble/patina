//! Internal implementation for project module
//!
//! Handles .patina/config.toml - unified project configuration.
//! Supports migration from legacy config.json format.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// =============================================================================
// Config Types - Unified Schema
// =============================================================================

/// Project configuration stored in .patina/config.toml
/// All sections are optional with defaults for backward compatibility
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub project: ProjectSection,
    #[serde(default)]
    pub dev: DevSection,
    #[serde(default)]
    pub frontends: FrontendsSection,
    #[serde(default)]
    pub embeddings: EmbeddingsSection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<EnvironmentSection>,
}

impl ProjectConfig {
    /// Create config with project name
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            project: ProjectSection {
                name: name.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSection {
    /// Project name
    #[serde(default = "default_name")]
    pub name: String,
    /// Mode: "owner" (patina artifacts in main) or "contrib" (CI strips artifacts)
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Creation timestamp (ISO 8601)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
}

fn default_name() -> String {
    "unnamed".to_string()
}
fn default_mode() -> String {
    "owner".to_string()
}

impl Default for ProjectSection {
    fn default() -> Self {
        Self {
            name: default_name(),
            mode: default_mode(),
            created: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevSection {
    /// Dev environment type: "docker" | "native"
    #[serde(default = "default_dev_type", rename = "type")]
    pub dev_type: String,
    /// Dev environment version
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

fn default_dev_type() -> String {
    "docker".to_string()
}

impl Default for DevSection {
    fn default() -> Self {
        Self {
            dev_type: default_dev_type(),
            version: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendsSection {
    /// Allowed frontends for this project
    #[serde(default = "default_allowed")]
    pub allowed: Vec<String>,
    /// Default frontend for this project
    #[serde(default = "default_frontend")]
    pub default: String,
}

fn default_allowed() -> Vec<String> {
    vec!["claude".to_string()]
}
fn default_frontend() -> String {
    "claude".to_string()
}

impl Default for FrontendsSection {
    fn default() -> Self {
        Self {
            allowed: default_allowed(),
            default: default_frontend(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsSection {
    /// Embedding model to use
    #[serde(default = "default_model")]
    pub model: String,
}

fn default_model() -> String {
    "e5-base-v2".to_string()
}

impl Default for EmbeddingsSection {
    fn default() -> Self {
        Self {
            model: default_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSection {
    /// Operating system
    pub os: String,
    /// Architecture
    pub arch: String,
    /// Detected tools
    #[serde(default)]
    pub detected_tools: Vec<String>,
}

// =============================================================================
// Path Functions
// =============================================================================

/// Get the .patina directory for a project
pub fn patina_dir(project_path: &Path) -> PathBuf {
    project_path.join(".patina")
}

/// Get the config file path for a project
pub fn config_path(project_path: &Path) -> PathBuf {
    patina_dir(project_path).join("config.toml")
}

/// Get the legacy config.json path
pub fn legacy_config_path(project_path: &Path) -> PathBuf {
    patina_dir(project_path).join("config.json")
}

/// Get the backups directory for a project
pub fn backups_dir(project_path: &Path) -> PathBuf {
    patina_dir(project_path).join("backups")
}

// =============================================================================
// Detection
// =============================================================================

/// Check if a directory is a patina project
pub fn is_patina_project(path: &Path) -> bool {
    patina_dir(path).exists()
}

/// Check if legacy config.json exists
pub fn has_legacy_config(project_path: &Path) -> bool {
    legacy_config_path(project_path).exists()
}

// =============================================================================
// Migration
// =============================================================================

/// Migrate from legacy config.json to unified config.toml
/// Returns true if migration was performed
pub fn migrate_legacy_config(project_path: &Path) -> Result<bool> {
    let json_path = legacy_config_path(project_path);
    if !json_path.exists() {
        return Ok(false);
    }

    // Load existing TOML config (may have [embeddings] section)
    let mut config = load(project_path)?;

    // Read legacy JSON
    let json_content = fs::read_to_string(&json_path)
        .with_context(|| format!("Failed to read legacy config: {}", json_path.display()))?;
    let json: serde_json::Value = serde_json::from_str(&json_content)
        .with_context(|| "Failed to parse legacy config.json")?;

    // Extract fields from JSON
    if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
        config.project.name = name.to_string();
    }
    if let Some(created) = json.get("created").and_then(|v| v.as_str()) {
        config.project.created = Some(created.to_string());
    }
    if let Some(dev) = json.get("dev").and_then(|v| v.as_str()) {
        config.dev.dev_type = dev.to_string();
    }
    if let Some(llm) = json.get("llm").and_then(|v| v.as_str()) {
        // Map llm to frontends.default and ensure it's in allowed list
        config.frontends.default = llm.to_string();
        if !config.frontends.allowed.contains(&llm.to_string()) {
            config.frontends.allowed.push(llm.to_string());
        }
    }

    // Extract environment snapshot if present
    if let Some(env) = json.get("environment_snapshot") {
        let os = env.get("os").and_then(|v| v.as_str()).unwrap_or("unknown");
        let arch = env
            .get("arch")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let tools = env
            .get("detected_tools")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        config.environment = Some(EnvironmentSection {
            os: os.to_string(),
            arch: arch.to_string(),
            detected_tools: tools,
        });
    }

    // Save unified config
    save(project_path, &config)?;

    // Backup and remove legacy config
    backup_file(project_path, &json_path)?;
    fs::remove_file(&json_path)?;

    Ok(true)
}

// =============================================================================
// Config Load/Save
// =============================================================================

/// Load project config from .patina/config.toml
/// Automatically migrates from legacy config.json if needed
pub fn load(project_path: &Path) -> Result<ProjectConfig> {
    let path = config_path(project_path);

    if !path.exists() {
        return Ok(ProjectConfig::default());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read project config: {}", path.display()))?;

    toml::from_str(&contents)
        .with_context(|| format!("Failed to parse project config: {}", path.display()))
}

/// Load project config with automatic migration
pub fn load_with_migration(project_path: &Path) -> Result<ProjectConfig> {
    // Try migration first (short-circuit: only migrate if legacy config exists)
    if has_legacy_config(project_path) && migrate_legacy_config(project_path)? {
        eprintln!("  ✓ Migrated config.json → config.toml");
    }
    load(project_path)
}

/// Save project config to .patina/config.toml
pub fn save(project_path: &Path, config: &ProjectConfig) -> Result<()> {
    let path = config_path(project_path);

    // Ensure .patina directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let contents = toml::to_string_pretty(config)?;
    fs::write(&path, contents)?;
    Ok(())
}

// =============================================================================
// Backup
// =============================================================================

/// Create a backup of a file before modifying it
/// Returns the backup path if a backup was created
pub fn backup_file(project_path: &Path, file_path: &Path) -> Result<Option<PathBuf>> {
    if !file_path.exists() {
        return Ok(None);
    }

    let backups = backups_dir(project_path);
    fs::create_dir_all(&backups)?;

    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let filename = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let backup_path = backups.join(format!("{}-{}", filename, timestamp));

    fs::copy(file_path, &backup_path).with_context(|| {
        format!(
            "Failed to backup {} to {}",
            file_path.display(),
            backup_path.display()
        )
    })?;

    Ok(Some(backup_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = ProjectConfig::default();
        assert_eq!(config.project.name, "unnamed");
        assert_eq!(config.project.mode, "owner");
        assert_eq!(config.dev.dev_type, "docker");
        assert_eq!(config.frontends.default, "claude");
        assert!(config.frontends.allowed.contains(&"claude".to_string()));
        assert_eq!(config.embeddings.model, "e5-base-v2");
    }

    #[test]
    fn test_config_with_name() {
        let config = ProjectConfig::with_name("my-project");
        assert_eq!(config.project.name, "my-project");
    }

    #[test]
    fn test_config_serialization() {
        let config = ProjectConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[project]"));
        assert!(toml_str.contains("[dev]"));
        assert!(toml_str.contains("[frontends]"));
        assert!(toml_str.contains("[embeddings]"));
    }

    #[test]
    fn test_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let project_path = tmp.path();

        let mut config = ProjectConfig::with_name("test-project");
        config.frontends.allowed = vec!["claude".to_string(), "gemini".to_string()];

        save(project_path, &config).unwrap();
        let loaded = load(project_path).unwrap();

        assert_eq!(loaded.project.name, "test-project");
        assert_eq!(loaded.frontends.allowed.len(), 2);
    }

    #[test]
    fn test_load_missing_returns_default() {
        let tmp = TempDir::new().unwrap();
        let config = load(tmp.path()).unwrap();
        assert_eq!(config.project.name, "unnamed");
    }

    #[test]
    fn test_load_partial_config() {
        // Test that loading a config with only [embeddings] works (backward compat)
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".patina/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, "[embeddings]\nmodel = \"all-minilm-l6-v2\"\n").unwrap();

        let config = load(tmp.path()).unwrap();
        assert_eq!(config.embeddings.model, "all-minilm-l6-v2");
        // Other sections should have defaults
        assert_eq!(config.project.name, "unnamed");
        assert_eq!(config.frontends.default, "claude");
    }

    #[test]
    fn test_is_patina_project() {
        let tmp = TempDir::new().unwrap();
        assert!(!is_patina_project(tmp.path()));

        fs::create_dir_all(patina_dir(tmp.path())).unwrap();
        assert!(is_patina_project(tmp.path()));
    }

    #[test]
    fn test_migrate_legacy_config() {
        let tmp = TempDir::new().unwrap();
        let patina = patina_dir(tmp.path());
        fs::create_dir_all(&patina).unwrap();

        // Create legacy config.json
        let json = r#"{
            "name": "test-project",
            "llm": "gemini",
            "dev": "native",
            "created": "2025-01-01T00:00:00Z",
            "environment_snapshot": {
                "os": "linux",
                "arch": "x86_64",
                "detected_tools": ["cargo", "git"]
            }
        }"#;
        fs::write(patina.join("config.json"), json).unwrap();

        // Create existing config.toml with just embeddings
        fs::write(
            patina.join("config.toml"),
            "[embeddings]\nmodel = \"bge-base\"\n",
        )
        .unwrap();

        // Migrate
        let migrated = migrate_legacy_config(tmp.path()).unwrap();
        assert!(migrated);

        // Verify migration
        let config = load(tmp.path()).unwrap();
        assert_eq!(config.project.name, "test-project");
        assert_eq!(config.dev.dev_type, "native");
        assert_eq!(config.frontends.default, "gemini");
        assert!(config.frontends.allowed.contains(&"gemini".to_string()));
        assert_eq!(config.embeddings.model, "bge-base"); // preserved from existing toml

        // Verify JSON was removed
        assert!(!legacy_config_path(tmp.path()).exists());

        // Verify backup was created
        assert!(backups_dir(tmp.path()).exists());
    }
}
