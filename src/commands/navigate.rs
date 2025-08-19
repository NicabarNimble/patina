use anyhow::{Context, Result};
use colored::Colorize;
use patina::indexer::{Confidence, Layer, Location, NavigationResponse, PatternIndexer};
use patina::session::SessionManager;
use std::path::Path;
use std::process::Command;

/// Execute navigation query
pub fn execute(
    query: &str,
    _all_branches: bool, // TODO: Implement cross-branch navigation
    layer: Option<String>,
    json_output: bool,
) -> Result<()> {
    // Validate layer filter early if provided
    if let Some(ref layer_filter) = layer {
        match layer_filter.to_lowercase().as_str() {
            "core" | "surface" | "dust" => {}
            _ => {
                anyhow::bail!(
                    "Invalid layer: {}. Must be one of: core, surface, dust",
                    layer_filter
                );
            }
        }
    }

    // Find project root
    let project_root = SessionManager::find_project_root()
        .context("Not in a Patina project directory. Run 'patina init' first.")?;

    // Initialize indexer with HybridDatabase (SQLite + optional CRDT)
    let db_path = project_root.join(".patina/navigation.db");
    let enable_crdt = std::env::var("PATINA_ENABLE_CRDT").is_ok();

    let indexer = if db_path.parent().map(|p| p.exists()).unwrap_or(false) {
        // Try HybridDatabase first
        match PatternIndexer::with_hybrid_database(&db_path, enable_crdt) {
            Ok(indexer) => {
                if !json_output {
                    println!(
                        "Using HybridDatabase at {} (CRDT: {})",
                        db_path.display(),
                        if enable_crdt { "enabled" } else { "disabled" }
                    );
                }
                indexer
            }
            Err(e) => {
                if !json_output {
                    eprintln!("Warning: Could not open HybridDatabase: {e}");
                    eprintln!("Falling back to regular SQLite...");
                }
                // Fall back to regular SQLite
                match PatternIndexer::with_database(&db_path) {
                    Ok(indexer) => {
                        if !json_output {
                            println!("Using regular SQLite database at {}", db_path.display());
                        }
                        indexer
                    }
                    Err(e2) => {
                        if !json_output {
                            eprintln!("Warning: Could not open any database: {e2}");
                            eprintln!("Using in-memory indexing only.");
                        }
                        PatternIndexer::new()?
                    }
                }
            }
        }
    } else {
        if !json_output {
            println!("Using in-memory indexing (no .patina directory found)");
        }
        PatternIndexer::new()?
    };

    // For now, scan the layer directory to build index
    let layer_path = project_root.join("layer");
    if !layer_path.exists() {
        if !json_output {
            println!("No 'layer' directory found. Creating one...");
        }
        std::fs::create_dir_all(&layer_path)?;
    }

    // Index the patterns
    indexer.index_directory(&layer_path)?;

    // Navigate and get results
    let mut response = indexer.navigate(query);

    // Update confidence based on Git file age (NEW!)
    for location in &mut response.locations {
        if let Some(age_confidence) = calculate_git_age_confidence(&location.path) {
            location.confidence = age_confidence;
        }
    }

    // Apply layer filter if specified
    let mut filtered_response = response;
    if let Some(layer_filter) = layer {
        let target_layer = match layer_filter.to_lowercase().as_str() {
            "core" => Layer::Core,
            "surface" => Layer::Surface,
            "dust" => Layer::Dust,
            _ => {
                anyhow::bail!(
                    "Invalid layer: {}. Must be one of: core, surface, dust",
                    layer_filter
                );
            }
        };

        filtered_response
            .locations
            .retain(|loc| loc.layer == target_layer);
    }

    // Display results
    if json_output {
        display_json_results(&filtered_response)?;
    } else {
        display_human_results(&filtered_response, query)?;
    }

    Ok(())
}

