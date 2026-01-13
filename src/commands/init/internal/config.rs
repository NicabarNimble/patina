//! Configuration management for project initialization

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use patina::dev_env::DevEnvironment;
use patina::environment::Environment;
use patina::project::{
    DevSection, EmbeddingsSection, EnvironmentSection, AdaptersSection, ProjectConfig,
    ProjectSection, RetrievalSection, SearchSection,
};
// Note: CiSection and UpstreamSection are optional, set to None for new projects
use patina::version::VersionManifest;

/// Create project configuration file (unified config.toml format)
///
/// Note: This creates a minimal skeleton config with empty adapters.
/// Use 'patina adapter add <name>' to add LLM support.
pub fn create_project_config(
    project_path: &Path,
    name: &str,
    dev: &str,
    environment: &Environment,
    _dev_env: &dyn DevEnvironment,
) -> Result<()> {
    let patina_dir = project_path.join(".patina");
    fs::create_dir_all(&patina_dir).context("Failed to create .patina directory")?;

    // Check if config already exists (re-init case)
    // Note: We check if the config FILE exists, not just call load()
    // because load() returns a default config when file doesn't exist
    let config_file = patina_dir.join("config.toml");
    let existing_config = if config_file.exists() {
        patina::project::load(project_path).ok()
    } else {
        None
    };

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
            // Preserve existing name and created date on re-init
            name: existing_config
                .as_ref()
                .map(|c| c.project.name.clone())
                .unwrap_or_else(|| name.to_string()),
            created: existing_config
                .as_ref()
                .and_then(|c| c.project.created.clone())
                .or_else(|| Some(chrono::Utc::now().to_rfc3339())),
        },
        dev: DevSection {
            dev_type: dev.to_string(),
            version: None,
        },
        // Preserve existing adapters on re-init, otherwise empty
        adapters: existing_config
            .as_ref()
            .map(|c| c.adapters.clone())
            .unwrap_or_else(|| AdaptersSection {
                allowed: vec![],
                default: String::new(),
            }),
        // Preserve existing upstream/ci on re-init
        upstream: existing_config.as_ref().and_then(|c| c.upstream.clone()),
        ci: existing_config.as_ref().and_then(|c| c.ci.clone()),
        embeddings: EmbeddingsSection {
            model: "e5-base-v2".to_string(),
        },
        search: SearchSection::default(),
        retrieval: RetrievalSection::default(),
        // Always refresh environment detection
        environment: Some(EnvironmentSection {
            os: environment.os.clone(),
            arch: environment.arch.clone(),
            detected_tools,
        }),
    };

    // Save using project module
    patina::project::save(project_path, &config)?;

    // Create default oxidize recipe for embeddings
    create_oxidize_recipe(&patina_dir)?;

    Ok(())
}

/// Create default oxidize.yaml recipe for embeddings
fn create_oxidize_recipe(patina_dir: &Path) -> Result<()> {
    let recipe_path = patina_dir.join("oxidize.yaml");
    if recipe_path.exists() {
        return Ok(()); // Don't overwrite existing recipe
    }

    let default_recipe = r#"# Oxidize Recipe - Build embeddings and projections
# Run: patina oxidize

version: 1
embedding_model: e5-base-v2

projections:
  # Semantic projection - observations from same session are similar
  semantic:
    layers: [768, 1024, 256]
    epochs: 10
    batch_size: 32

  # Temporal projection - files that co-change are related
  temporal:
    layers: [768, 1024, 256]
    epochs: 10
    batch_size: 32

  # Dependency projection - functions that call each other are related
  dependency:
    layers: [768, 1024, 256]
    epochs: 10
    batch_size: 32
"#;

    fs::write(&recipe_path, default_recipe)
        .with_context(|| format!("Failed to create oxidize recipe: {}", recipe_path.display()))?;

    Ok(())
}

/// Create or update version manifest
pub fn handle_version_manifest(
    project_path: &Path,
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
