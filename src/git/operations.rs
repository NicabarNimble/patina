//! Low-level git operations

use anyhow::{Context, Result};
use std::process::Command;

/// Check if current directory is a git repository
pub fn is_git_repo() -> Result<bool> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .context("Failed to check if directory is a git repository")?;

    Ok(output.status.success())
}

/// Get the current branch name
pub fn current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .context("Failed to get current branch")?;

    if !output.status.success() {
        anyhow::bail!("Failed to get current branch");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Get the default branch name (main or master)
pub fn default_branch() -> Result<String> {
    // Try to get from remote
    let output = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout);
            if let Some(name) = branch.trim().strip_prefix("refs/remotes/origin/") {
                return Ok(name.to_string());
            }
        }
    }

    // Fallback: check if main exists, otherwise master
    if branch_exists("main")? {
        Ok("main".to_string())
    } else if branch_exists("master")? {
        Ok("master".to_string())
    } else {
        // Last resort: use current branch
        current_branch()
    }
}

/// Check if a branch exists
pub fn branch_exists(name: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", &format!("refs/heads/{}", name)])
        .output()
        .context("Failed to check if branch exists")?;

    Ok(output.status.success())
}

/// Check if working tree is clean
pub fn is_clean() -> Result<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to check git status")?;

    Ok(output.stdout.is_empty())
}

/// Count modified files
pub fn status_count() -> Result<usize> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to get git status")?;

    let count = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .count();

    Ok(count)
}

/// Get number of commits current branch is behind another
pub fn commits_behind(current: &str, other: &str) -> Result<usize> {
    let output = Command::new("git")
        .args(["rev-list", "--count", &format!("{}..{}", current, other)])
        .output()
        .context("Failed to count commits behind")?;

    if !output.status.success() {
        return Ok(0);
    }

    let count_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    count_str.parse().context("Failed to parse commit count")
}

/// Create and checkout a new branch
pub fn checkout_new_branch(name: &str, from: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["checkout", "-b", name, from])
        .output()
        .context("Failed to create and checkout branch")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to create branch: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Rename a branch
pub fn branch_rename(old: &str, new: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["branch", "-m", old, new])
        .output()
        .context("Failed to rename branch")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to rename branch: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Get remote URL
pub fn remote_url(remote: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", remote])
        .output()
        .context("Failed to get remote URL")?;

    if !output.status.success() {
        anyhow::bail!("Remote '{}' not found", remote);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Check if a remote URL exists in any remote
pub fn has_remote(url: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["remote", "-v"])
        .output()
        .context("Failed to list remotes")?;

    let remotes = String::from_utf8_lossy(&output.stdout);
    Ok(remotes.contains(url))
}

/// Add a git remote
pub fn add_remote(name: &str, url: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["remote", "add", name, url])
        .output()
        .context("Failed to add remote")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to add remote: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Get repository name from remote URL
pub fn repo_name() -> Result<String> {
    let url = remote_url("origin")?;
    let (_, repo) = parse_github_url(&url)?;
    Ok(repo)
}

/// Parse GitHub URL into (owner, repo)
pub fn parse_github_url(url: &str) -> Result<(String, String)> {
    // Handle both SSH and HTTPS formats
    // git@github.com:owner/repo.git
    // https://github.com/owner/repo.git

    let cleaned = url
        .trim()
        .strip_suffix(".git")
        .unwrap_or(url)
        .replace("git@github.com:", "")
        .replace("https://github.com/", "");

    let parts: Vec<&str> = cleaned.split('/').collect();
    if parts.len() >= 2 {
        Ok((parts[0].to_string(), parts[1].to_string()))
    } else {
        anyhow::bail!("Invalid GitHub URL format: {}", url)
    }
}

/// Stage all changes
pub fn add_all() -> Result<()> {
    let output = Command::new("git")
        .args(["add", "."])
        .output()
        .context("Failed to stage changes")?;

    if !output.status.success() {
        anyhow::bail!("Failed to stage changes");
    }

    Ok(())
}

/// Create a commit
pub fn commit(message: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["commit", "-m", message])
        .output()
        .context("Failed to create commit")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to create commit: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Stash changes with a named message (includes untracked files)
pub fn stash_push(message: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["stash", "push", "--include-untracked", "-m", message])
        .output()
        .context("Failed to stash changes")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to stash changes: {}", stderr);
    }

    Ok(())
}

/// Checkout an existing branch
pub fn checkout(branch: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["checkout", branch])
        .output()
        .context("Failed to checkout branch")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to checkout {}: {}",
            branch,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Rebase current branch onto another
/// Returns Ok(true) if rebase succeeded, Ok(false) if conflicts, Err on other failure
pub fn rebase(onto: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["rebase", onto])
        .output()
        .context("Failed to rebase")?;

    if output.status.success() {
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("CONFLICT") || stderr.contains("could not apply") {
            Ok(false) // Conflicts - caller should handle
        } else {
            anyhow::bail!("Failed to rebase onto {}: {}", onto, stderr);
        }
    }
}

/// Abort an in-progress rebase
pub fn rebase_abort() -> Result<()> {
    let output = Command::new("git")
        .args(["rebase", "--abort"])
        .output()
        .context("Failed to abort rebase")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to abort rebase: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Fetch from remote
pub fn fetch(remote: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["fetch", remote])
        .output()
        .context("Failed to fetch from remote")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to fetch {}: {}",
            remote,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url_ssh() {
        let url = "git@github.com:dustproject/dust.git";
        let (owner, repo) = parse_github_url(url).unwrap();
        assert_eq!(owner, "dustproject");
        assert_eq!(repo, "dust");
    }

    #[test]
    fn test_parse_github_url_https() {
        let url = "https://github.com/dustproject/dust.git";
        let (owner, repo) = parse_github_url(url).unwrap();
        assert_eq!(owner, "dustproject");
        assert_eq!(repo, "dust");
    }
}
