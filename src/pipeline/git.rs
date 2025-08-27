use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct GitMetrics {
    pub repository: String,
    pub branch: String,
    pub commit: String,
    pub analyzed_at: DateTime<Utc>,
    pub file_metrics: HashMap<String, FileMetrics>,
    pub author_stats: HashMap<String, AuthorStats>,
    pub total_commits: usize,
    pub total_lines: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetrics {
    pub path: String,
    pub commits: usize,
    pub authors: Vec<String>,
    pub last_modified: DateTime<Utc>,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub churn_rate: f64,  // (added + removed) / commits
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorStats {
    pub name: String,
    pub commits: usize,
    pub files_touched: usize,
    pub lines_added: usize,
    pub lines_removed: usize,
}

/// Analyze Git repository metrics
pub fn analyze_git(repo: &Path) -> Result<GitMetrics> {
    // Get current branch
    let branch = get_current_branch(repo)?;
    
    // Get current commit
    let commit = get_current_commit(repo)?;
    
    // Get file-level metrics
    let file_metrics = get_file_metrics(repo)?;
    
    // Get author statistics
    let author_stats = get_author_stats(repo)?;
    
    // Count total commits
    let total_commits = count_total_commits(repo)?;
    
    // Calculate total lines
    let total_lines = file_metrics.values()
        .map(|f| f.lines_added.saturating_sub(f.lines_removed))
        .sum();
    
    Ok(GitMetrics {
        repository: repo.display().to_string(),
        branch,
        commit,
        analyzed_at: Utc::now(),
        file_metrics,
        author_stats,
        total_commits,
        total_lines,
    })
}

fn get_current_branch(repo: &Path) -> Result<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .current_dir(repo)
        .output()
        .context("Failed to get current branch")?;
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_current_commit(repo: &Path) -> Result<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(repo)
        .output()
        .context("Failed to get current commit")?;
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_file_metrics(repo: &Path) -> Result<HashMap<String, FileMetrics>> {
    let mut metrics = HashMap::new();
    
    // Get list of all files in the repository
    let output = Command::new("git")
        .args(&["ls-files"])
        .current_dir(repo)
        .output()
        .context("Failed to list files")?;
    
    let files = String::from_utf8_lossy(&output.stdout);
    
    for file in files.lines() {
        if file.is_empty() {
            continue;
        }
        
        // Get file statistics using git log
        let log_output = Command::new("git")
            .args(&[
                "log",
                "--follow",
                "--pretty=format:%H|%an|%at",
                "--numstat",
                "--",
                file,
            ])
            .current_dir(repo)
            .output()
            .context("Failed to get file history")?;
        
        let log_str = String::from_utf8_lossy(&log_output.stdout);
        let mut commits = 0;
        let mut authors = Vec::new();
        let mut lines_added = 0;
        let mut lines_removed = 0;
        let mut last_modified = 0i64;
        
        let mut lines = log_str.lines();
        while let Some(line) = lines.next() {
            if line.contains('|') {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 3 {
                    commits += 1;
                    
                    let author = parts[1].to_string();
                    if !authors.contains(&author) {
                        authors.push(author);
                    }
                    
                    if let Ok(timestamp) = parts[2].parse::<i64>() {
                        if timestamp > last_modified {
                            last_modified = timestamp;
                        }
                    }
                }
                
                // Next line should be the numstat
                if let Some(stat_line) = lines.next() {
                    let stat_parts: Vec<&str> = stat_line.split_whitespace().collect();
                    if stat_parts.len() >= 2 {
                        if let Ok(added) = stat_parts[0].parse::<usize>() {
                            lines_added += added;
                        }
                        if let Ok(removed) = stat_parts[1].parse::<usize>() {
                            lines_removed += removed;
                        }
                    }
                }
            }
        }
        
        if commits > 0 {
            let churn_rate = (lines_added + lines_removed) as f64 / commits as f64;
            
            metrics.insert(
                file.to_string(),
                FileMetrics {
                    path: file.to_string(),
                    commits,
                    authors,
                    last_modified: DateTime::from_timestamp(last_modified, 0)
                        .unwrap_or_else(|| Utc::now()),
                    lines_added,
                    lines_removed,
                    churn_rate,
                },
            );
        }
    }
    
    Ok(metrics)
}

fn get_author_stats(repo: &Path) -> Result<HashMap<String, AuthorStats>> {
    let mut stats = HashMap::new();
    
    // Get commit statistics per author
    let output = Command::new("git")
        .args(&["shortlog", "-sn", "--all"])
        .current_dir(repo)
        .output()
        .context("Failed to get author statistics")?;
    
    let shortlog = String::from_utf8_lossy(&output.stdout);
    
    for line in shortlog.lines() {
        let parts: Vec<&str> = line.trim().splitn(2, '\t').collect();
        if parts.len() == 2 {
            if let Ok(commits) = parts[0].trim().parse::<usize>() {
                let name = parts[1].to_string();
                
                // Get detailed stats for this author
                let stat_output = Command::new("git")
                    .args(&[
                        "log",
                        "--author",
                        &name,
                        "--pretty=tformat:",
                        "--numstat",
                    ])
                    .current_dir(repo)
                    .output()
                    .context("Failed to get author file statistics")?;
                
                let stat_str = String::from_utf8_lossy(&stat_output.stdout);
                let mut files_touched = std::collections::HashSet::new();
                let mut lines_added = 0;
                let mut lines_removed = 0;
                
                for stat_line in stat_str.lines() {
                    let parts: Vec<&str> = stat_line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        if let Ok(added) = parts[0].parse::<usize>() {
                            lines_added += added;
                        }
                        if let Ok(removed) = parts[1].parse::<usize>() {
                            lines_removed += removed;
                        }
                        files_touched.insert(parts[2].to_string());
                    }
                }
                
                stats.insert(
                    name.clone(),
                    AuthorStats {
                        name,
                        commits,
                        files_touched: files_touched.len(),
                        lines_added,
                        lines_removed,
                    },
                );
            }
        }
    }
    
    Ok(stats)
}

fn count_total_commits(repo: &Path) -> Result<usize> {
    let output = Command::new("git")
        .args(&["rev-list", "--all", "--count"])
        .current_dir(repo)
        .output()
        .context("Failed to count commits")?;
    
    let count_str = String::from_utf8_lossy(&output.stdout);
    count_str.trim().parse().context("Failed to parse commit count")
}