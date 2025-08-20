//! Code survival metrics - tracking how long code lives and why it dies

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Code survival analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivalAnalysis {
    pub file: PathBuf,
    pub birth_date: DateTime<Utc>,
    pub death_date: Option<DateTime<Utc>>,
    pub lifespan_days: i64,
    pub survival_rate: f64,
    pub death_cause: Option<DeathCause>,
    pub resurrection_count: usize,  // Times file was deleted then recreated
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeathCause {
    Refactor,
    Obsolete,
    Moved,
    Merged,
    Experimental,  // Experiment that didn't work out
    Unknown,
}

/// Track code survival across the repository
pub fn analyze_survival(repo_path: &PathBuf) -> Result<Vec<SurvivalAnalysis>> {
    let mut survival_data = Vec::new();
    
    // Get all files that have ever existed
    let output = Command::new("git")
        .args(&["log", "--all", "--pretty=format:", "--name-status"])
        .current_dir(repo_path)
        .output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut file_history: HashMap<PathBuf, FileHistory> = HashMap::new();
    
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            let status = parts[0];
            let file = PathBuf::from(parts[1]);
            
            let history = file_history.entry(file.clone()).or_insert_with(|| {
                FileHistory {
                    path: file,
                    events: Vec::new(),
                }
            });
            
            match status {
                "A" => history.events.push(FileEvent::Added),
                "D" => history.events.push(FileEvent::Deleted),
                "M" => history.events.push(FileEvent::Modified),
                "R" => history.events.push(FileEvent::Renamed),
                _ => {}
            }
        }
    }
    
    // Analyze each file's survival
    for (path, history) in file_history {
        let survival = analyze_file_survival(&path, &history, repo_path)?;
        survival_data.push(survival);
    }
    
    // Sort by survival rate
    survival_data.sort_by(|a, b| b.survival_rate.partial_cmp(&a.survival_rate).unwrap());
    
    Ok(survival_data)
}

#[derive(Debug)]
struct FileHistory {
    path: PathBuf,
    events: Vec<FileEvent>,
}

#[derive(Debug)]
enum FileEvent {
    Added,
    Modified,
    Deleted,
    Renamed,
}

fn analyze_file_survival(
    path: &PathBuf,
    history: &FileHistory,
    repo_path: &PathBuf,
) -> Result<SurvivalAnalysis> {
    // Get first and last commit dates for this file
    let first_commit = get_first_commit_date(path, repo_path)?;
    let last_commit = get_last_commit_date(path, repo_path)?;
    
    // Check if file still exists
    let still_exists = repo_path.join(path).exists();
    let death_date = if still_exists {
        None
    } else {
        Some(last_commit)
    };
    
    // Calculate lifespan
    let lifespan_days = if let Some(death) = death_date {
        (death - first_commit).num_days()
    } else {
        (Utc::now() - first_commit).num_days()
    };
    
    // Calculate survival rate (0.0 to 1.0)
    let max_lifespan_days = (Utc::now() - first_commit).num_days();
    let survival_rate = if still_exists {
        1.0
    } else {
        (lifespan_days as f64) / (max_lifespan_days as f64).max(1.0)
    };
    
    // Count resurrections (deleted then added again)
    let mut resurrection_count = 0;
    let mut was_deleted = false;
    for event in &history.events {
        match event {
            FileEvent::Deleted => was_deleted = true,
            FileEvent::Added if was_deleted => {
                resurrection_count += 1;
                was_deleted = false;
            }
            _ => {}
        }
    }
    
    // Determine death cause if file is gone
    let death_cause = if !still_exists {
        determine_death_cause(path, &history.events)
    } else {
        None
    };
    
    Ok(SurvivalAnalysis {
        file: path.clone(),
        birth_date: first_commit,
        death_date,
        lifespan_days,
        survival_rate,
        death_cause,
        resurrection_count,
    })
}

fn get_first_commit_date(file: &PathBuf, repo_path: &PathBuf) -> Result<DateTime<Utc>> {
    let output = Command::new("git")
        .args(&["log", "--reverse", "--format=%aI", "--", file.to_str().unwrap()])
        .current_dir(repo_path)
        .output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap_or("");
    
    DateTime::parse_from_rfc3339(first_line)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| Ok(Utc::now()))
}

fn get_last_commit_date(file: &PathBuf, repo_path: &PathBuf) -> Result<DateTime<Utc>> {
    let output = Command::new("git")
        .args(&["log", "-1", "--format=%aI", "--", file.to_str().unwrap()])
        .current_dir(repo_path)
        .output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap_or("");
    
    DateTime::parse_from_rfc3339(first_line)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| Ok(Utc::now()))
}

fn determine_death_cause(path: &PathBuf, events: &[FileEvent]) -> Option<DeathCause> {
    // Simple heuristics for death cause
    let path_str = path.to_string_lossy();
    
    if path_str.contains("experiment") || path_str.contains("test") {
        Some(DeathCause::Experimental)
    } else if path_str.contains("old") || path_str.contains("deprecated") {
        Some(DeathCause::Obsolete)
    } else if events.iter().any(|e| matches!(e, FileEvent::Renamed)) {
        Some(DeathCause::Moved)
    } else {
        Some(DeathCause::Unknown)
    }
}

/// Generate survival report
pub fn generate_survival_report(analyses: &[SurvivalAnalysis]) -> String {
    let mut report = String::from("# Code Survival Report\n\n");
    
    // Statistics
    let total_files = analyses.len();
    let alive_files = analyses.iter().filter(|a| a.death_date.is_none()).count();
    let avg_lifespan = analyses.iter().map(|a| a.lifespan_days).sum::<i64>() as f64 / total_files as f64;
    let resurrection_files = analyses.iter().filter(|a| a.resurrection_count > 0).count();
    
    report.push_str(&format!("## Statistics\n"));
    report.push_str(&format!("- Total files tracked: {}\n", total_files));
    report.push_str(&format!("- Currently alive: {} ({:.1}%)\n", 
        alive_files, (alive_files as f64 / total_files as f64) * 100.0));
    report.push_str(&format!("- Average lifespan: {:.0} days\n", avg_lifespan));
    report.push_str(&format!("- Files with resurrections: {}\n\n", resurrection_files));
    
    // Top survivors
    report.push_str("## Top Survivors\n");
    for analysis in analyses.iter().take(10) {
        if analysis.death_date.is_none() {
            report.push_str(&format!("- {} ({} days, {:.0}% survival rate)\n",
                analysis.file.display(),
                analysis.lifespan_days,
                analysis.survival_rate * 100.0
            ));
        }
    }
    
    // Recent deaths
    report.push_str("\n## Recent Deaths\n");
    let mut deaths: Vec<_> = analyses.iter()
        .filter(|a| a.death_date.is_some())
        .collect();
    deaths.sort_by_key(|a| a.death_date);
    
    for analysis in deaths.iter().rev().take(10) {
        report.push_str(&format!("- {} (lived {} days, cause: {:?})\n",
            analysis.file.display(),
            analysis.lifespan_days,
            analysis.death_cause.as_ref().unwrap_or(&DeathCause::Unknown)
        ));
    }
    
    // Resurrection champions
    report.push_str("\n## Resurrection Champions\n");
    let mut resurrections: Vec<_> = analyses.iter()
        .filter(|a| a.resurrection_count > 0)
        .collect();
    resurrections.sort_by_key(|a| a.resurrection_count);
    resurrections.reverse();
    
    for analysis in resurrections.iter().take(5) {
        report.push_str(&format!("- {} (resurrected {} times)\n",
            analysis.file.display(),
            analysis.resurrection_count
        ));
    }
    
    report
}