/// Calculate confidence based on file age in Git
fn calculate_git_age_confidence(file_path: &Path) -> Option<Confidence> {
    // Get the file's age in days using git log
    let output = Command::new("git")
        .args([
            "log",
            "-1",
            "--format=%ct", // Commit timestamp
            "--",
            file_path.to_str()?,
        ])
        .output()
        .ok()?;

    if output.stdout.is_empty() {
        // File not in Git yet
        return Some(Confidence::Experimental);
    }

    // Parse timestamp
    let timestamp_str = String::from_utf8_lossy(&output.stdout);
    let timestamp: i64 = timestamp_str.trim().parse().ok()?;
    
    // Calculate age in days
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    let age_days = (now - timestamp) / 86400; // seconds per day

    // Also check if file has been modified recently
    let modified_output = Command::new("git")
        .args([
            "diff",
            "--name-only",
            "HEAD",
            "--",
            file_path.to_str()?,
        ])
        .output()
        .ok()?;
    
    let is_modified = !modified_output.stdout.is_empty();

    // Calculate confidence based on age and modification status
    let confidence = if is_modified {
        // Recently modified files have lower confidence
        Confidence::Low
    } else {
        match age_days {
            0..=7 => Confidence::Experimental,   // Very new
            8..=30 => Confidence::Low,          // New, settling
            31..=90 => Confidence::Medium,      // Established
            91..=180 => Confidence::High,       // Proven
            _ => Confidence::Verified,          // Battle-tested (6+ months)
        }
    };

    Some(confidence)
}

/// Display results in human-readable format
fn display_human_results(response: &NavigationResponse, query: &str) -> Result<()> {
    println!("\n🔍 Navigation results for: {}\n", query.cyan());

    if response.locations.is_empty() {
        println!("No patterns found matching your query.");
        println!("\nTry:");
        println!("  • Using different keywords");
        println!("  • Checking if patterns exist in the layer directory");
        println!("  • Running 'patina doctor' to check project health");
        return Ok(());
    }

    // Group by layer
    let mut core_results = Vec::new();
    let mut surface_results = Vec::new();
    let mut dust_results = Vec::new();

    for location in &response.locations {
        match location.layer {
            Layer::Core => core_results.push(location),
            Layer::Surface => surface_results.push(location),
            Layer::Dust => dust_results.push(location),
        }
    }

    // Display Core results first (highest priority)
    if !core_results.is_empty() {
        println!(
            "{}",
            "Core Patterns (Verified Implementation):".green().bold()
        );
        for loc in core_results {
            display_location(loc);
        }
        println!();
    }

    // Display Surface results (active development)
    if !surface_results.is_empty() {
        println!(
            "{}",
            "Surface Patterns (Active Development):".yellow().bold()
        );
        for loc in surface_results {
            display_location(loc);
        }
        println!();
    }

    // Display Dust results (historical)
    if !dust_results.is_empty() {
        println!(
            "{}",
            "Dust Patterns (Historical Reference):".dimmed().bold()
        );
        for loc in dust_results {
            display_location(loc);
        }
        println!();
    }

    // Display confidence explanation
    if !response.confidence_explanation.is_empty() {
        println!("{}", "Confidence Scoring:".blue());
        println!("  {}", response.confidence_explanation.dimmed());
    } else {
        // Add our own confidence explanation
        println!("{}", "Confidence Scoring:".blue());
        println!("  {}", "Based on Git history age and modification status".dimmed());
    }

    Ok(())
}

/// Display a single location result
fn display_location(location: &Location) {
    let confidence_indicator = match location.confidence {
        Confidence::Verified => "✓".green(),
        Confidence::High => "↑".bright_green(),
        Confidence::Medium => "→".yellow(),
        Confidence::Low => "↓".bright_yellow(),
        Confidence::Experimental => "?".bright_magenta(),
        Confidence::Historical => "⌛".dimmed(),
    };

    let path_display = location.path.display().to_string();
    let shortened_path = if path_display.len() > 50 {
        format!("...{}", &path_display[path_display.len() - 47..])
    } else {
        path_display
    };

    println!(
        "  {} {} {}",
        confidence_indicator,
        shortened_path.bright_blue(),
        format!("({})", location.relevance).dimmed()
    );

    // Show Git state if available
    if let Some(ref git_state) = location.git_state {
        println!("      Git: {}", format!("{git_state:?}").dimmed());
    }
}

/// Display results in JSON format
fn display_json_results(response: &NavigationResponse) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(response)?);
    Ok(())
}