use anyhow::{Context, Result};
use colored::Colorize;
use patina::indexer::{Confidence, Layer, Location, NavigationResponse, PatternIndexer};
use patina::session::SessionManager;
use std::path::Path;

/// Execute navigation query
pub fn execute(
    query: &str,
    _all_branches: bool, // TODO: Implement cross-branch navigation
    layer: Option<String>,
    json_output: bool,
) -> Result<()> {
    // Find project root
    let project_root = SessionManager::find_project_root()
        .context("Not in a Patina project directory. Run 'patina init' first.")?;

    // Initialize indexer with database if available
    let runtime = tokio::runtime::Runtime::new()?;
    let indexer = runtime.block_on(async {
        // Try to connect to rqlite first
        let db_url = "http://localhost:4001";
        match PatternIndexer::with_database(db_url).await {
            Ok(indexer) => {
                if !json_output {
                    println!("Connected to rqlite database at {}", db_url);
                }
                Ok(indexer)
            }
            Err(e) => {
                if !json_output {
                    eprintln!("Warning: Could not connect to rqlite at {}: {}", db_url, e);
                    eprintln!(
                        "Using in-memory indexing only. Start rqlite with: docker-compose up -d"
                    );
                }
                PatternIndexer::new()
            }
        }
    })?;

    // For now, scan the layer directory to build index
    let layer_path = project_root.join("layer");
    if !layer_path.exists() {
        if !json_output {
            println!("No layer directory found. No patterns to navigate.");
        }
        return Ok(());
    }

    // Index all markdown files in the layer directory
    // With rqlite connected, this will persist to database
    runtime.block_on(async { index_layer_directory(&indexer, &layer_path).await })?;

    // Execute the navigation query
    let response = runtime.block_on(async { indexer.navigate(query).await });

    // Filter by layer if specified
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

/// Index all markdown files in the layer directory
async fn index_layer_directory(indexer: &PatternIndexer, layer_path: &Path) -> Result<()> {
    use walkdir::WalkDir;

    for entry in WalkDir::new(layer_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Only index markdown files
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            // Skip session files for now
            if path.to_string_lossy().contains("/sessions/") {
                continue;
            }

            if let Err(e) = indexer.index_document(path).await {
                eprintln!("Warning: Failed to index {}: {}", path.display(), e);
            }
        }
    }

    Ok(())
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
        Confidence::Experimental => "?".red(),
        Confidence::Historical => "⌛".dimmed(),
    };

    let path_display = location.path.to_string_lossy();
    let short_path = if let Some(pos) = path_display.rfind("layer/") {
        &path_display[pos + 6..]
    } else {
        &path_display
    };

    println!(
        "  {} {} - {}",
        confidence_indicator,
        short_path.bright_white(),
        location.relevance.dimmed()
    );

    // Display git state if available
    if let Some(git_state) = &location.git_state {
        let git_info = match git_state {
            patina::indexer::GitState::Merged { into_branch, .. } => {
                format!("merged to {}", into_branch).green()
            }
            patina::indexer::GitState::Pushed { branch, .. } => {
                format!("pushed to {}", branch).bright_green()
            }
            patina::indexer::GitState::Committed { message, .. } => {
                let short_msg = message.lines().next().unwrap_or("");
                format!("committed: {}", short_msg).yellow()
            }
            patina::indexer::GitState::Modified { .. } => "modified".bright_yellow(),
            patina::indexer::GitState::Untracked { .. } => "untracked".red(),
            _ => String::new().normal(),
        };

        if !git_info.is_empty() {
            println!("      {}", git_info.dimmed());
        }
    }
}

/// Display results in JSON format
fn display_json_results(response: &NavigationResponse) -> Result<()> {
    // Convert to a JSON-serializable structure
    let json_response = serde_json::json!({
        "query": response.query,
        "total_results": response.locations.len(),
        "locations": response.locations.iter().map(|loc| {
            serde_json::json!({
                "path": loc.path.to_string_lossy(),
                "layer": format!("{:?}", loc.layer),
                "relevance": loc.relevance,
                "confidence": format!("{:?}", loc.confidence),
                "git_state": loc.git_state.as_ref().map(|gs| format!("{:?}", gs)),
            })
        }).collect::<Vec<_>>(),
        "confidence_explanation": response.confidence_explanation,
    });

    println!("{}", serde_json::to_string_pretty(&json_response)?);
    Ok(())
}
