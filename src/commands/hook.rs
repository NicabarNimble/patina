//! Hook commands for LLM integration
//! 
//! These commands are called by Claude Code hooks (or other LLM hooks)
//! to integrate Git tracking with session management.

use anyhow::{Context, Result};
use serde_json::Value;
use std::io::Read;
use std::path::Path;
use std::process::Command;

/// Process hook events from LLMs
pub fn process_hook(event: &str) -> Result<()> {
    match event {
        "on-stop" => on_stop(),
        "on-modified" => on_file_modified(),
        "on-before-edit" => on_before_edit(),
        "on-session-start" => on_session_start(),
        _ => {
            eprintln!("Unknown hook event: {}", event);
            Ok(())
        }
    }
}

/// Called when Claude/LLM stops - commit session updates
pub fn on_stop() -> Result<()> {
    // Read hook input from stdin
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    
    let data: Value = serde_json::from_str(&input).unwrap_or_else(|_| {
        serde_json::json!({})
    });
    
    let session_id = data["session_id"].as_str().unwrap_or("unknown");
    let cwd = data["cwd"].as_str().unwrap_or(".");
    
    // Change to project directory
    std::env::set_current_dir(cwd)?;
    
    // Check for active session files in any LLM directory
    let session_dirs = vec![".claude/context", ".gemini/context", ".llm/context"];
    let mut found_session = false;
    
    for dir in &session_dirs {
        let session_file = Path::new(dir).join("active-session.md");
        if session_file.exists() {
            found_session = true;
            
            // Add timestamp to session
            let timestamp = format!("\n_Auto-checkpoint at {} (session: {})_\n", 
                chrono::Local::now().format("%H:%M"), 
                &session_id[0..8.min(session_id.len())]);
            
            std::fs::OpenOptions::new()
                .append(true)
                .open(&session_file)?
                .write_all(timestamp.as_bytes())?;
            
            // Git add the session directory
            Command::new("git")
                .args(&["add", dir])
                .output()
                .context("Failed to git add session files")?;
        }
    }
    
    if found_session {
        // Commit session changes
        let commit_msg = format!("session: checkpoint {}", &session_id[0..8.min(session_id.len())]);
        let output = Command::new("git")
            .args(&["commit", "-m", &commit_msg])
            .output()
            .context("Failed to git commit")?;
        
        if output.status.success() {
            println!("✓ Session checkpoint committed");
        } else {
            // No changes to commit is okay
            if !String::from_utf8_lossy(&output.stdout).contains("nothing to commit") {
                eprintln!("Git commit failed: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
    }
    
    // Also check for pattern changes
    if Path::new("layer/").exists() {
        let status = Command::new("git")
            .args(&["status", "--porcelain", "layer/"])
            .output()?;
        
        if !status.stdout.is_empty() {
            Command::new("git")
                .args(&["add", "layer/"])
                .output()?;
            
            Command::new("git")
                .args(&["commit", "-m", "patterns: auto-update"])
                .output()
                .ok(); // Don't fail if nothing to commit
        }
    }
    
    Ok(())
}

/// Called after file modification - track co-modifications
pub fn on_file_modified() -> Result<()> {
    // Read hook input
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    
    let data: Value = serde_json::from_str(&input).unwrap_or_else(|_| {
        serde_json::json!({})
    });
    
    let file_path = data["tool_input"]["file_path"].as_str();
    let session_id = data["session_id"].as_str().unwrap_or("unknown");
    
    if let Some(file) = file_path {
        // Track co-modification
        track_comodification(session_id, file)?;
        
        // Check file survival and add to session
        if let Ok(survival) = check_file_survival(file) {
            if survival.is_old {
                append_to_active_session(&format!(
                    "- Modified `{}` (survived {})",
                    file, survival.age
                ))?;
            }
        }
    }
    
    Ok(())
}

/// Called before file edit - check survival and warn
pub fn on_before_edit() -> Result<()> {
    // Read hook input
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    
    let data: Value = serde_json::from_str(&input).unwrap_or_else(|_| {
        serde_json::json!({})
    });
    
    let tool = data["tool"].as_str().unwrap_or("");
    let file_path = data["tool_input"]["file_path"].as_str();
    
    // Only check for edit operations
    if !["Edit", "MultiEdit", "Write"].contains(&tool) {
        return Ok(());
    }
    
    if let Some(file) = file_path {
        if let Ok(survival) = check_file_survival(file) {
            if survival.months > 3 {
                eprintln!("⚠️  PATINA WARNING: {}", file);
                eprintln!("   Survived: {}", survival.age);
                eprintln!("   Commits: {}", survival.commits);
                eprintln!("   This is a stable pattern - modify carefully!");
                
                // Show co-modified files
                if let Ok(comodified) = get_comodified_files(file) {
                    if !comodified.is_empty() {
                        eprintln!("   Often changes with:");
                        for file in comodified.iter().take(3) {
                            eprintln!("     - {}", file);
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Called when session starts - create branch if needed
pub fn on_session_start() -> Result<()> {
    // Read hook input
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    
    let data: Value = serde_json::from_str(&input).unwrap_or_else(|_| {
        serde_json::json!({})
    });
    
    let session_id = data["session_id"].as_str().unwrap_or("unknown");
    let cwd = data["cwd"].as_str().unwrap_or(".");
    
    std::env::set_current_dir(cwd)?;
    
    // Check current branch
    let output = Command::new("git")
        .args(&["branch", "--show-current"])
        .output()?;
    
    let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    // Create session branch if on main/master
    if current_branch == "main" || current_branch == "master" {
        let session_branch = format!("session/{}", &session_id[0..8.min(session_id.len())]);
        
        let output = Command::new("git")
            .args(&["checkout", "-b", &session_branch])
            .output()?;
        
        if output.status.success() {
            println!("✓ Created session branch: {}", session_branch);
        }
    }
    
    Ok(())
}

// Helper functions

#[derive(Debug)]
struct SurvivalMetrics {
    age: String,
    commits: usize,
    months: usize,
    is_old: bool,
}

fn check_file_survival(file: &str) -> Result<SurvivalMetrics> {
    // Get file age
    let age_output = Command::new("git")
        .args(&["log", "-1", "--format=%ar", "--", file])
        .output()?;
    
    let age = String::from_utf8_lossy(&age_output.stdout).trim().to_string();
    
    // Get commit count
    let commits_output = Command::new("git")
        .args(&["log", "--oneline", "--", file])
        .output()?;
    
    let commits = String::from_utf8_lossy(&commits_output.stdout)
        .lines()
        .count();
    
    // Parse months from age
    let months = if age.contains("year") {
        12 // At least 12 months
    } else if age.contains("month") {
        age.split_whitespace()
            .next()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1)
    } else {
        0
    };
    
    let is_old = age.contains("month") || age.contains("year");
    
    Ok(SurvivalMetrics {
        age: if age.is_empty() { "new".to_string() } else { age },
        commits,
        months,
        is_old,
    })
}

fn track_comodification(session_id: &str, file: &str) -> Result<()> {
    // Create .patina directory if needed
    std::fs::create_dir_all(".patina")?;
    
    // Append to comodification log
    let log_entry = format!("{}:{}:{}\n", 
        chrono::Utc::now().to_rfc3339(),
        session_id,
        file
    );
    
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(".patina/comodifications.log")?;
    
    f.write_all(log_entry.as_bytes())?;
    
    Ok(())
}

fn get_comodified_files(file: &str) -> Result<Vec<String>> {
    // Get files that often change with this file
    let output = Command::new("git")
        .args(&["log", "--name-only", "--pretty=format:", "--", file])
        .output()?;
    
    let mut file_counts = std::collections::HashMap::new();
    
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if !line.is_empty() && line != file {
            *file_counts.entry(line.to_string()).or_insert(0) += 1;
        }
    }
    
    // Sort by frequency
    let mut files: Vec<_> = file_counts.into_iter().collect();
    files.sort_by(|a, b| b.1.cmp(&a.1));
    
    Ok(files.into_iter()
        .take(5)
        .map(|(file, _)| file)
        .collect())
}

fn append_to_active_session(line: &str) -> Result<()> {
    // Find active session file
    let session_dirs = vec![".claude/context", ".gemini/context", ".llm/context"];
    
    for dir in &session_dirs {
        let session_file = Path::new(dir).join("active-session.md");
        if session_file.exists() {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(session_file)?;
            
            writeln!(f, "{}", line)?;
            return Ok(());
        }
    }
    
    Ok(())
}

use std::io::Write;