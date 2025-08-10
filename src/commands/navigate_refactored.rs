// Dependable Rust: Black-box boundary for navigate command
// This hides all indexer implementation details

use anyhow::Result;

/// Execute navigation query with clean interface
pub fn execute(
    query: &str,
    all_branches: bool,
    layer_filter: Option<String>,
    json_output: bool,
) -> Result<()> {
    implementation::execute_impl(query, all_branches, layer_filter, json_output)
}

// Everything else is private
mod implementation {
    use anyhow::{Context, Result};
    use colored::Colorize;
    use patina::indexer_refactored::{Confidence, Layer, Location, NavigationResponse, PatternIndexer};
    use patina::session::SessionManager;
    use serde_json::json;

    pub(super) fn execute_impl(
        query: &str,
        _all_branches: bool, // TODO: Implement cross-branch navigation
        layer_filter: Option<String>,
        json_output: bool,
    ) -> Result<()> {
        // Find project root
        let project_root = SessionManager::find_project_root()
            .context("Not in a Patina project directory. Run 'patina init' first.")?;

        // Initialize indexer
        let indexer = create_indexer(&project_root, json_output)?;

        // Perform navigation query
        let response = indexer.navigate(query);

        // Filter by layer if specified
        let filtered_response = filter_by_layer(response, layer_filter)?;

        // Display results
        if json_output {
            display_json_results(&filtered_response)?;
        } else {
            display_human_results(&filtered_response, query)?;
        }

        Ok(())
    }

    fn create_indexer(project_root: &std::path::Path, json_output: bool) -> Result<PatternIndexer> {
        let db_path = project_root.join(".patina/navigation.db");
        let enable_crdt = std::env::var("PATINA_ENABLE_CRDT").is_ok();

        if db_path.parent().map(|p| p.exists()).unwrap_or(false) {
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
                    Ok(indexer)
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
                            Ok(indexer)
                        }
                        Err(e2) => {
                            if !json_output {
                                eprintln!("Warning: Could not open any database: {e2}");
                                eprintln!("Using in-memory indexing only.");
                            }
                            Ok(PatternIndexer::new()?)
                        }
                    }
                }
            }
        } else {
            if !json_output {
                println!("Using in-memory pattern indexing (no database found)");
            }
            Ok(PatternIndexer::new()?)
        }
    }

    fn filter_by_layer(
        mut response: NavigationResponse,
        layer_filter: Option<String>,
    ) -> Result<NavigationResponse> {
        if let Some(layer_name) = layer_filter {
            let target_layer = match layer_name.to_lowercase().as_str() {
                "core" => Layer::Core,
                "surface" => Layer::Surface,
                "dust" => Layer::Dust,
                _ => anyhow::bail!("Invalid layer: {}. Must be one of: core, surface, dust", layer_name),
            };

            response.locations.retain(|loc| loc.layer == target_layer);
        }
        Ok(response)
    }

    fn display_json_results(response: &NavigationResponse) -> Result<()> {
        let output = json!({
            "query": response.query,
            "results": response.locations.iter().map(|loc| {
                json!({
                    "path": loc.path.to_string_lossy(),
                    "layer": format!("{:?}", loc.layer),
                    "confidence": format!("{:?}", loc.confidence),
                    "relevance": loc.relevance,
                    "git_state": loc.git_state.as_ref().map(|s| format!("{:?}", s)),
                })
            }).collect::<Vec<_>>(),
            "total": response.locations.len(),
        });

        println!("{}", serde_json::to_string_pretty(&output)?);
        Ok(())
    }

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

        // Display workspace hints if any
        if !response.workspace_hints.is_empty() {
            println!("{}", "Active Workspaces:".blue());
            for hint in &response.workspace_hints {
                println!("  • {} ({}): {}", 
                    hint.workspace_id.bright_blue(),
                    hint.branch.yellow(),
                    hint.relevance.dimmed()
                );
            }
            println!();
        }

        Ok(())
    }

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

        // Show git state if available
        if let Some(ref git_state) = location.git_state {
            println!("      Git: {}", format!("{:?}", git_state).dimmed());
        }
    }
}