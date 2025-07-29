use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::adapters::claude::CLAUDE_ADAPTER_VERSION;
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
        let manifest_path = project_path.join(".patina").join("versions.json");
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
        available.insert("claude-adapter".to_string(), CLAUDE_ADAPTER_VERSION.to_string());
        available.insert("gemini-adapter".to_string(), GEMINI_ADAPTER_VERSION.to_string());
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
                    updates.push((
                        component,
                        current_version.to_string(),
                        available_version,
                    ));
                }
            }
        }
        
        updates
    }
    
    pub fn force_all_updates(manifest: &VersionManifest) -> Vec<(String, String, String)> {
        let available = Self::get_available_versions();
        let mut updates = Vec::new();
        
        for (component, available_version) in available {
            let current_version = manifest.get_component_version(&component)
                .unwrap_or(&available_version)
                .to_string();
            
            updates.push((
                component,
                current_version,
                available_version,
            ));
        }
        
        updates
    }
}