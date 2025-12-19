//! Repo command - Manage external repositories for cross-project knowledge
//!
//! External repos (learning or contributing) are stored centrally in `~/.patina/repos/`.
//! Each repo is a full patina project with `.patina/`, `layer/`, and patina branch.
//!
//! # Example
//!
//! ```no_run
//! # fn main() -> anyhow::Result<()> {
//! // Add a repo for learning
//! // patina repo https://github.com/dojoengine/dojo
//!
//! // Add a repo for contributing (creates fork)
//! // patina repo https://github.com/dojoengine/dojo --contrib
//!
//! // List all repos
//! // patina repo list
//!
//! // Query a specific repo
//! // patina scry "spawn patterns" --repo dojo
//! # Ok(())
//! # }
//! ```

pub(crate) mod internal;

use anyhow::Result;

pub use internal::RepoEntry;

/// Repo CLI subcommands (used by main.rs via clap)
#[derive(Debug, Clone, clap::Subcommand)]
pub enum RepoCommands {
    /// Add an external repository
    Add {
        /// GitHub URL (e.g., https://github.com/owner/repo or owner/repo)
        url: String,

        /// Enable contribution mode (create fork for PRs)
        #[arg(long)]
        contrib: bool,

        /// Also fetch and index GitHub issues
        #[arg(long)]
        with_issues: bool,
    },

    /// List registered repositories
    List,

    /// Update a repository (git pull + rescrape)
    Update {
        /// Repository name (or --all for all repos)
        name: Option<String>,

        /// Update all repositories
        #[arg(long)]
        all: bool,

        /// Also run oxidize to build semantic indices
        #[arg(long)]
        oxidize: bool,
    },

    /// Remove a repository
    #[command(alias = "rm")]
    Remove {
        /// Repository name
        name: String,
    },

    /// Show details about a repository
    Show {
        /// Repository name
        name: String,
    },
}

/// Execute repo command from CLI arguments
///
/// Handles both subcommand form (`patina repo add <url>`) and
/// shorthand form (`patina repo <url>`).
pub fn execute_cli(
    command: Option<RepoCommands>,
    url: Option<String>,
    contrib: bool,
    with_issues: bool,
) -> Result<()> {
    let cmd = match (command, url) {
        // Subcommand form: patina repo add/list/update/etc
        (
            Some(RepoCommands::Add {
                url,
                contrib,
                with_issues,
            }),
            _,
        ) => RepoCommand::Add {
            url,
            contrib,
            with_issues,
        },
        (Some(RepoCommands::List), _) => RepoCommand::List,
        (Some(RepoCommands::Update { name, all, oxidize }), _) => {
            if all {
                RepoCommand::Update {
                    name: None,
                    oxidize,
                }
            } else {
                RepoCommand::Update { name, oxidize }
            }
        }
        (Some(RepoCommands::Remove { name }), _) => RepoCommand::Remove { name },
        (Some(RepoCommands::Show { name }), _) => RepoCommand::Show { name },

        // Shorthand form: patina repo <url> [--contrib] [--with-issues]
        (None, Some(url)) => RepoCommand::Add {
            url,
            contrib,
            with_issues,
        },

        // No args: show list
        (None, None) => RepoCommand::List,
    };

    execute(cmd)
}

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

/// Migrate registry paths to the new cache location.
///
/// This handles the case where repos were moved but the registry wasn't updated,
/// or where repos were registered with old paths before the migration existed.
/// Called from main.rs after patina::migration::migrate_if_needed().
pub fn migrate_registry_paths() -> bool {
    let Ok(mut registry) = internal::Registry::load() else {
        return false;
    };

    if registry.repos.is_empty() {
        return false;
    }

    let cache_base = patina::paths::repos::cache_dir();
    let mut updated_any = false;
    let mut updates: Vec<(String, String)> = Vec::new(); // (name, new_path)

    for (name, entry) in registry.repos.iter() {
        let expected_path = cache_base.join(name);
        let expected_path_str = expected_path.to_string_lossy().to_string();

        // Check if path needs updating
        if entry.path != expected_path_str {
            // Verify the repo actually exists at the expected location
            if expected_path.join(".patina/data/patina.db").exists()
                || expected_path.join(".git").exists()
            {
                updates.push((name.clone(), expected_path_str));
            }
        }
    }

    if updates.is_empty() {
        return false;
    }

    println!("📦 Updating registry paths to new cache location...");

    for (name, new_path) in updates {
        if let Some(entry) = registry.repos.get_mut(&name) {
            entry.path = new_path.clone();
            updated_any = true;
            println!("   ✓ {} -> {}", name, new_path);
        }
    }

    if updated_any {
        if let Err(e) = registry.save() {
            eprintln!("Warning: Could not save updated registry: {}", e);
            return false;
        }
        println!();
    }

    updated_any
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
