use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// Import from refactored module if available, otherwise from old module
#[cfg(not(feature = "refactored"))]
use crate::adapters::claude::CLAUDE_ADAPTER_VERSION;
#[cfg(feature = "refactored")]
use crate::adapters::claude_refactored::CLAUDE_ADAPTER_VERSION;
use crate::adapters::gemini::GEMINI_ADAPTER_VERSION;
use crate::dev_env::dagger::DAGGER_VERSION;
use crate::dev_env::docker::DOCKER_VERSION;

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionManifest {
    pub patina: String,
    pub components: HashMap<String, ComponentInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentInfo {
    pub version: String,
    pub description: String,
}

impl Default for VersionManifest {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionManifest {
    pub fn new() -> Self {
        let mut components = HashMap::new();

        // LLM Adapters
        components.insert(
            "claude-adapter".to_string(),
            ComponentInfo {
                version: CLAUDE_ADAPTER_VERSION.to_string(),
                description: "Claude AI session management and context generation".to_string(),
            },
        );

        components.insert(
            "gemini-adapter".to_string(),
            ComponentInfo {
                version: GEMINI_ADAPTER_VERSION.to_string(),
                description: "Gemini AI context file generation".to_string(),
            },
        );

        // Dev Environments
        components.insert(
            "dagger".to_string(),
            ComponentInfo {
                version: DAGGER_VERSION.to_string(),
                description: "Dagger CI/CD pipeline integration".to_string(),
            },
        );

        components.insert(
            "docker".to_string(),
            ComponentInfo {
                version: DOCKER_VERSION.to_string(),
                description: "Docker containerization templates and integration".to_string(),
            },
        );

        Self {
            patina: env!("CARGO_PKG_VERSION").to_string(),
            components,
        }
    }

    pub fn load(project_path: &Path) -> Result<Self> {
        let manifest_path = project_path.join(".patina").join("versions.json");

        if manifest_path.exists() {
            let content = fs::read_to_string(&manifest_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::new())
        }
    }

    pub fn save(&self, project_path: &Path) -> Result<()> {
        let patina_dir = project_path.join(".patina");
        fs::create_dir_all(&patina_dir)?;
        let manifest_path = patina_dir.join("versions.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(manifest_path, content)?;
        Ok(())
    }

    pub fn get_component_version(&self, component: &str) -> Option<&str> {
        self.components.get(component).map(|c| c.version.as_str())
    }

    pub fn update_component_version(&mut self, component: &str, version: &str) {
        if let Some(info) = self.components.get_mut(component) {
            info.version = version.to_string();
        }
    }
}

pub struct UpdateChecker;

impl UpdateChecker {
    pub fn get_available_versions() -> HashMap<String, String> {
        let mut available = HashMap::new();

        // Pull from the actual constants - single source of truth
        available.insert(
            "claude-adapter".to_string(),
            CLAUDE_ADAPTER_VERSION.to_string(),
        );
        available.insert(
            "gemini-adapter".to_string(),
            GEMINI_ADAPTER_VERSION.to_string(),
        );
        available.insert("dagger".to_string(), DAGGER_VERSION.to_string());
        available.insert("docker".to_string(), DOCKER_VERSION.to_string());

        available
    }

    pub fn check_for_updates(manifest: &VersionManifest) -> Vec<(String, String, String)> {
        let available = Self::get_available_versions();
        let mut updates = Vec::new();

        for (component, available_version) in available {
            if let Some(current_version) = manifest.get_component_version(&component) {
                if current_version != available_version {
                    updates.push((component, current_version.to_string(), available_version));
                }
            }
        }

        updates
    }

    pub fn force_all_updates(manifest: &VersionManifest) -> Vec<(String, String, String)> {
        let available = Self::get_available_versions();
        let mut updates = Vec::new();

        for (component, available_version) in available {
            let current_version = manifest
                .get_component_version(&component)
                .unwrap_or(&available_version)
                .to_string();

            updates.push((component, current_version, available_version));
        }

        updates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_version_manifest_new() {
        let manifest = VersionManifest::new();

        // Check Patina version
        assert_eq!(manifest.patina, env!("CARGO_PKG_VERSION"));

        // Check all expected components exist
        assert!(manifest.components.contains_key("claude-adapter"));
        assert!(manifest.components.contains_key("gemini-adapter"));
        assert!(manifest.components.contains_key("dagger"));
        assert!(manifest.components.contains_key("docker"));

        // Verify component info
        let claude = &manifest.components["claude-adapter"];
        assert_eq!(claude.version, CLAUDE_ADAPTER_VERSION);
        assert!(!claude.description.is_empty());
    }

    #[test]
    fn test_version_manifest_default() {
        let manifest1 = VersionManifest::default();
        let manifest2 = VersionManifest::new();

        assert_eq!(manifest1.patina, manifest2.patina);
        assert_eq!(manifest1.components.len(), manifest2.components.len());
    }

    #[test]
    fn test_load_nonexistent_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let manifest = VersionManifest::load(temp_dir.path()).unwrap();

        // Should return a new manifest
        assert_eq!(manifest.patina, env!("CARGO_PKG_VERSION"));
        assert_eq!(manifest.components.len(), 4);
    }

    #[test]
    fn test_save_and_load_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let manifest = VersionManifest::new();

        // Save manifest
        manifest.save(temp_dir.path()).unwrap();

        // Verify file exists
        let manifest_path = temp_dir.path().join(".patina").join("versions.json");
        assert!(manifest_path.exists());

        // Load and verify
        let loaded = VersionManifest::load(temp_dir.path()).unwrap();
        assert_eq!(loaded.patina, manifest.patina);
        assert_eq!(loaded.components.len(), manifest.components.len());
    }

    #[test]
    fn test_get_component_version() {
        let manifest = VersionManifest::new();

        // Test existing component
        let version = manifest.get_component_version("claude-adapter").unwrap();
        assert_eq!(version, CLAUDE_ADAPTER_VERSION);

        // Test non-existent component
        let version = manifest.get_component_version("nonexistent");
        assert!(version.is_none());
    }

    #[test]
    fn test_update_component_version() {
        let mut manifest = VersionManifest::new();

        // Update existing component
        manifest.update_component_version("claude-adapter", "2.0.0");
        assert_eq!(manifest.components["claude-adapter"].version, "2.0.0");

        // Update non-existent component (should not crash)
        manifest.update_component_version("new-component", "1.0.0");
        // Component should not be added
        assert!(!manifest.components.contains_key("new-component"));
    }

    #[test]
    fn test_update_checker_get_available_versions() {
        let versions = UpdateChecker::get_available_versions();

        assert_eq!(versions.len(), 4);
        assert!(versions.contains_key("claude-adapter"));
        assert!(versions.contains_key("gemini-adapter"));
        assert!(versions.contains_key("dagger"));
        assert!(versions.contains_key("docker"));
    }

    #[test]
    fn test_update_checker_check_for_updates() {
        let mut manifest = VersionManifest::new();

        // Set some components to old versions
        manifest.update_component_version("claude-adapter", "0.1.0");
        manifest.update_component_version("dagger", "0.1.0");

        let updates = UpdateChecker::check_for_updates(&manifest);

        // Should detect 2 updates
        assert_eq!(updates.len(), 2);

        // Verify update details
        for (component, current, available) in &updates {
            assert_eq!(current, "0.1.0");
            match component.as_str() {
                "claude-adapter" => assert_eq!(available, CLAUDE_ADAPTER_VERSION),
                "dagger" => assert_eq!(available, DAGGER_VERSION),
                _ => panic!("Unexpected component in updates"),
            }
        }
    }

    #[test]
    fn test_update_checker_no_updates_needed() {
        let manifest = VersionManifest::new();
        let updates = UpdateChecker::check_for_updates(&manifest);

        // All components should be up to date
        assert!(updates.is_empty());
    }

    #[test]
    fn test_version_manifest_serialization() {
        let manifest = VersionManifest::new();

        // Serialize to JSON
        let json = serde_json::to_string(&manifest).unwrap();

        // Deserialize back
        let deserialized: VersionManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.patina, manifest.patina);
        assert_eq!(deserialized.components.len(), manifest.components.len());

        // Verify a specific component
        assert_eq!(
            deserialized.components["docker"].version,
            manifest.components["docker"].version
        );
    }

    #[test]
    fn test_update_checker_force_all_updates() {
        let manifest = VersionManifest::new();
        let updates = UpdateChecker::force_all_updates(&manifest);

        // Should return all components even if up to date
        assert_eq!(updates.len(), 4);

        // Verify all components are included
        let components: Vec<String> = updates.iter().map(|(c, _, _)| c.clone()).collect();
        assert!(components.contains(&"claude-adapter".to_string()));
        assert!(components.contains(&"gemini-adapter".to_string()));
        assert!(components.contains(&"dagger".to_string()));
        assert!(components.contains(&"docker".to_string()));
    }
}
