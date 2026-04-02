//! Rebuild command - Regenerate Patina projections from layer/ and local sources
//!
//! This is the "clone and go" command that makes Patina projects portable.
//!
//! # Use Cases
//! 1. Clone a repo with `layer/` → `patina rebuild` → working local RAG
//! 2. Corrupted local cache/projections → `patina rebuild` → fresh indices
//! 3. Upgrade embedding model → `patina rebuild` → new projections

use anyhow::{Context, Result};
use patina::eventlog;
use std::path::Path;

/// Options for the rebuild command
#[derive(Default)]
pub struct RebuildOptions {
    /// Only run scrape step (skip oxidize)
    pub scrape_only: bool,
    /// Only run oxidize step (assume db exists)
    pub oxidize_only: bool,
    /// Delete existing data before rebuild
    pub force: bool,
    /// Show what would be rebuilt without doing it
    pub dry_run: bool,
}

/// Validation result for rebuild prerequisites
struct ValidationResult {
    has_git: bool,
    session_count: usize,
    projection_count: usize,
}

/// Execute the rebuild command
pub fn execute(options: RebuildOptions) -> Result<()> {
    println!("🔄 Rebuilding Patina data from layer/\n");

    // Step 1: Validate
    println!("📋 Validation");
    let validation = validate()?;

    if options.dry_run {
        println!("\n🔍 Dry run - would execute:");
        if !options.oxidize_only {
            println!("   • scrape git (if .git/ exists)");
            println!(
                "   • scrape layer ({} sessions + patterns)",
                validation.session_count
            );
            println!("   • scrape code");
        }
        if !options.scrape_only {
            println!("   • oxidize ({} projections)", validation.projection_count);
        }
        println!("\n✅ Dry run complete - no changes made");
        return Ok(());
    }

    // Step 2: Force cleanup if requested
    if options.force {
        println!("\n🗑️  Force mode - clearing existing data...");
        clear_data()?;
    }

    // Step 3: Scrape (unless oxidize-only)
    if !options.oxidize_only {
        println!("\n📥 Scrape (Step 1/2)");
        run_scrape(&validation)?;
    }

    // Step 4: Oxidize (unless scrape-only)
    if !options.scrape_only {
        println!("\n🧪 Oxidize (Step 2/2)");
        run_oxidize()?;
    }

    // Summary
    print_summary()?;

    Ok(())
}

/// Validate that rebuild prerequisites exist
fn validate() -> Result<ValidationResult> {
    // Check layer/ (required)
    if !Path::new("layer").exists() {
        anyhow::bail!(
            "❌ Not a Patina project (no layer/ found)\n\n\
             Run 'patina init .' to initialize this project."
        );
    }
    let session_count = count_sessions()?;
    println!("   ✓ layer/ found ({} sessions)", session_count);

    // Check oxidize.yaml (required)
    if !Path::new(".patina/oxidize.yaml").exists() {
        anyhow::bail!(
            "❌ No recipe found (.patina/oxidize.yaml)\n\n\
             Run 'patina init .' to create the recipe file."
        );
    }
    let projection_count = count_projections()?;
    println!("   ✓ oxidize.yaml found ({} projections)", projection_count);

    // Check .git/ (optional)
    let has_git = Path::new(".git").exists();
    if has_git {
        let commit_count = count_commits()?;
        println!("   ✓ .git/ found ({} commits)", commit_count);
    } else {
        println!("   ⚠️  .git/ not found (git scrape will be skipped)");
    }

    Ok(ValidationResult {
        has_git,
        session_count: count_sessions()?,
        projection_count,
    })
}

