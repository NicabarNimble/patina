use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
        
        components.insert(
            "claude-adapter".to_string(),
            ComponentInfo {
                version: "0.3.0".to_string(),
                description: "Claude AI session management and context generation".to_string(),
            },
        );
        
        components.insert(
            "dagger-templates".to_string(),
            ComponentInfo {
                version: "1.0.0".to_string(),
                description: "Dagger pipeline templates for container workflows".to_string(),
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
        
        available.insert("claude-adapter".to_string(), "0.3.0".to_string());
        available.insert("dagger-templates".to_string(), "1.0.0".to_string());
        
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
}