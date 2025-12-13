//! Configuration management for project initialization

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use patina::dev_env::DevEnvironment;
use patina::environment::Environment;
use patina::project::{
    DevSection, EmbeddingsSection, EnvironmentSection, FrontendsSection, ProjectConfig,
    ProjectSection, RetrievalSection, SearchSection,
};
// Note: CiSection and UpstreamSection are optional, set to None for new projects
use patina::version::VersionManifest;

/// Create project configuration file (unified config.toml format)
pub fn create_project_config(
    project_path: &Path,
    name: &str,
    llm: &str,
    dev: &str,
    environment: &Environment,
    _dev_env: &dyn DevEnvironment,
) -> Result<()> {
    let patina_dir = project_path.join(".patina");
    fs::create_dir_all(&patina_dir).context("Failed to create .patina directory")?;

    // Build detected tools list
    let detected_tools: Vec<String> = environment
        .tools
        .iter()
        .filter(|(_, info)| info.available)
        .map(|(name, _)| name.clone())
        .collect();

    // Create unified project config
    // Note: upstream and ci are None by default (owned repo)
    // For contrib repos, user/LLM sets [upstream] section later
    let config = ProjectConfig {
        project: ProjectSection {
            name: name.to_string(),
            created: Some(chrono::Utc::now().to_rfc3339()),
        },
        dev: DevSection {
            dev_type: dev.to_string(),
            version: None,
        },
        frontends: FrontendsSection {
            allowed: vec![llm.to_string()],
            default: llm.to_string(),
        },
        upstream: None, // Set when contributing to another repo
        ci: None,       // Set with repo's CI requirements
        embeddings: EmbeddingsSection {
            model: "e5-base-v2".to_string(),
        },
        search: SearchSection::default(),
        retrieval: RetrievalSection::default(),
        environment: Some(EnvironmentSection {
            os: environment.os.clone(),
            arch: environment.arch.clone(),
            detected_tools,
        }),
    };

    // Save using project module
    patina::project::save(project_path, &config)?;

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
                println!("  • {component}: {current} → {latest}");
            }
            println!();
        }
    }

    // Create and save new version manifest
    let manifest = VersionManifest::new();
    manifest.save(project_path)?;

    Ok(if updates_available.is_empty() {
        None
    } else {
        Some(updates_available)
    })
}