/// Count session files in layer/sessions/
fn count_sessions() -> Result<usize> {
    let sessions_dir = Path::new("layer/sessions");
    if !sessions_dir.exists() {
        return Ok(0);
    }

    let count = std::fs::read_dir(sessions_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .count();

    Ok(count)
}

/// Count projections in oxidize.yaml
fn count_projections() -> Result<usize> {
    use crate::commands::oxidize::recipe::OxidizeRecipe;
    let recipe = OxidizeRecipe::load()?;
    Ok(recipe.projections.len())
}

/// Count git commits
fn count_commits() -> Result<usize> {
    let output = std::process::Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .context("Failed to count git commits")?;

    if !output.status.success() {
        return Ok(0);
    }

    let count_str = String::from_utf8_lossy(&output.stdout);
    count_str.trim().parse().unwrap_or(0).pipe(Ok)
}

/// Clear existing data directory
fn clear_data() -> Result<()> {
    let project_root = std::env::current_dir()?;
    let patina_db_path = eventlog::resolve_patina_db_path(&project_root);
    let embeddings_dir = patina::paths::project::embeddings_dir(&project_root);

    if patina_db_path.exists() {
        std::fs::remove_file(&patina_db_path)
            .with_context(|| format!("Failed to remove {}", patina_db_path.display()))?;
        for suffix in ["-wal", "-shm"] {
            let sidecar =
                std::path::PathBuf::from(format!("{}{}", patina_db_path.display(), suffix));
            let _ = std::fs::remove_file(sidecar);
        }
        println!("   ✓ Cleared {}", patina_db_path.display());
    }

    if embeddings_dir.exists() {
        std::fs::remove_dir_all(&embeddings_dir)
            .with_context(|| format!("Failed to remove {}", embeddings_dir.display()))?;
        println!("   ✓ Cleared {}", embeddings_dir.display());
    }
    Ok(())
}

/// Run all scrapers
fn run_scrape(validation: &ValidationResult) -> Result<()> {
    use crate::commands::scrape;

    // Git scrape (if available)
    if validation.has_git {
        print!("   • git: ");
        let stats = scrape::git::run(false)?;
        println!("{} commits", stats.items_processed);
    }

    // Layer scrape (patterns + sessions)
    print!("   • layer: ");
    let stats = scrape::layer::run(false)?;
    println!("{} items", stats.items_processed);

    // Code scrape
    print!("   • code: ");
    scrape::execute_code(false, false)?;
    println!("complete");

    // Get total event count
    let db_path = eventlog::patina_db_path()?;
    if db_path.exists() {
        let total = count_events(&db_path)?;
        println!("   ✓ patina.db: {} events", total);
    }

    Ok(())
}

/// Count total events in database
fn count_events(db_path: &Path) -> Result<usize> {
    let conn = rusqlite::Connection::open(db_path)?;
    let count: usize = conn.query_row("SELECT COUNT(*) FROM eventlog", [], |row| row.get(0))?;
    Ok(count)
}

/// Run oxidize to build projections
fn run_oxidize() -> Result<()> {
    use crate::commands::oxidize;

    oxidize::oxidize()?;

    Ok(())
}

/// Print summary of rebuild results
fn print_summary() -> Result<()> {
    println!("\n✅ Rebuild complete!");

    // Database size
    let db_path = eventlog::patina_db_path()?;
    if db_path.exists() {
        let size_kb = std::fs::metadata(&db_path)?.len() / 1024;
        println!("   Database: {} ({} KB)", db_path.display(), size_kb);
    }

    // Embeddings size
    let project_root = std::env::current_dir()?;
    let embeddings_dir = patina::paths::project::embeddings_dir(&project_root);
    if embeddings_dir.exists() {
        let size_kb = dir_size(&embeddings_dir)? / 1024;
        println!("   Indices: {} ({} KB)", embeddings_dir.display(), size_kb);
    }

    Ok(())
}

/// Calculate total size of a directory
fn dir_size(path: &Path) -> Result<u64> {
    let mut total = 0;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                total += dir_size(&path)?;
            } else {
                total += std::fs::metadata(&path)?.len();
            }
        }
    }
    Ok(total)
}

/// Pipe trait for functional chaining
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl<T> Pipe for T {}
