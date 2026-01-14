//! Fork detection and creation

use super::operations::{add_remote, has_remote, parse_github_url, remote_url};
use crate::forge::{ForgeWriter, GitHubWriter};
use anyhow::{Context, Result};
use std::path::Path;
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
    /// Local-only mode (no GitHub remote)
    LocalOnly,
    /// Created new GitHub repository
    CreatedNew { repo_name: String },
}

/// Get the default ForgeWriter for GitHub.
fn writer() -> GitHubWriter {
    GitHubWriter
}

/// Create initial README and commit for new projects
fn create_initial_commit(repo_name: &str) -> Result<()> {
    use std::fs;
    use std::path::Path;

    // Check if there are any commits already
    let output = Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .output();

    // If we have commits, don't create an initial one
    if let Ok(output) = output {
        if output.status.success() {
            let count = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if count != "0" && !count.is_empty() {
                return Ok(());
            }
        }
    }

    // Create README if it doesn't exist
    if !Path::new("README.md").exists() {
        let readme_content = format!(
            r#"# {}

## 🎨 Patina-Powered Development

This project uses [Patina](https://github.com/ai-1st/patina) for AI-assisted development. Patina captures and evolves development patterns, making AI assistants smarter about your project over time.

### What This Means

- **AI Context**: Your AI assistant understands this project's architecture and patterns
- **Pattern Evolution**: Development wisdom accumulates and improves over time
- **Session Tracking**: Development sessions are tracked with Git for pattern extraction
- **LLM-Agnostic**: Works with Claude, Gemini, or other AI assistants

### Quick Start

```bash
# Start a development session (with Claude)
patina session start "implementing new feature"

# Build the project
patina build

# Run tests
patina test

# End session and capture learnings
patina session end
```

### Project Structure

- `PROJECT_DESIGN.toml` - Core architecture and design decisions
- `.claude/` or `.gemini/` - AI assistant configuration
- `layer/` - Accumulated patterns and knowledge
- `.devcontainer/` - Containerized development environment

### Learn More

- [Patina Documentation](https://github.com/ai-1st/patina)
- [Session Capture](https://github.com/ai-1st/patina/blob/main/layer/core/session-capture.md)
- [Design Patterns](https://github.com/ai-1st/patina/blob/main/layer/core/)

---
*Built with Patina - Context orchestration for AI development*
"#,
            repo_name
        );
        fs::write("README.md", readme_content)?;

        // Add README to git
        Command::new("git")
            .args(["add", "README.md"])
            .output()
            .context("Failed to add README.md")?;
    }

    // Create initial commit
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .output()
        .context("Failed to create initial commit")?;

    Ok(())
}

/// Create a new GitHub repository
fn gh_repo_create(name: &str) -> Result<()> {
    // Ensure we have at least one commit
    create_initial_commit(name)?;

    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    writer().create_repo(name, true, &current_dir)?;
    Ok(())
}

/// Get the current directory name for use as repo name
fn current_dir_name() -> Result<String> {
    std::env::current_dir()
        .context("Failed to get current directory")?
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Invalid directory name"))
}

/// Handle connecting to an existing GitHub repository
fn handle_existing_github_repo(repo_name: &str, user: &str) -> Result<()> {
    use std::fs;

    // Check if directory has any files
    let current_dir = std::env::current_dir()?;
    let has_local_files = fs::read_dir(&current_dir)?.any(|_| true);

    if !has_local_files {
        // Empty directory - just clone normally
        println!("📥 Cloning repository...");
        clone_to_current_directory(repo_name, user)?;
    } else {
        // Directory has files - need to preserve them
        println!("📋 Preserving local files before syncing with GitHub...");

        // Clone just the .git directory
        clone_git_dir_only(repo_name, user)?;

        // Check for local changes
        let status_output = Command::new("git")
            .args(["status", "--porcelain"])
            .output()?;

        if !status_output.stdout.is_empty() {
            // There are local changes - preserve them in a branch
            preserve_local_work_in_branch()?;

            // Switch to main/master/patina branch
            ensure_clean_branch()?;

            println!("✓ Local work preserved in branch");
            println!("💡 To review your local work: git branch -a");
        } else {
            println!("✓ Local files match repository");
        }
    }

    Ok(())
}

/// Clone repository to current directory (must be empty)
fn clone_to_current_directory(repo_name: &str, user: &str) -> Result<()> {
    let output = Command::new("gh")
        .args(["repo", "clone", &format!("{}/{}", user, repo_name), "."])
        .output()
        .context("Failed to clone repository")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to clone repository: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Clone only the .git directory to current location
fn clone_git_dir_only(repo_name: &str, user: &str) -> Result<()> {
    use std::fs;

    let target_git = Path::new(".git");

    // If .git already exists, just add the origin remote
    if target_git.exists() {
        println!("📝 Using existing git repository, adding origin remote...");

        // Add origin remote
        let remote_url = format!("git@github.com:{}/{}.git", user, repo_name);
        let output = Command::new("git")
            .args(["remote", "add", "origin", &remote_url])
            .output()?;

        // If remote already exists, update it
        if !output.status.success() {
            Command::new("git")
                .args(["remote", "set-url", "origin", &remote_url])
                .output()
                .context("Failed to set origin remote")?;
        }

        // Fetch from origin
        Command::new("git")
            .args(["fetch", "origin"])
            .output()
            .context("Failed to fetch from origin")?;

        return Ok(());
    }

    // Create temp directory for cloning
    let temp_dir = format!(".patina-clone-tmp-{}", std::process::id());

    // Clone to temp directory
    let output = Command::new("gh")
        .args([
            "repo",
            "clone",
            &format!("{}/{}", user, repo_name),
            &temp_dir,
        ])
        .output()
        .context("Failed to clone repository")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to clone repository: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Move .git directory from temp to current directory
    let temp_git = Path::new(&temp_dir).join(".git");

    fs::rename(&temp_git, target_git).context("Failed to move .git directory")?;

    // Clean up temp directory
    fs::remove_dir_all(&temp_dir).context("Failed to clean up temporary directory")?;

    Ok(())
}

/// Preserve local work in a timestamped branch
fn preserve_local_work_in_branch() -> Result<()> {
    use chrono::Utc;

    // Create timestamped branch name
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let branch_name = format!("local-work-{}", timestamp);

    // Create and checkout new branch
    Command::new("git")
        .args(["checkout", "-b", &branch_name])
        .output()
        .context("Failed to create branch")?;

    // Add all files (respects .gitignore)
    Command::new("git")
        .args(["add", "-A"])
        .output()
        .context("Failed to add files")?;

    // Commit the local work
    let commit_message = format!("Local work preserved by patina init at {}", timestamp);

    Command::new("git")
        .args(["commit", "-m", &commit_message])
        .output()
        .context("Failed to commit local work")?;

    println!("🌿 Created branch '{}' with your local work", branch_name);

    Ok(())
}

/// Ensure we're on a clean main/master/patina branch
fn ensure_clean_branch() -> Result<()> {
    // Try branches in order of preference
    let branches = ["patina", "main", "master"];

    for branch in branches {
        let output = Command::new("git").args(["checkout", branch]).output()?;

        if output.status.success() {
            println!("✓ Switched to '{}' branch", branch);
            return Ok(());
        }
    }

    // If no standard branch exists, create patina from HEAD
    Command::new("git")
        .args(["checkout", "-b", "patina"])
        .output()
        .context("Failed to create patina branch")?;

    println!("✓ Created 'patina' branch");
    Ok(())
}

/// Ensure fork exists and is configured
pub fn ensure_fork(local: bool) -> Result<ForkStatus> {
    // Local-only mode - skip GitHub integration
    if local {
        println!("✓ Local-only mode (no GitHub remote required)");
        return Ok(ForkStatus::LocalOnly);
    }

    // Try to get the origin remote
    match remote_url("origin") {
        Ok(url) => {
            // Origin exists, proceed with normal fork detection
            ensure_fork_with_origin(url)
        }
        Err(_) => {
            // No origin remote - check if GitHub repo exists
            let repo_name = current_dir_name()?;
            let current_user = writer().current_user()?;
            let full_repo_name = format!("{}/{}", current_user, repo_name);

            if writer().repo_exists(&current_user, &repo_name)? {
                // Repository exists on GitHub - clone it
                println!("📦 Found existing GitHub repository: {}", full_repo_name);

                // Handle existing repo (clone and preserve local work)
                handle_existing_github_repo(&repo_name, &current_user)?;

                println!("✓ Connected to existing repository");
                Ok(ForkStatus::CreatedNew { repo_name })
            } else {
                // No GitHub repo exists - create a new one
                println!("📦 No GitHub repository found. Creating new repository...");
                println!("   Repository name: {}", repo_name);

                // Create the repository (this also sets origin and pushes)
                gh_repo_create(&repo_name)?;

                println!(
                    "✓ Created private repository: github.com/{}/{}",
                    current_user, repo_name
                );
                println!("✓ Added origin remote and pushed initial commit");

                Ok(ForkStatus::CreatedNew { repo_name })
            }
        }
    }
}

/// Handle fork logic when origin exists
fn ensure_fork_with_origin(origin_url: String) -> Result<ForkStatus> {
    let (owner, repo) = parse_github_url(&origin_url)?;
    let current_user = writer().current_user()?;

    // If user owns the repo, no fork needed
    if owner == current_user {
        println!("✓ Repository owned by you");
        return Ok(ForkStatus::Owned);
    }

    // Check if fork already exists on GitHub
    let fork_exists = writer().repo_exists(&current_user, &repo)?;

    if fork_exists {
        // Check if we already have it as a remote
        let fork_url = format!("git@github.com:{}/{}.git", current_user, repo);
        let has_fork_remote = has_remote(&fork_url)?;

        if has_fork_remote {
            println!("✓ Fork already configured (remote: fork)");
            Ok(ForkStatus::AlreadyForked {
                remote_name: "fork".to_string(),
            })
        } else {
            println!("📍 Fork exists on GitHub, adding remote...");
            add_remote("fork", &fork_url)?;
            println!("✓ Added remote 'fork'");
            Ok(ForkStatus::AlreadyForked {
                remote_name: "fork".to_string(),
            })
        }
    } else {
        // Need to create fork
        println!("📍 External repo: {}/{}", owner, repo);
        println!("⚡ Creating fork...");

        let current_dir = std::env::current_dir().context("Failed to get current directory")?;
        let fork_remote = writer().fork(&current_dir)?;
        add_remote("fork", &fork_remote)?;

        println!("✓ Fork created: {}", fork_remote);
        println!("✓ Added remote 'fork'");

        Ok(ForkStatus::AlreadyForked {
            remote_name: "fork".to_string(),
        })
    }
}

/// Detect fork status for current repository (legacy - for backward compatibility)
pub fn detect_fork_status() -> Result<ForkStatus> {
    // Get current repo remote
    let origin_url = remote_url("origin")?;
    let (owner, repo) = parse_github_url(&origin_url)?;

    // Get current user
    let current_user = writer().current_user()?;

    // If user owns the repo, no fork needed
    if owner == current_user {
        return Ok(ForkStatus::Owned);
    }

    // Check if fork already exists on GitHub
    let fork_exists = writer().repo_exists(&current_user, &repo)?;

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
