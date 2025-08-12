//! Configuration management for project initialization

use anyhow::{Context, Result};
use serde_json::json;
use std::fs;
use std::path::Path;

use patina::environment::Environment;
use patina::version::VersionManifest;
use patina::dev_env::DevEnvironment;

/// Create project configuration file
pub fn create_project_config(
    project_path: &Path,
    name: &str,
    llm: &str,
    dev: &str,
    environment: &Environment,
    dev_env: &dyn DevEnvironment,
) -> Result<()> {
    let patina_dir = project_path.join(".patina");
    fs::create_dir_all(&patina_dir).context("Failed to create .patina directory")?;
    
    // Create dev manifest
    let dev_manifest = json!({
        "environment": dev,
        "version": dev_env.version(),
        "available": dev_env.is_available(),
    });
    
    // Store current project configuration with environment snapshot
    let config = json!({
        "name": name,
        "llm": llm,
        "dev": dev,
        "dev_manifest": dev_manifest,
        "created": chrono::Utc::now().to_rfc3339(),
        "environment_snapshot": {
            "os": environment.os,
            "arch": environment.arch,
            "detected_tools": environment.tools.iter()
                .filter(|(_, info)| info.available)
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
        }
    });
    
    let config_path = patina_dir.join("config.json");
    fs::write(&config_path, serde_json::to_string_pretty(&config)?)
        .context("Failed to write project config")?;
        
    Ok(())
}

/// Create or update version manifest
pub fn handle_version_manifest(
    project_path: &Path,
    _llm: &str,
    _dev: &str,
    is_reinit: bool,
    json_output: bool,
) -> Result<Option<Vec<(String, String, String)>>> {
    let manifest_path = project_path.join(".patina");
    
    let mut updates_available = Vec::new();
    
    if is_reinit && manifest_path.join("versions.json").exists() {
        // Check for component updates
        if !json_output {
            println!("🔍 Checking for component updates...");
        }
        
        let current_manifest = VersionManifest::load(&manifest_path)?;
        let latest_manifest = VersionManifest::new();
        
        // Compare versions
        for (component, latest_info) in &latest_manifest.components {
            if let Some(current_version) = current_manifest.get_component_version(component) {
                if current_version != latest_info.version {
                    updates_available.push((
                        component.clone(),
                        current_version.to_string(),
                        latest_info.version.clone(),
                    ));
                }
            }
        }
        
        if !updates_available.is_empty() && !json_output {
            println!("\n📦 Component updates available:");
            for (component, current, latest) in &updates_available {
                println!("  • {}: {} → {}", component, current, latest);
            }
            println!();
        }
    }
    
    // Create and save new version manifest
    let manifest = VersionManifest::new();
    manifest.save(project_path)?;
    
    Ok(if updates_available.is_empty() { None } else { Some(updates_available) })
}