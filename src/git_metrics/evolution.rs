//! Pattern evolution tracking - how patterns grow and change over time

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use crate::git_metrics::{PatternEvolution, EvolutionMilestone, ImpactLevel};

/// Track how patterns evolve over time
pub fn track_patterns(repo_path: &Path) -> Result<HashMap<String, PatternEvolution>> {
    let mut evolutions = HashMap::new();
    
    // Track patterns in layer/ directory
    let layer_path = repo_path.join("layer");
    if layer_path.exists() {
        track_layer_patterns(&layer_path, repo_path, &mut evolutions)?;
    }
    
    // Track patterns in src/ by analyzing imports and usage
    track_code_patterns(repo_path, &mut evolutions)?;
    
    Ok(evolutions)
}

/// Track patterns in the layer/ directory
fn track_layer_patterns(
    layer_path: &Path,
    repo_path: &Path,
    evolutions: &mut HashMap<String, PatternEvolution>,
) -> Result<()> {
    // Track each category
    for category in &["core", "surface", "dust", "sessions"] {
        let category_path = layer_path.join(category);
        if !category_path.exists() {
            continue;
        }
        
        // Find all markdown files
        for entry in std::fs::read_dir(&category_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let pattern_name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                let milestones = track_file_evolution(&path, repo_path)?;
                
                evolutions.insert(pattern_name.clone(), PatternEvolution {
                    name: pattern_name,
                    milestones,
                });
            }
        }
    }
    
    Ok(())
}

/// Track evolution of a specific file
fn track_file_evolution(file_path: &Path, repo_path: &Path) -> Result<Vec<EvolutionMilestone>> {
    let mut milestones = Vec::new();
    
    // Get commit history for this file
    let relative_path = file_path.strip_prefix(repo_path)
        .unwrap_or(file_path);
    
    let output = Command::new("git")
        .args(&["log", "--follow", "--format=%H|%aI|%s", "--", 
               relative_path.to_str().unwrap()])
        .current_dir(repo_path)
        .output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    for (i, line) in stdout.lines().enumerate() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() == 3 {
            let timestamp = DateTime::parse_from_rfc3339(parts[1])
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            
            let message = parts[2];
            let event = classify_evolution_event(message, i == 0);
            let impact = determine_impact_level(message);
            
            milestones.push(EvolutionMilestone {
                timestamp,
                event,
                impact,
            });
        }
    }
    
    // Reverse to get chronological order
    milestones.reverse();
    
    Ok(milestones)
}

/// Track patterns in code by analyzing usage
fn track_code_patterns(
    repo_path: &Path,
    evolutions: &mut HashMap<String, PatternEvolution>,
) -> Result<()> {
    // Look for common architectural patterns
    let patterns_to_track = vec![
        ("async", vec!["async fn", "tokio", ".await"]),
        ("error-handling", vec!["Result<", "anyhow", "thiserror"]),
        ("testing", vec!["#[test]", "#[cfg(test)]", "assert"]),
        ("builder-pattern", vec!["Builder", ".build()"]),
        ("iterator", vec![".iter()", ".map(", ".filter("]),
    ];
    
    for (pattern_name, indicators) in patterns_to_track {
        let milestones = track_pattern_adoption(repo_path, &indicators)?;
        
        if !milestones.is_empty() {
            evolutions.insert(pattern_name.to_string(), PatternEvolution {
                name: pattern_name.to_string(),
                milestones,
            });
        }
    }
    
    Ok(())
}

/// Track adoption of a specific pattern
fn track_pattern_adoption(
    repo_path: &Path,
    indicators: &[&str],
) -> Result<Vec<EvolutionMilestone>> {
    let mut milestones = Vec::new();
    
    for indicator in indicators {
        // Find first occurrence
        let output = Command::new("git")
            .args(&["log", "-S", indicator, "--reverse", "--format=%H|%aI|%s", "--max-count=1"])
            .current_dir(repo_path)
            .output()?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = stdout.lines().next() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() == 3 {
                let timestamp = DateTime::parse_from_rfc3339(parts[1])
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                
                milestones.push(EvolutionMilestone {
                    timestamp,
                    event: format!("First use of '{}'", indicator),
                    impact: ImpactLevel::Medium,
                });
            }
        }
    }
    
    // Sort by timestamp
    milestones.sort_by_key(|m| m.timestamp);
    
    Ok(milestones)
}

/// Classify the type of evolution event from commit message
fn classify_evolution_event(message: &str, is_first: bool) -> String {
    let message_lower = message.to_lowercase();
    
    if is_first {
        "Pattern introduced".to_string()
    } else if message_lower.contains("refactor") {
        "Refactored".to_string()
    } else if message_lower.contains("update") || message_lower.contains("improve") {
        "Enhanced".to_string()
    } else if message_lower.contains("fix") {
        "Bug fixed".to_string()
    } else if message_lower.contains("deprecate") {
        "Deprecated".to_string()
    } else if message_lower.contains("remove") || message_lower.contains("delete") {
        "Removed".to_string()
    } else {
        "Modified".to_string()
    }
}

/// Determine impact level from commit message
fn determine_impact_level(message: &str) -> ImpactLevel {
    let message_lower = message.to_lowercase();
    
    if message_lower.contains("breaking") || message_lower.contains("major") {
        ImpactLevel::Critical
    } else if message_lower.contains("feat") || message_lower.contains("add") {
        ImpactLevel::High
    } else if message_lower.contains("refactor") || message_lower.contains("improve") {
        ImpactLevel::Medium
    } else {
        ImpactLevel::Low
    }
}

/// Generate evolution report
pub fn generate_evolution_report(evolutions: &HashMap<String, PatternEvolution>) -> String {
    let mut report = String::from("# Pattern Evolution Report\n\n");
    
    // Sort patterns by number of milestones (most active first)
    let mut sorted_patterns: Vec<_> = evolutions.iter().collect();
    sorted_patterns.sort_by_key(|(_, e)| -(e.milestones.len() as i32));
    
    for (name, evolution) in sorted_patterns {
        report.push_str(&format!("## {}\n", name));
        report.push_str(&format!("Milestones: {}\n\n", evolution.milestones.len()));
        
        // Show timeline
        for milestone in &evolution.milestones {
            let impact_emoji = match milestone.impact {
                ImpactLevel::Critical => "🔴",
                ImpactLevel::High => "🟠",
                ImpactLevel::Medium => "🟡",
                ImpactLevel::Low => "🟢",
            };
            
            report.push_str(&format!("{} {} - {}\n",
                impact_emoji,
                milestone.timestamp.format("%Y-%m-%d"),
                milestone.event
            ));
        }
        
        report.push_str("\n");
    }
    
    report
}