//! Fork detection and creation

use super::operations::{add_remote, has_remote, parse_github_url, remote_url, repo_name};
use anyhow::{Context, Result};
use std::process::Command;

/// Fork status for current repository
#[derive(Debug, Clone, PartialEq)]
pub enum ForkStatus {
    /// User owns this repository
    Owned,
    /// Fork already exists and remote is configured
    AlreadyForked { remote_name: String },
    /// Fork exists on GitHub but no remote configured
    ForkExistsNeedRemote,
    /// Need to create fork
    NeedsFork { upstream: (String, String) },
}

/// Get current GitHub user
fn gh_current_user() -> Result<String> {
    let output = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .output()
        .context("Failed to get current GitHub user. Is 'gh' installed and authenticated?")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to get GitHub user: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Check if a GitHub repository exists
fn gh_repo_exists(full_name: &str) -> Result<bool> {
    let output = Command::new("gh")
        .args(["repo", "view", full_name])
        .output()
        .context("Failed to check if repository exists")?;

    Ok(output.status.success())
}

/// Create a fork on GitHub
fn gh_repo_fork() -> Result<()> {
    let output = Command::new("gh")
        .args(["repo", "fork", "--remote=false"])
        .output()
        .context("Failed to create fork")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to create fork: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Detect fork status for current repository
pub fn detect_fork_status() -> Result<ForkStatus> {
    // Get current repo remote
    let origin_url = remote_url("origin")?;
    let (owner, repo) = parse_github_url(&origin_url)?;

    // Get current user
    let current_user = gh_current_user()?;

    // If user owns the repo, no fork needed
    if owner == current_user {
        return Ok(ForkStatus::Owned);
    }

    // Check if fork already exists on GitHub
    let fork_exists = gh_repo_exists(&format!("{}/{}", current_user, repo))?;

    if fork_exists {
        // Check if we already have it as a remote
        let fork_url = format!("git@github.com:{}/{}.git", current_user, repo);
        let has_fork_remote = has_remote(&fork_url)?;

        if has_fork_remote {
            Ok(ForkStatus::AlreadyForked {
                remote_name: "fork".to_string(),
            })
        } else {
            Ok(ForkStatus::ForkExistsNeedRemote)
        }
    } else {
        Ok(ForkStatus::NeedsFork {
            upstream: (owner, repo),
        })
    }
}

/// Ensure fork exists and is configured
pub fn ensure_fork() -> Result<ForkStatus> {
    match detect_fork_status()? {
        ForkStatus::Owned => {
            println!("✓ Repository owned by you");
            Ok(ForkStatus::Owned)
        }

        ForkStatus::NeedsFork { upstream } => {
            println!("📍 External repo: {}/{}", upstream.0, upstream.1);
            println!("⚡ Creating fork...");

            gh_repo_fork()?;

            let current_user = gh_current_user()?;
            let fork_remote = format!("git@github.com:{}/{}.git", current_user, upstream.1);
            add_remote("fork", &fork_remote)?;

            println!("✓ Fork created: {}", fork_remote);
            println!("✓ Added remote 'fork'");

            Ok(ForkStatus::AlreadyForked {
                remote_name: "fork".to_string(),
            })
        }

        ForkStatus::ForkExistsNeedRemote => {
            println!("📍 Fork exists on GitHub, adding remote...");

            let current_user = gh_current_user()?;
            let repo = repo_name()?;
            let fork_remote = format!("git@github.com:{}/{}.git", current_user, repo);
            add_remote("fork", &fork_remote)?;

            println!("✓ Added remote 'fork'");

            Ok(ForkStatus::AlreadyForked {
                remote_name: "fork".to_string(),
            })
        }

        ForkStatus::AlreadyForked { remote_name } => {
            println!("✓ Fork already configured (remote: {})", remote_name);
            Ok(ForkStatus::AlreadyForked { remote_name })
        }
    }
}
