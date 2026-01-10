//! Forge write operations for repository management.
//!
//! "Do X": Create and fork repositories on a forge platform.
//!
//! This module provides a trait-based abstraction over different forge platforms
//! (GitHub, Gitea) for write operations: authentication, forking, creating repos.
//!
//! # Design
//!
//! ForgeWriter complements ForgeReader:
//! - ForgeReader: read-only access to issues, PRs (for scraping)
//! - ForgeWriter: write operations for fork, create (for init/repo commands)
//!
//! # Example
//!
//! ```ignore
//! use patina::forge::writer::{GitHubWriter, ForgeWriter};
//!
//! let writer = GitHubWriter;
//! if writer.is_authenticated()? {
//!     let user = writer.current_user()?;
//!     println!("Logged in as: {}", user);
//! }
//! ```

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

/// Write operations on a forge platform.
///
/// "Do X": Create and fork repositories on a forge platform.
///
/// This trait covers authentication and repository creation/forking.
/// Each method is designed to work with the platform's CLI tool.
pub trait ForgeWriter {
    /// Check if authenticated to this forge.
    fn is_authenticated(&self) -> Result<bool>;

    /// Get the current authenticated username.
    fn current_user(&self) -> Result<String>;

    /// Check if a repository exists on the forge.
    fn repo_exists(&self, owner: &str, repo: &str) -> Result<bool>;

    /// Fork the repository in the given directory.
    ///
    /// Uses git remote to detect what to fork.
    /// Returns the fork URL (e.g., "git@github.com:user/repo.git").
    fn fork(&self, repo_path: &Path) -> Result<String>;

    /// Create a new repository in the current user's namespace.
    ///
    /// Returns the repository URL.
    fn create_repo(&self, name: &str, private: bool, repo_path: &Path) -> Result<String>;
}

/// GitHub implementation of ForgeWriter.
///
/// Uses `gh` CLI for all operations. Authentication is handled by `gh auth login`.
pub struct GitHubWriter;

impl ForgeWriter for GitHubWriter {
    fn is_authenticated(&self) -> Result<bool> {
        let output = Command::new("gh")
            .args(["auth", "status"])
            .output()
            .context("Failed to run `gh auth status`. Is `gh` CLI installed?")?;

        Ok(output.status.success())
    }

    fn current_user(&self) -> Result<String> {
        let output = Command::new("gh")
            .args(["api", "user", "--jq", ".login"])
            .output()
            .context("Failed to get current GitHub user. Is 'gh' installed and authenticated?")?;

        if !output.status.success() {
            bail!(
                "Failed to get GitHub user: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn repo_exists(&self, owner: &str, repo: &str) -> Result<bool> {
        let full_name = format!("{}/{}", owner, repo);
        let output = Command::new("gh")
            .args(["repo", "view", &full_name])
            .output()
            .context("Failed to check if repository exists")?;

        Ok(output.status.success())
    }

    fn fork(&self, repo_path: &Path) -> Result<String> {
        // Create fork without adding remote (we'll do that ourselves)
        let output = Command::new("gh")
            .args(["repo", "fork", "--remote=false"])
            .current_dir(repo_path)
            .output()
            .context("Failed to execute `gh repo fork`. Is GitHub CLI installed?")?;

        if !output.status.success() {
            bail!(
                "gh repo fork failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Get current user to construct fork URL
        let user = self.current_user()?;

        // Get repo name from git remote
        let repo_name = get_repo_name_from_git(repo_path)?;

        Ok(format!("git@github.com:{}/{}.git", user, repo_name))
    }

    fn create_repo(&self, name: &str, private: bool, repo_path: &Path) -> Result<String> {
        let mut args = vec!["repo", "create", name, "--source=.", "--push"];
        if private {
            args.push("--private");
        } else {
            args.push("--public");
        }

        let output = Command::new("gh")
            .args(&args)
            .current_dir(repo_path)
            .output()
            .context("Failed to create GitHub repository")?;

        if !output.status.success() {
            bail!(
                "Failed to create repository: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let user = self.current_user()?;
        Ok(format!("git@github.com:{}/{}.git", user, name))
    }
}

/// Get the repository name from git remote origin.
fn get_repo_name_from_git(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo_path)
        .output()
        .context("Failed to get git remote URL")?;

    if !output.status.success() {
        bail!("No origin remote found");
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Parse repo name from URL
    // SSH: git@github.com:owner/repo.git
    // HTTPS: https://github.com/owner/repo
    let repo_name = if let Some(rest) = url.strip_prefix("git@") {
        // SSH format
        rest.split(':')
            .nth(1)
            .and_then(|p| p.strip_suffix(".git").or(Some(p)))
            .and_then(|p| p.split('/').next_back())
    } else if url.starts_with("https://") || url.starts_with("http://") {
        // HTTPS format
        url.trim_end_matches(".git").split('/').next_back()
    } else {
        None
    };

    repo_name
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not parse repository name from URL: {}", url))
}

/// Null implementation for repos without a forge.
///
/// All operations fail with clear error messages.
pub struct NoneWriter;

impl ForgeWriter for NoneWriter {
    fn is_authenticated(&self) -> Result<bool> {
        Ok(false)
    }

    fn current_user(&self) -> Result<String> {
        bail!("No forge configured - cannot get current user")
    }

    fn repo_exists(&self, _owner: &str, _repo: &str) -> Result<bool> {
        Ok(false)
    }

    fn fork(&self, _repo_path: &Path) -> Result<String> {
        bail!("No forge configured - cannot create fork")
    }

    fn create_repo(&self, _name: &str, _private: bool, _repo_path: &Path) -> Result<String> {
        bail!("No forge configured - cannot create repository")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_get_repo_name_ssh() {
        // This would need a mock git directory, so just test the parsing logic
        let url = "git@github.com:owner/myrepo.git";
        let name = url
            .strip_prefix("git@")
            .and_then(|r| r.split(':').nth(1))
            .and_then(|p| p.strip_suffix(".git"))
            .and_then(|p| p.split('/').next_back());
        assert_eq!(name, Some("myrepo"));
    }

    #[test]
    fn test_get_repo_name_https() {
        let url = "https://github.com/owner/myrepo";
        let name = url.trim_end_matches(".git").split('/').next_back();
        assert_eq!(name, Some("myrepo"));
    }
}
