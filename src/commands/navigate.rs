use anyhow::{Context, Result};
use colored::Colorize;
use patina::indexer::{Confidence, Layer, Location, NavigationResponse, PatternIndexer};
use patina::session::SessionManager;
use rusqlite;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

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

    // Check if index is stale before re-indexing
    if should_reindex(&layer_path, json_output)? {
        if !json_output {
            println!("Index is stale, refreshing...");
        }
        indexer.index_directory(&layer_path)?;
    } else if !json_output {
        // Skip the verbose message for clean output
        // println!("Index is current, skipping re-index");
    }

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

    // Track pattern usage in SQLite
    track_pattern_usage(&filtered_response, &project_root);

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

/// Track pattern usage in SQLite for evolution tracking
fn track_pattern_usage(response: &NavigationResponse, project_root: &Path) {
    // Get current session ID from Git tags
    let session_id = get_current_session_id().unwrap_or_else(|| "no-session".to_string());
    
    // Skip if no patterns found
    if response.locations.is_empty() {
        return;
    }
    
    // Open SQLite connection
    let db_path = project_root.join(".patina/navigation.db");
    if !db_path.exists() {
        return;
    }
    
    // Use rusqlite to track usage
    if let Ok(conn) = rusqlite::Connection::open(&db_path) {
        for location in &response.locations {
            // Extract pattern ID from path (file name without extension)
            let pattern_id = location.path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            // Insert usage record (ignore errors silently)
            let _ = conn.execute(
                "INSERT INTO pattern_usage (pattern_id, session_id, domain) 
                 VALUES (?1, ?2, 'general')",
                rusqlite::params![pattern_id, session_id],
            );
        }
    }
}

/// Get current session ID from Git tags
fn get_current_session_id() -> Option<String> {
    // Look for most recent session-*-start tag
    let output = Command::new("git")
        .args(["describe", "--tags", "--match", "session-*-start", "--abbrev=0"])
        .output()
        .ok()?;
    
    if output.status.success() {
        let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Extract session ID from tag (session-YYYYMMDD-HHMMSS-start)
        tag.strip_prefix("session-")
            .and_then(|s| s.strip_suffix("-start"))
            .map(|s| s.to_string())
    } else {
        None
    }
}

/// Check if the index needs to be refreshed
fn should_reindex(layer_path: &Path, json_output: bool) -> Result<bool> {
    // Get the project root to find the database
    let db_path = layer_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid layer path"))?
        .join(".patina/navigation.db");
    
    // If database doesn't exist, we need to index
    if !db_path.exists() {
        return Ok(true);
    }
    
    // Get the last index time from the database
    let conn = rusqlite::Connection::open(&db_path)?;
    let last_index_time: Option<i64> = conn
        .query_row(
            "SELECT CAST(strftime('%s', MAX(last_indexed)) AS INTEGER) FROM documents",
            [],
            |row| row.get(0),
        )
        .ok();
    
    // If no documents indexed yet, need to index
    let Some(last_index_timestamp) = last_index_time else {
        return Ok(true);
    };
    
    // Check if any markdown files in layer directory are newer than last index
    let needs_reindex = check_directory_modified_since(layer_path, last_index_timestamp)?;
    
    if needs_reindex && !json_output {
        // Count how many files changed
        let changed_count = count_modified_files(layer_path, last_index_timestamp)?;
        if changed_count > 0 {
            println!("Found {} modified files since last index", changed_count);
        }
    }
    
    Ok(needs_reindex)
}

/// Recursively check if any files in directory were modified after timestamp
fn check_directory_modified_since(dir: &Path, timestamp: i64) -> Result<bool> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        
        if metadata.is_dir() {
            // Skip .git and other hidden directories
            if path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }
            
            if check_directory_modified_since(&path, timestamp)? {
                return Ok(true);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            // Check if markdown file was modified
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                    if duration.as_secs() as i64 > timestamp {
                        return Ok(true);
                    }
                }
            }
        }
    }
    
    Ok(false)
}

/// Count how many files were modified since timestamp
fn count_modified_files(dir: &Path, timestamp: i64) -> Result<usize> {
    let mut count = 0;
    
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        
        if metadata.is_dir() {
            // Skip hidden directories
            if path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }
            
            count += count_modified_files(&path, timestamp)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            // Check if markdown file was modified
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                    if duration.as_secs() as i64 > timestamp {
                        count += 1;
                    }
                }
            }
        }
    }
    
    Ok(count)
}