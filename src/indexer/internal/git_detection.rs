//! Git state detection using shell commands
//!
//! This module provides git state detection without adding git2 dependency,
//! staying true to Patina's philosophy of minimal dependencies and escape hatches.

use super::GitState;
use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Detect git state for a file using git CLI
pub fn detect_file_state(repo_path: &Path, file_path: &Path) -> Result<GitState> {
    // Make path relative to repo
    let relative_path = file_path.strip_prefix(repo_path).unwrap_or(file_path);

    // Get git status for the file
    let output = Command::new("git")
        .current_dir(repo_path)
        .args([
            "status",
            "--porcelain",
            "--",
            relative_path.to_str().unwrap(),
        ])
        .output()
        .context("Failed to run git status")?;

    if !output.status.success() {
        // Not in a git repo or git not available
        return Ok(GitState::Untracked {
            detected_at: Utc::now(),
            files: vec![file_path.to_path_buf()],
        });
    }

    let status = String::from_utf8_lossy(&output.stdout);
    let status = status.trim();

    // Parse git status output
    match status {
        "" => {
            // File is tracked and unmodified, check if it's committed
            detect_committed_state(repo_path, relative_path)
        }
        s if s.starts_with("??") => Ok(GitState::Untracked {
            detected_at: Utc::now(),
            files: vec![file_path.to_path_buf()],
        }),
        s if s.starts_with("A ") || s.starts_with("M ") => Ok(GitState::Staged {
            files: vec![file_path.to_path_buf()],
            staged_at: Utc::now(),
        }),
        s if s.starts_with(" M") => Ok(GitState::Modified {
            files: vec![file_path.to_path_buf()],
            has_staged: false,
            last_change: Utc::now(),
        }),
        s if s.starts_with("MM") => Ok(GitState::Modified {
            files: vec![file_path.to_path_buf()],
            has_staged: true,
            last_change: Utc::now(),
        }),
        _ => {
            // Default to modified for other states
            Ok(GitState::Modified {
                files: vec![file_path.to_path_buf()],
                has_staged: false,
                last_change: Utc::now(),
            })
        }
    }
}

/// Detect if a file has been committed and get commit info
fn detect_committed_state(repo_path: &Path, file_path: &Path) -> Result<GitState> {
    // Get last commit that touched this file
    let output = Command::new("git")
        .current_dir(repo_path)
        .args([
            "log",
            "-1",
            "--pretty=format:%H|%s|%aI",
            "--",
            file_path.to_str().unwrap(),
        ])
        .output()
        .context("Failed to run git log")?;

    if output.stdout.is_empty() {
        // No commits for this file yet
        return Ok(GitState::Untracked {
            detected_at: Utc::now(),
            files: vec![file_path.to_path_buf()],
        });
    }

    let commit_info = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = commit_info.split('|').collect();

    if parts.len() >= 3 {
        let sha = parts[0].to_string();
        let message = parts[1].to_string();
        let timestamp = parts[2].parse().unwrap_or_else(|_| Utc::now());

        // Check if this commit has been pushed
        if is_commit_pushed(repo_path, &sha)? {
            let (remote, branch) = get_push_info(repo_path, &sha)?;
            Ok(GitState::Pushed {
                remote,
                branch,
                sha,
            })
        } else {
            Ok(GitState::Committed {
                sha,
                message,
                timestamp,
                files: vec![file_path.to_path_buf()],
            })
        }
    } else {
        Ok(GitState::Committed {
            sha: "unknown".to_string(),
            message: "unknown".to_string(),
            timestamp: Utc::now(),
            files: vec![file_path.to_path_buf()],
        })
    }
}

/// Check if a commit has been pushed to any remote
fn is_commit_pushed(repo_path: &Path, sha: &str) -> Result<bool> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["branch", "-r", "--contains", sha])
        .output()
        .context("Failed to check if commit is pushed")?;

    Ok(!output.stdout.is_empty())
}

/// Get remote and branch info for a pushed commit
fn get_push_info(repo_path: &Path, sha: &str) -> Result<(String, String)> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["branch", "-r", "--contains", sha])
        .output()
        .context("Failed to get push info")?;

    let branches = String::from_utf8_lossy(&output.stdout);
    let first_branch = branches.lines().next().unwrap_or("origin/main");

    // Parse remote/branch
    let parts: Vec<&str> = first_branch.trim().split('/').collect();
    if parts.len() >= 2 {
        Ok((parts[0].to_string(), parts[1..].join("/")))
    } else {
        Ok(("origin".to_string(), "main".to_string()))
    }
}

/// Batch detect git states for multiple files (more efficient)
pub fn detect_batch_states(
    repo_path: &Path,
    files: &[PathBuf],
) -> Result<HashMap<PathBuf, GitState>> {
    let mut states = HashMap::new();

    // Get status for all files at once
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to run git status")?;

    if !output.status.success() {
        // Not in a git repo, mark all as untracked
        for file in files {
            states.insert(
                file.clone(),
                GitState::Untracked {
                    detected_at: Utc::now(),
                    files: vec![file.clone()],
                },
            );
        }
        return Ok(states);
    }

    // Parse status output
    let status_map = parse_status_output(&output.stdout, repo_path)?;

    // Check each file
    for file in files {
        let relative_path = file.strip_prefix(repo_path).unwrap_or(file);

        if let Some(status) = status_map.get(relative_path) {
            states.insert(file.clone(), status.clone());
        } else {
            // File is tracked and unmodified
            let state = detect_committed_state(repo_path, relative_path)?;
            states.insert(file.clone(), state);
        }
    }

    Ok(states)
}

/// Parse git status --porcelain output into a map
fn parse_status_output(output: &[u8], _repo_path: &Path) -> Result<HashMap<PathBuf, GitState>> {
    let mut status_map = HashMap::new();
    let output_str = String::from_utf8_lossy(output);

    for line in output_str.lines() {
        if line.len() < 3 {
            continue;
        }

        let status_code = &line[0..2];
        let file_path = PathBuf::from(line[3..].trim());

        let state = match status_code {
            "??" => GitState::Untracked {
                detected_at: Utc::now(),
                files: vec![file_path.clone()],
            },
            "A " | "M " => GitState::Staged {
                files: vec![file_path.clone()],
                staged_at: Utc::now(),
            },
            " M" => GitState::Modified {
                files: vec![file_path.clone()],
                has_staged: false,
                last_change: Utc::now(),
            },
            "MM" => GitState::Modified {
                files: vec![file_path.clone()],
                has_staged: true,
                last_change: Utc::now(),
            },
            _ => GitState::Modified {
                files: vec![file_path.clone()],
                has_staged: false,
                last_change: Utc::now(),
            },
        };

        status_map.insert(file_path, state);
    }

    Ok(status_map)
}

/// Check if git is available
pub fn is_git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get current branch name
pub fn get_current_branch(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["branch", "--show-current"])
        .output()
        .context("Failed to get current branch")?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_available() {
        // This test might fail in CI without git
        let available = is_git_available();
        println!("Git available: {}", available);
    }
}
