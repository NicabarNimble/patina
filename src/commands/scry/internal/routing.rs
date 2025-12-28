//! Remote and multi-repo routing for scry
//!
//! Handles routing queries to mothership daemon and cross-repo searches.

use std::path::Path;

use anyhow::Result;

use patina::mothership;

use crate::commands::persona;

use super::super::{ScryOptions, ScryResult};
use super::enrichment::truncate_content;
use super::search::scry_text;

/// Execute scry via mothership daemon
pub fn execute_via_mothership(query: Option<&str>, options: &ScryOptions) -> Result<()> {
    let address = mothership::get_address().unwrap_or_else(|| "unknown".to_string());
    println!("🔮 Scry - Querying mothership at {}\n", address);

    // File-based queries not supported via mothership yet
    if options.file.is_some() {
        anyhow::bail!("File-based queries (--file) not supported via mothership. Run locally.");
    }

    let query = query.ok_or_else(|| anyhow::anyhow!("Query text required"))?;
    println!("Query: \"{}\"\n", query);

    // Build request
    let request = mothership::ScryRequest {
        query: query.to_string(),
        dimension: options.dimension.clone(),
        repo: options.repo.clone(),
        all_repos: options.all_repos,
        include_issues: options.include_issues,
        include_persona: options.include_persona,
        limit: options.limit,
        min_score: options.min_score,
    };

    // Execute query
    let response = mothership::scry(request)?;

    if response.results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    println!("Found {} results:\n", response.count);
    println!("{}", "─".repeat(60));

    for (i, result) in response.results.iter().enumerate() {
        let timestamp_display = if result.timestamp.is_empty() {
            String::new()
        } else {
            format!(" | {}", result.timestamp)
        };
        println!(
            "\n[{}] Score: {:.3} | {} | {}{}",
            i + 1,
            result.score,
            result.event_type,
            result.source_id,
            timestamp_display
        );
        println!("    {}", truncate_content(&result.content, 200));
    }

    println!("\n{}", "─".repeat(60));

    Ok(())
}

/// Execute query across all repos (current project + all reference repos)
pub fn execute_all_repos(query: Option<&str>, options: &ScryOptions) -> Result<()> {
    let query = query.ok_or_else(|| anyhow::anyhow!("Query required for --all-repos"))?;

    println!("Mode: All Repos (cross-project search)\n");
    println!("Query: \"{}\"\n", query);

    let mut all_results: Vec<(String, ScryResult)> = Vec::new();

    // 1. Query current project if we're in one
    let in_project = Path::new(".patina/data/patina.db").exists();
    if in_project {
        println!("📂 Searching current project...");
        let project_options = ScryOptions {
            repo: None,
            all_repos: false,
            ..options.clone()
        };
        match scry_text(query, &project_options) {
            Ok(results) => {
                println!("   Found {} results", results.len());
                for r in results {
                    all_results.push(("[PROJECT]".to_string(), r));
                }
            }
            Err(e) => {
                eprintln!("   ⚠️  Project search failed: {}", e);
            }
        }
    }

    // 2. Query all registered reference repos
    let repos = crate::commands::repo::list()?;
    for repo in repos {
        println!("📚 Searching {}...", repo.name);
        let repo_options = ScryOptions {
            repo: Some(repo.name.clone()),
            all_repos: false,
            ..options.clone()
        };
        match scry_text(query, &repo_options) {
            Ok(results) => {
                println!("   Found {} results", results.len());
                for r in results {
                    all_results.push((format!("[{}]", repo.name.to_uppercase()), r));
                }
            }
            Err(e) => {
                eprintln!("   ⚠️  {} search failed: {}", repo.name, e);
            }
        }
    }

    // 3. Query persona if enabled
    if options.include_persona {
        println!("🧠 Searching persona...");
        if let Ok(persona_results) = persona::query(query, options.limit, options.min_score, None) {
            println!("   Found {} results", persona_results.len());
            for p in persona_results {
                all_results.push((
                    "[PERSONA]".to_string(),
                    ScryResult {
                        id: 0,
                        content: p.content,
                        score: p.score,
                        event_type: p.source.clone(),
                        source_id: p.domains.join(", "),
                        timestamp: p.timestamp,
                    },
                ));
            }
        }
    }

    // 4. Sort by score and take top limit
    all_results.sort_by(|a, b| {
        b.1.score
            .partial_cmp(&a.1.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all_results.truncate(options.limit);

    println!();

    if all_results.is_empty() {
        println!("No results found across any repos.");
        return Ok(());
    }

    println!("Found {} results (combined):\n", all_results.len());
    println!("{}", "─".repeat(60));

    for (i, (source, result)) in all_results.iter().enumerate() {
        let timestamp_display = if result.timestamp.is_empty() {
            String::new()
        } else {
            format!(" | {}", result.timestamp)
        };
        println!(
            "\n[{}] {} Score: {:.3} | {} | {}{}",
            i + 1,
            source,
            result.score,
            result.event_type,
            result.source_id,
            timestamp_display
        );
        println!("    {}", truncate_content(&result.content, 200));
    }

    println!("\n{}", "─".repeat(60));

    Ok(())
}
