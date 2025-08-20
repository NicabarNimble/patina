use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use colored::*;
use std::collections::HashMap;
use std::process::Command;

pub fn execute(pattern: &str) -> Result<()> {
    println!("{}", format!("\n🔍 Tracing pattern: {}", pattern).bright_cyan());
    
    // Find pattern mentions in docs
    let doc_timeline = trace_in_docs(pattern)?;
    
    // Find pattern implementations in code
    let code_timeline = trace_in_code(pattern)?;
    
    // Analyze survival
    let survival = analyze_survival(pattern)?;
    
    // Display timeline
    display_timeline(&doc_timeline, &code_timeline)?;
    
    // Display current status
    display_status(pattern, &survival)?;
    
    Ok(())
}

#[derive(Debug)]
struct TimelineEntry {
    date: String,
    file: String,
    action: String,
    commit: String,
}

fn trace_in_docs(pattern: &str) -> Result<Vec<TimelineEntry>> {
    // Search for pattern mentions in layer/ docs
    let output = Command::new("git")
        .args(&["log", "--all", "--grep", pattern, "--pretty=format:%H|%ai|%s", "--", "layer/"])
        .output()
        .context("Failed to search git history")?;
    
    let mut entries = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    for line in stdout.lines() {
        if line.is_empty() { continue; }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 3 {
            // Get the files changed in this commit
            let files_output = Command::new("git")
                .args(&["diff-tree", "--no-commit-id", "--name-only", "-r", parts[0]])
                .output()?;
            
            let files = String::from_utf8_lossy(&files_output.stdout);
            for file in files.lines() {
                if file.starts_with("layer/") && file.ends_with(".md") {
                    entries.push(TimelineEntry {
                        date: parts[1].to_string(),
                        file: file.to_string(),
                        action: format!("Doc: {}", parts[2]),
                        commit: parts[0][..8].to_string(),
                    });
                }
            }
        }
    }
    
    // Also search for pattern in current docs
    let grep_output = Command::new("grep")
        .args(&["-r", "-l", pattern, "layer/"])
        .output()
        .context("Failed to search current docs")?;
    
    let current_docs = String::from_utf8_lossy(&grep_output.stdout);
    for doc in current_docs.lines() {
        if !doc.is_empty() {
            // Get last modification date
            let log_output = Command::new("git")
                .args(&["log", "-1", "--pretty=format:%ai|%H", "--", doc])
                .output()?;
            
            let log = String::from_utf8_lossy(&log_output.stdout);
            if let Some(line) = log.lines().next() {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 2 {
                    entries.push(TimelineEntry {
                        date: parts[0].to_string(),
                        file: doc.to_string(),
                        action: "Currently documented".to_string(),
                        commit: parts[1][..8].to_string(),
                    });
                }
            }
        }
    }
    
    entries.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(entries)
}

fn trace_in_code(pattern: &str) -> Result<Vec<TimelineEntry>> {
    // Search for pattern implementations in code
    let output = Command::new("git")
        .args(&["log", "--all", "-S", pattern, "--pretty=format:%H|%ai|%s", "--", "src/", "modules/"])
        .output()
        .context("Failed to search code history")?;
    
    let mut entries = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    for line in stdout.lines() {
        if line.is_empty() { continue; }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 3 {
            // Get the files changed
            let files_output = Command::new("git")
                .args(&["diff-tree", "--no-commit-id", "--name-only", "-r", parts[0]])
                .output()?;
            
            let files = String::from_utf8_lossy(&files_output.stdout);
            for file in files.lines() {
                if (file.starts_with("src/") || file.starts_with("modules/")) 
                    && (file.ends_with(".rs") || file.ends_with(".go")) {
                    entries.push(TimelineEntry {
                        date: parts[1].to_string(),
                        file: file.to_string(),
                        action: format!("Code: {}", parts[2]),
                        commit: parts[0][..8].to_string(),
                    });
                }
            }
        }
    }
    
    entries.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(entries)
}

fn analyze_survival(pattern: &str) -> Result<SurvivalStats> {
    // Find current files mentioning the pattern
    let grep_output = Command::new("grep")
        .args(&["-r", "-l", pattern, "src/", "modules/"])
        .output()
        .context("Failed to search current code")?;
    
    let current_files = String::from_utf8_lossy(&grep_output.stdout);
    let mut surviving_files = Vec::new();
    let mut total_files = 0;
    
    for file in current_files.lines() {
        if !file.is_empty() {
            total_files += 1;
            // Check how long since last modification
            let log_output = Command::new("git")
                .args(&["log", "-1", "--pretty=format:%ai", "--", file])
                .output()?;
            
            let last_modified = String::from_utf8_lossy(&log_output.stdout);
            surviving_files.push((file.to_string(), last_modified.to_string()));
        }
    }
    
    // Count historical implementations that were deleted
    let deleted_output = Command::new("git")
        .args(&["log", "--diff-filter=D", "--summary", "--pretty=format:", "--", "src/", "modules/"])
        .output()?;
    
    let deleted = String::from_utf8_lossy(&deleted_output.stdout);
    let deleted_count = deleted.lines()
        .filter(|l| l.contains("delete mode"))
        .count();
    
    Ok(SurvivalStats {
        surviving: surviving_files.len(),
        total: total_files + deleted_count,
        files: surviving_files,
    })
}

#[derive(Debug)]
struct SurvivalStats {
    surviving: usize,
    total: usize,
    files: Vec<(String, String)>,
}

fn display_timeline(docs: &[TimelineEntry], code: &[TimelineEntry]) -> Result<()> {
    println!("\n{}", "📅 Timeline:".bright_yellow());
    
    // Merge and sort all entries
    let mut all_entries = Vec::new();
    for entry in docs {
        all_entries.push((entry, "doc"));
    }
    for entry in code {
        all_entries.push((entry, "code"));
    }
    all_entries.sort_by(|a, b| a.0.date.cmp(&b.0.date));
    
    for (entry, kind) in all_entries {
        let icon = if kind == "doc" { "📝" } else { "💻" };
        let color = if kind == "doc" {
            entry.action.bright_blue()
        } else {
            entry.action.bright_green()
        };
        
        println!("{} {} [{}]: {}",
            icon,
            &entry.date[..10],
            entry.commit.bright_black(),
            color
        );
        println!("     └─ {}", entry.file.bright_black());
    }
    
    Ok(())
}

fn display_status(pattern: &str, survival: &SurvivalStats) -> Result<()> {
    println!("\n{}", "📊 Current Status:".bright_yellow());
    
    let survival_rate = if survival.total > 0 {
        (survival.surviving as f64 / survival.total as f64) * 100.0
    } else {
        0.0
    };
    
    let status = if survival_rate > 80.0 {
        "Thriving".bright_green()
    } else if survival_rate > 50.0 {
        "Stable".bright_yellow()
    } else if survival.surviving > 0 {
        "Struggling".bright_red()
    } else {
        "Not Found".bright_black()
    };
    
    println!("Pattern: {} | Status: {} | Survival: {:.0}% ({}/{})",
        pattern.bright_cyan(),
        status,
        survival_rate,
        survival.surviving,
        survival.total
    );
    
    if !survival.files.is_empty() {
        println!("\n{}", "Active implementations:".bright_black());
        for (file, last_mod) in &survival.files[..survival.files.len().min(5)] {
            println!("  • {} (last modified: {})", 
                file.bright_white(),
                &last_mod[..10]
            );
        }
    }
    
    Ok(())
}