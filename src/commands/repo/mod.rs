//! Repo command - Manage external repositories for cross-project knowledge
//!
//! External repos (learning or contributing) are stored centrally in `~/.patina/repos/`.
//! Each repo is a full patina project with `.patina/`, `layer/`, and patina branch.
//!
//! # Example
//!
//! ```no_run
//! // Add a repo for learning
//! patina repo https://github.com/dojoengine/dojo
//!
//! // Add a repo for contributing (creates fork)
//! patina repo https://github.com/dojoengine/dojo --contrib
//!
//! // List all repos
//! patina repo list
//!
//! // Query a specific repo
//! patina scry "spawn patterns" --repo dojo
//! ```

mod internal;

use anyhow::Result;

pub use internal::RepoEntry;

/// Add an external repository
///
/// Clones the repo to `~/.patina/repos/<name>/`, creates patina branch,
/// scaffolds `.patina/` structure, and runs scrape.
///
/// With `--contrib`, also creates a GitHub fork and sets up push remote.
/// With `--with-issues`, also fetches and indexes GitHub issues.
pub fn add(url: &str, contrib: bool, with_issues: bool) -> Result<()> {
    internal::add_repo(url, contrib, with_issues)
}

/// List all registered repositories
pub fn list() -> Result<Vec<RepoEntry>> {
    internal::list_repos()
}

/// Update a repository (git pull + rescrape + optional oxidize)
pub fn update(name: &str, oxidize: bool) -> Result<()> {
    internal::update_repo(name, oxidize)
}

/// Update all repositories
pub fn update_all(oxidize: bool) -> Result<()> {
    internal::update_all_repos(oxidize)
}

/// Remove a repository
pub fn remove(name: &str) -> Result<()> {
    internal::remove_repo(name)
}

/// Show details about a repository
pub fn show(name: &str) -> Result<()> {
    internal::show_repo(name)
}

/// Get the database path for a repo (for scry --repo)
pub fn get_db_path(name: &str) -> Result<String> {
    internal::get_repo_db_path(name)
}

/// Execute the repo command (main entry point from CLI)
pub fn execute(command: RepoCommand) -> Result<()> {
    match command {
        RepoCommand::Add {
            url,
            contrib,
            with_issues,
        } => add(&url, contrib, with_issues),
        RepoCommand::List => {
            let repos = list()?;
            if repos.is_empty() {
                println!("No repositories registered.");
                println!("\nAdd one with: patina repo <url>");
                return Ok(());
            }

            println!("📚 Registered Repositories\n");
            println!("{:<20} {:<35} {:<8} DOMAINS", "NAME", "GITHUB", "CONTRIB");
            println!("{}", "─".repeat(80));

            for repo in repos {
                let contrib_str = if repo.contrib { "✓ fork" } else { "-" };
                let domains = repo.domains.join(", ");
                println!(
                    "{:<20} {:<35} {:<8} {}",
                    repo.name, repo.github, contrib_str, domains
                );
            }
            Ok(())
        }
        RepoCommand::Update { name, oxidize } => {
            if let Some(n) = name {
                update(&n, oxidize)
            } else {
                update_all(oxidize)
            }
        }
        RepoCommand::Remove { name } => remove(&name),
        RepoCommand::Show { name } => show(&name),
    }
}

/// Repo subcommands
#[derive(Debug, Clone)]
pub enum RepoCommand {
    Add {
        url: String,
        contrib: bool,
        with_issues: bool,
    },
    List,
    Update {
        name: Option<String>,
        oxidize: bool,
    },
    Remove {
        name: String,
    },
    Show {
        name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_command_variants() {
        let add = RepoCommand::Add {
            url: "https://github.com/test/repo".to_string(),
            contrib: false,
            with_issues: true,
        };
        assert!(matches!(add, RepoCommand::Add { .. }));

        let list = RepoCommand::List;
        assert!(matches!(list, RepoCommand::List));
    }
}
