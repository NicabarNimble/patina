//! Model command - Manage embedding models in mother cache
//!
//! Models are downloaded to `~/.patina/cache/models/` and shared across projects.
//! Provenance tracked in `~/.patina/models.lock`.

use anyhow::Result;
use patina::embeddings::models::ModelRegistry;
use patina::models::{self, ModelLock, ModelStatus};
use patina::paths;

/// Model CLI subcommands
#[derive(Debug, Clone, clap::Subcommand)]
pub enum ModelCommands {
    /// List available models with download status
    List,

    /// Download a model to cache
    Add {
        /// Model name (from registry)
        name: String,
    },

    /// Remove a model from cache
    #[command(alias = "rm")]
    Remove {
        /// Model name
        name: String,
    },

    /// Show model status for current project
    Status,
}

/// Execute model command from CLI
pub fn execute_cli(command: Option<ModelCommands>) -> Result<()> {
    match command {
        Some(cmd) => execute(cmd),
        None => execute(ModelCommands::List),
    }
}

/// Execute model command
pub fn execute(command: ModelCommands) -> Result<()> {
    match command {
        ModelCommands::List => list(),
        ModelCommands::Add { name } => add(&name),
        ModelCommands::Remove { name } => remove(&name),
        ModelCommands::Status => status(),
    }
}

/// List available models with download status
fn list() -> Result<()> {
    let registry = ModelRegistry::load()?;
    let lock = ModelLock::load()?;

    println!("📦 Available Models\n");
    println!("{:<25} {:>6} {:>8}  STATUS", "NAME", "DIMS", "SIZE");
    println!("{}", "─".repeat(60));

    let mut models: Vec<_> = registry.models.iter().collect();
    models.sort_by_key(|(name, _)| *name);

    for (name, def) in models {
        let status = models::model_status(name)?;
        let status_str = format_status(&status, &lock);
        let size = def.size_int8.as_deref().unwrap_or("-");

        println!(
            "{:<25} {:>6} {:>8}  {}",
            name, def.dimensions, size, status_str
        );
    }

    // Show cache location
    let cache_dir = paths::models::cache_dir();
    println!("\nCache: {}", cache_dir.display());

    Ok(())
}

fn format_status(status: &ModelStatus, lock: &ModelLock) -> String {
    if status.in_cache {
        if let Some(prov) = lock.get(&status.name) {
            // Parse date from ISO format
            let date = prov
                .downloaded
                .split('T')
                .next()
                .unwrap_or(&prov.downloaded);
            format!("✓ cached ({})", date)
        } else {
            "✓ cached".to_string()
        }
    } else if status.in_local {
        "✓ local".to_string()
    } else {
        "not downloaded".to_string()
    }
}

/// Download a model to cache
fn add(name: &str) -> Result<()> {
    // Check if already in cache
    let status = models::model_status(name)?;
    if status.in_cache {
        println!("Model '{}' already in cache.", name);
        println!("  Location: {:?}", paths::models::model_dir(name));
        return Ok(());
    }

    models::add_model(name)
}

/// Remove a model from cache
fn remove(name: &str) -> Result<()> {
    let model_dir = paths::models::model_dir(name);

    if !model_dir.exists() {
        println!("Model '{}' not in cache.", name);
        return Ok(());
    }

    // Get size for display
    let size = dir_size(&model_dir)?;
    let size_mb = size / (1024 * 1024);

    println!("Remove '{}' from cache? ({} MB)", name, size_mb);
    print!("  [y/N]: ");
    std::io::Write::flush(&mut std::io::stdout())?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() == "y" {
        std::fs::remove_dir_all(&model_dir)?;

        // Update lock file
        let mut lock = ModelLock::load()?;
        lock.remove(name);
        lock.save()?;

        println!("✓ Removed '{}'", name);
    } else {
        println!("Cancelled.");
    }

    Ok(())
}

/// Show model status for current project
fn status() -> Result<()> {
    let lock = ModelLock::load()?;

    println!("📊 Model Status\n");

    // Show what's in cache
    println!("Mother cache:");
    if lock.list().is_empty() {
        println!("  (no models downloaded)");
    } else {
        for name in lock.list() {
            let model = lock.get(name).unwrap();
            let size_mb = model.size_bytes / (1024 * 1024);
            println!("  ✓ {} ({} MB, {} dims)", name, size_mb, model.dimensions);
        }
    }

    // Show what current project needs
    println!("\nCurrent project:");
    match patina::embeddings::models::Config::load() {
        Ok(config) => {
            let model_name = &config.embeddings.model;
            let status = models::model_status(model_name)?;

            let available = if status.in_cache {
                "✓ in cache"
            } else if status.in_local {
                "✓ local"
            } else {
                "✗ not available"
            };

            println!("  Model: {} ({})", model_name, available);

            if !status.in_cache && !status.in_local {
                println!("\n  Run: patina model add {}", model_name);
            }
        }
        Err(_) => {
            println!("  (not a patina project)");
        }
    }

    Ok(())
}

/// Calculate directory size
fn dir_size(path: &std::path::Path) -> Result<u64> {
    let mut size = 0;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                size += std::fs::metadata(&path)?.len();
            }
        }
    }
    Ok(size)
}
