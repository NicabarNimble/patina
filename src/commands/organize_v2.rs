use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Args, Debug)]
pub struct OrganizeArgs {
    /// Analyze patterns without making changes
    #[arg(long, default_value = "false")]
    dry_run: bool,

    /// Show detailed analysis
    #[arg(long, short = 'v', default_value = "false")]
    verbose: bool,

    /// Days of history to analyze
    #[arg(long, default_value = "90")]
    days: u32,

    /// Focus on specific layer
    #[arg(long)]
    layer: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GitMetrics {
    path: PathBuf,
    layer: String,
    
    // Activity metrics from Git
    change_count: u32,           // How many times modified
    days_since_last_change: i64, // Freshness
    unique_authors: u32,          // How many people care about it
    survival_rate: f32,           // % of changes that survive 7+ days
    
    // Relationship metrics
    co_modified_with: Vec<String>, // Files that change together
    referenced_in_commits: u32,     // Mentioned in commit messages
    
    // Derived recommendation
    recommendation: PatternAction,
    rationale: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum PatternAction {
    PromoteToCore,    // Stable, valuable pattern
    KeepInSurface,    // Active development
    DemoteToDust,     // No longer active
    Archive,          // Dead code
    ExtractInsights,  // High-change file needs pattern extraction
    NoAction,
}

pub fn execute(args: OrganizeArgs) -> Result<()> {
    println!("🔍 Analyzing patterns using Git history ({} days)...\n", args.days);
    
    // Get all pattern files
    let patterns = discover_patterns(&args.layer)?;
    
    // Analyze each pattern using Git
    let mut metrics = Vec::new();
    for path in patterns {
        let metric = analyze_with_git(&path, args.days)?;
        metrics.push(metric);
    }
    
    // Sort by activity (most active first)
    metrics.sort_by(|a, b| {
        b.change_count.cmp(&a.change_count)
            .then(a.days_since_last_change.cmp(&b.days_since_last_change))
    });
    
    // Display results
    display_git_analysis(&metrics, args.verbose);
    
    if !args.dry_run {
        apply_git_recommendations(&metrics)?;
    } else {
        println!("\n🔍 Dry run - no changes made");
    }
    
    Ok(())
}

fn discover_patterns(layer_filter: &Option<String>) -> Result<Vec<PathBuf>> {
    let mut patterns = Vec::new();
    
    let layers = match layer_filter {
        Some(l) => vec![l.as_str()],
        None => vec!["core", "surface", "dust"],
    };
    
    for layer in layers {
        let layer_path = Path::new("layer").join(layer);
        if layer_path.exists() {
            for entry in std::fs::read_dir(&layer_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    patterns.push(path);
                }
            }
        }
    }
    
    Ok(patterns)
}

fn analyze_with_git(path: &Path, days: u32) -> Result<GitMetrics> {
    let path_str = path.to_string_lossy();
    let layer = path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    // Count changes in last N days
    let change_count = git_command(&[
        "log",
        &format!("--since={} days ago", days),
        "--oneline",
        "--",
        &path_str,
    ])?
    .lines()
    .count() as u32;
    
    // Days since last change
    let last_change = git_command(&[
        "log",
        "-1",
        "--format=%at",
        "--",
        &path_str,
    ])?;
    
    let days_since_last_change = if !last_change.trim().is_empty() {
        let timestamp = last_change.trim().parse::<i64>().unwrap_or(0);
        let last = DateTime::from_timestamp(timestamp, 0)
            .unwrap_or_else(|| Utc::now());
        (Utc::now() - last).num_days()
    } else {
        999 // Never changed
    };
    
    // Count unique authors
    let unique_authors = git_command(&[
        "log",
        &format!("--since={} days ago", days),
        "--format=%an",
        "--",
        &path_str,
    ])?
    .lines()
    .collect::<std::collections::HashSet<_>>()
    .len() as u32;
    
    // Calculate survival rate (% of changes that don't get reverted quickly)
    let survival_rate = calculate_survival_rate(path, days)?;
    
    // Find co-modified files
    let co_modified = find_co_modified_files(path, days)?;
    
    // Count references in commit messages
    let pattern_name = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    
    let referenced_in_commits = git_command(&[
        "log",
        &format!("--since={} days ago", days),
        "--grep",
        pattern_name,
        "--oneline",
    ])?
    .lines()
    .count() as u32;
    
    // Determine recommendation
    let (recommendation, rationale) = recommend_action(
        &layer,
        change_count,
        days_since_last_change,
        survival_rate,
        unique_authors,
    );
    
    Ok(GitMetrics {
        path: path.to_path_buf(),
        layer,
        change_count,
        days_since_last_change,
        unique_authors,
        survival_rate,
        co_modified_with: co_modified,
        referenced_in_commits,
        recommendation,
        rationale,
    })
}

fn git_command(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context("Failed to run git command")?;
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn calculate_survival_rate(path: &Path, days: u32) -> Result<f32> {
    // Get all commits that modified this file
    let commits = git_command(&[
        "log",
        &format!("--since={} days ago", days),
        "--format=%H",
        "--",
        &path.to_string_lossy(),
    ])?;
    
    let commit_list: Vec<&str> = commits.lines().collect();
    if commit_list.is_empty() {
        return Ok(100.0); // No changes means stable
    }
    
    let mut survived = 0;
    let mut total = 0;
    
    // Check if each change survived (wasn't reverted within 7 days)
    for commit in commit_list.iter() {
        total += 1;
        
        // Check if this commit's changes to the file still exist
        let diff = git_command(&[
            "diff",
            &format!("{}^", commit),
            commit,
            "--",
            &path.to_string_lossy(),
        ])?;
        
        if !diff.is_empty() {
            // The change had content
            survived += 1;
        }
    }
    
    if total == 0 {
        Ok(100.0)
    } else {
        Ok((survived as f32 / total as f32) * 100.0)
    }
}

fn find_co_modified_files(path: &Path, days: u32) -> Result<Vec<String>> {
    // Get commits that touched this file
    let commits = git_command(&[
        "log",
        &format!("--since={} days ago", days),
        "--format=%H",
        "--",
        &path.to_string_lossy(),
    ])?;
    
    let mut file_counts: HashMap<String, u32> = HashMap::new();
    
    // For each commit, find what other files were modified
    for commit in commits.lines() {
        let files = git_command(&[
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            "-r",
            commit,
        ])?;
        
        for file in files.lines() {
            if file != path.to_string_lossy() && file.starts_with("layer/") {
                *file_counts.entry(file.to_string()).or_insert(0) += 1;
            }
        }
    }
    
    // Return top co-modified files
    let mut co_modified: Vec<(String, u32)> = file_counts.into_iter().collect();
    co_modified.sort_by(|a, b| b.1.cmp(&a.1));
    
    Ok(co_modified.into_iter()
        .take(5)
        .map(|(file, _)| file)
        .collect())
}

fn recommend_action(
    layer: &str,
    change_count: u32,
    days_since_last_change: i64,
    survival_rate: f32,
    unique_authors: u32,
) -> (PatternAction, String) {
    // Core layer patterns
    if layer == "core" {
        if days_since_last_change > 180 && change_count == 0 {
            return (PatternAction::Archive, "No activity in 6+ months".to_string());
        }
        if change_count > 10 {
            return (PatternAction::ExtractInsights, 
                format!("High change rate ({} changes) - extract patterns", change_count));
        }
        // Core should be stable
        return (PatternAction::NoAction, "Stable core pattern".to_string());
    }
    
    // Surface layer patterns
    if layer == "surface" {
        // High activity with good survival = promote
        if change_count < 3 && days_since_last_change > 30 && survival_rate > 90.0 {
            return (PatternAction::PromoteToCore, 
                "Stable pattern with high survival rate".to_string());
        }
        
        // Dead pattern
        if days_since_last_change > 90 && change_count == 0 {
            return (PatternAction::DemoteToDust, 
                "No activity in 3+ months".to_string());
        }
        
        // High churn needs investigation
        if change_count > 20 && survival_rate < 70.0 {
            return (PatternAction::ExtractInsights,
                format!("High churn ({} changes, {:.0}% survival) - needs pattern extraction", 
                    change_count, survival_rate));
        }
        
        // Active development
        if change_count > 0 && days_since_last_change < 30 {
            return (PatternAction::KeepInSurface,
                "Active development pattern".to_string());
        }
    }
    
    // Dust layer
    if layer == "dust" {
        if days_since_last_change > 365 {
            return (PatternAction::Archive, "Dead for over a year".to_string());
        }
        // Dust can stay dust
        return (PatternAction::NoAction, "Historical reference".to_string());
    }
    
    (PatternAction::NoAction, "No clear action".to_string())
}

fn display_git_analysis(metrics: &[GitMetrics], verbose: bool) {
    println!("📊 Git-Based Pattern Analysis");
    println!("═══════════════════════════════");
    
    // Group by action
    let mut by_action: HashMap<String, Vec<&GitMetrics>> = HashMap::new();
    for metric in metrics {
        let key = format!("{:?}", metric.recommendation);
        by_action.entry(key)
            .or_default()
            .push(metric);
    }
    
    // Most active patterns
    println!("\n🔥 Most Active Patterns:");
    for metric in metrics.iter().take(5) {
        if metric.change_count > 0 {
            let name = metric.path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            println!("  {} ({} layer): {} changes, {} days ago, {:.0}% survival",
                name, metric.layer, metric.change_count, 
                metric.days_since_last_change, metric.survival_rate);
        }
    }
    
    // Recommendations
    println!("\n🎯 Recommendations:");
    for (action_key, patterns) in by_action {
        if patterns.is_empty() { continue; }
        
        let action_str = match action_key.as_str() {
            "PromoteToCore" => "📈 Promote to Core",
            "KeepInSurface" => "✓ Keep in Surface",
            "DemoteToDust" => "📉 Demote to Dust",
            "Archive" => "🗄 Archive",
            "ExtractInsights" => "🔍 Extract Insights",
            "NoAction" => "◯ No Action",
            _ => "? Unknown",
        };
        
        println!("\n{} ({}):", action_str, patterns.len());
        
        for pattern in patterns.iter().take(if verbose { 100 } else { 3 }) {
            let name = pattern.path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            println!("  • {} - {}", name, pattern.rationale);
            
            if verbose && !pattern.co_modified_with.is_empty() {
                println!("    Co-modified with: {}", pattern.co_modified_with.join(", "));
            }
        }
    }
    
    // Insights
    println!("\n💡 Insights:");
    
    // Find patterns that change together
    let mut co_modification_pairs: HashMap<String, u32> = HashMap::new();
    for metric in metrics {
        for co_file in &metric.co_modified_with {
            let pair = format!("{} ↔ {}", 
                metric.path.file_stem().and_then(|s| s.to_str()).unwrap_or("?"),
                Path::new(co_file).file_stem().and_then(|s| s.to_str()).unwrap_or("?")
            );
            *co_modification_pairs.entry(pair).or_insert(0) += 1;
        }
    }
    
    if !co_modification_pairs.is_empty() {
        println!("  Patterns that change together:");
        for (pair, _count) in co_modification_pairs.iter().take(3) {
            println!("    {}", pair);
        }
    }
}

fn apply_git_recommendations(metrics: &[GitMetrics]) -> Result<()> {
    println!("\n📝 Applying recommendations based on Git history...");
    
    let mut moved = 0;
    let mut extracted = 0;
    
    for metric in metrics {
        match metric.recommendation {
            PatternAction::PromoteToCore => {
                promote_pattern(&metric.path, "core")?;
                moved += 1;
            }
            PatternAction::DemoteToDust => {
                promote_pattern(&metric.path, "dust")?;
                moved += 1;
            }
            PatternAction::Archive => {
                archive_pattern(&metric.path)?;
                moved += 1;
            }
            PatternAction::ExtractInsights => {
                create_insight_task(&metric)?;
                extracted += 1;
            }
            _ => {}
        }
    }
    
    if moved > 0 {
        println!("✓ Moved {} patterns based on Git activity", moved);
    }
    if extracted > 0 {
        println!("✓ Created {} insight extraction tasks", extracted);
    }
    
    Ok(())
}

fn promote_pattern(from: &Path, to_layer: &str) -> Result<()> {
    let file_name = from.file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;
    let to_path = Path::new("layer").join(to_layer).join(file_name);
    
    std::fs::create_dir_all(to_path.parent().unwrap())?;
    std::fs::rename(from, &to_path)?;
    
    println!("  Moved {} to {}", from.display(), to_layer);
    Ok(())
}

fn archive_pattern(path: &Path) -> Result<()> {
    let archive_dir = Path::new("layer/dust/archived");
    std::fs::create_dir_all(archive_dir)?;
    
    let file_name = path.file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;
    let archive_path = archive_dir.join(file_name);
    
    std::fs::rename(path, &archive_path)?;
    println!("  Archived {}", path.display());
    Ok(())
}

fn create_insight_task(metric: &GitMetrics) -> Result<()> {
    // Create a task to extract patterns from high-churn files
    let task_name = format!(
        "extract-patterns-{}.md",
        metric.path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown")
    );
    
    let task_content = format!(
        "# Pattern Extraction Needed\n\n\
        File: {}\n\
        Changes: {}\n\
        Survival Rate: {:.0}%\n\
        Authors: {}\n\n\
        ## Why\n\
        This pattern has high change frequency but low survival rate, \
        suggesting it contains multiple concepts that need separation.\n\n\
        ## Co-modified Files\n{}\n\n\
        ## Next Steps\n\
        1. Review recent changes\n\
        2. Identify stable vs volatile parts\n\
        3. Extract stable patterns to separate files\n",
        metric.path.display(),
        metric.change_count,
        metric.survival_rate,
        metric.unique_authors,
        metric.co_modified_with.join("\n")
    );
    
    let task_path = Path::new(".patina/tasks").join(task_name);
    std::fs::create_dir_all(task_path.parent().unwrap())?;
    std::fs::write(&task_path, task_content)?;
    
    println!("  Created insight task: {}", task_path.display());
    Ok(())
}