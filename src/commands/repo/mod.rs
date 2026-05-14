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

        /// Sparse checkout path (repeatable)
        #[arg(long = "sparse", value_name = "PATH", action = clap::ArgAction::Append)]
        sparse: Vec<String>,

        /// Skip building semantic indices (faster, lexical search only)
        #[arg(long)]
        no_oxidize: bool,
    },

    /// List registered repositories
    List {
        /// Partial registered repo name to search for (case-insensitive substring)
        query: Option<String>,

        /// Show git status (behind/dirty) for each repo
        #[arg(long)]
        status: bool,

        /// Output stable JSON for tools/LLMs
        #[arg(long)]
        json: bool,
    },

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

        /// Number of repositories to process concurrently (used with --all)
        ///
        /// Tip: with --oxidize, 2-3 jobs is usually best on laptops.
        #[arg(long)]
        jobs: Option<usize>,

        /// Retry only repositories that failed in the previous batch run
        #[arg(long)]
        failed_only: bool,
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
    sparse: Vec<String>,
) -> Result<()> {
    let cmd = match (command, url) {
        // Subcommand form: patina repo add/list/update/etc
        (
            Some(RepoCommands::Add {
                url,
                contrib,
                sparse,
                no_oxidize,
            }),
            _,
        ) => RepoCommand::Add {
            url,
            contrib,
            sparse,
            no_oxidize,
        },
        (
            Some(RepoCommands::List {
                query,
                status,
                json,
            }),
            _,
        ) => RepoCommand::List {
            query,
            status,
            json,
        },
        (
            Some(RepoCommands::Update {
                name,
                all,
                oxidize,
                jobs,
                failed_only,
            }),
            _,
        ) => {
            if all {
                RepoCommand::Update {
                    name: None,
                    oxidize,
                    jobs,
                    failed_only,
                }
            } else {
                RepoCommand::Update {
                    name,
                    oxidize,
                    jobs,
                    failed_only,
                }
            }
        }
        (Some(RepoCommands::Remove { name }), _) => RepoCommand::Remove { name },
        (Some(RepoCommands::Show { name }), _) => RepoCommand::Show { name },

        // Shorthand form: patina repo <url> [--contrib] [--sparse path]
        // Note: --no-oxidize not available in shorthand, defaults to false (oxidize runs)
        (None, Some(url)) => RepoCommand::Add {
            url,
            contrib,
            sparse,
            no_oxidize: false,
        },

        // No args: show list
        (None, None) => RepoCommand::List {
            query: None,
            status: false,
            json: false,
        },
    };

    execute(cmd)
}

/// Add an external repository
///
/// Clones the repo to `~/.patina/repos/<name>/`, creates patina branch,
/// scaffolds `.patina/` structure, runs scrape, and builds semantic indices.
///
/// With `--contrib`, also creates a GitHub fork and sets up push remote.
/// With `--no-oxidize`, skips building semantic indices (faster, lexical search only).
pub fn add(url: &str, contrib: bool, no_oxidize: bool, sparse: Vec<String>) -> Result<()> {
    internal::add_repo(url, contrib, no_oxidize, sparse)
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
pub fn update_all(oxidize: bool, jobs: Option<usize>, failed_only: bool) -> Result<()> {
    internal::update_all_repos(oxidize, jobs, failed_only)
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

/// Get the filesystem path for a repo (for oxidize --repo)
pub fn get_path(name: &str) -> Result<std::path::PathBuf> {
    internal::get_repo_path(name)
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
        if !entry.sparse.is_empty() {
            // Sparse entries are already on a dedicated cache lane.
            continue;
        }

        let expected_path = cache_base.join(name);
        let expected_path_str = expected_path.to_string_lossy().to_string();

        // Check if path needs updating
        if entry.path != expected_path_str {
            // Verify the repo actually exists at the expected location
            if patina::eventlog::resolve_patina_db_path(&expected_path).exists()
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

fn repo_mode(repo: &RepoEntry) -> String {
    if repo.sparse.is_empty() {
        "full".to_string()
    } else {
        format!("sparse({})", repo.sparse.len())
    }
}

fn normalize_query(query: Option<&str>) -> Option<String> {
    query
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .map(str::to_lowercase)
}

fn filter_repos_by_name_query(repos: Vec<RepoEntry>, query: Option<&str>) -> Vec<RepoEntry> {
    let Some(normalized) = normalize_query(query) else {
        return repos;
    };

    repos
        .into_iter()
        .filter(|repo| repo.name.to_lowercase().contains(&normalized))
        .collect()
}

fn repo_list_json(repos: &[RepoEntry], query: Option<&str>, status: bool) -> serde_json::Value {
    let rows: Vec<serde_json::Value> = repos
        .iter()
        .map(|repo| {
            let mut row = serde_json::json!({
                "name": repo.name,
                "github": repo.github,
                "path": repo.path,
                "domains": repo.domains,
                "mode": repo_mode(repo),
                "contrib": repo.contrib,
                "registered": repo.registered,
                "synced_commit": repo.synced_commit,
            });

            if status {
                row["status"] = serde_json::Value::String(internal::check_repo_status(
                    &repo.path,
                    repo.synced_commit.as_deref(),
                ));
            }

            row
        })
        .collect();

    serde_json::json!({
        "schema": "patina.repo.list.v1",
        "query": query,
        "count": rows.len(),
        "repositories": rows,
    })
}

fn print_no_repo_matches(query: &str) {
    println!("No registered repositories match: {}", query);
    println!("\nTry:");
    println!("  patina repo list");
    println!("  patina repo add <owner/repo>");
}

/// Execute the repo command (main entry point from CLI)
pub fn execute(command: RepoCommand) -> Result<()> {
    match command {
        RepoCommand::Add {
            url,
            contrib,
            sparse,
            no_oxidize,
        } => add(&url, contrib, no_oxidize, sparse),
        RepoCommand::List {
            query,
            status,
            json,
        } => {
            let query_ref = query.as_deref();
            let mut repos = list()?;
            repos = filter_repos_by_name_query(repos, query_ref);

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&repo_list_json(&repos, query_ref, status))?
                );
                return Ok(());
            }

            if repos.is_empty() {
                if let Some(query) = query_ref {
                    print_no_repo_matches(query);
                } else {
                    println!("No repositories registered.");
                    println!("\nAdd one with: patina repo <url>");
                }
                return Ok(());
            }

            if let Some(query) = query_ref {
                println!("📚 Matching Repositories: {}\n", query);
            } else {
                println!("📚 Registered Repositories\n");
            }

            let filtered = normalize_query(query_ref).is_some();

            if status {
                if filtered {
                    println!(
                        "{:<40} {:<8} {:<10} {:<28} PATH",
                        "NAME", "CONTRIB", "MODE", "STATUS"
                    );
                    println!("{}", "─".repeat(140));
                } else {
                    println!("{:<40} {:<8} {:<10} STATUS", "NAME", "CONTRIB", "MODE");
                    println!("{}", "─".repeat(100));
                }

                for repo in repos {
                    let contrib_str = if repo.contrib { "✓ fork" } else { "-" };
                    let mode = repo_mode(&repo);
                    let status_str =
                        internal::check_repo_status(&repo.path, repo.synced_commit.as_deref());
                    if filtered {
                        println!(
                            "{:<40} {:<8} {:<10} {:<28} {}",
                            repo.name, contrib_str, mode, status_str, repo.path
                        );
                    } else {
                        println!(
                            "{:<40} {:<8} {:<10} {}",
                            repo.name, contrib_str, mode, status_str
                        );
                    }
                }
            } else if filtered {
                println!(
                    "{:<40} {:<8} {:<10} {:<32} PATH",
                    "NAME", "CONTRIB", "MODE", "DOMAINS"
                );
                println!("{}", "─".repeat(140));

                for repo in repos {
                    let contrib_str = if repo.contrib { "✓ fork" } else { "-" };
                    let mode = repo_mode(&repo);
                    let domains = repo.domains.join(", ");
                    println!(
                        "{:<40} {:<8} {:<10} {:<32} {}",
                        repo.name, contrib_str, mode, domains, repo.path
                    );
                }
            } else {
                println!("{:<40} {:<8} {:<10} DOMAINS", "NAME", "CONTRIB", "MODE");
                println!("{}", "─".repeat(100));

                for repo in repos {
                    let contrib_str = if repo.contrib { "✓ fork" } else { "-" };
                    let mode = repo_mode(&repo);
                    let domains = repo.domains.join(", ");
                    println!(
                        "{:<40} {:<8} {:<10} {}",
                        repo.name, contrib_str, mode, domains
                    );
                }
            }
            Ok(())
        }
        RepoCommand::Update {
            name,
            oxidize,
            jobs,
            failed_only,
        } => {
            if let Some(n) = name {
                if jobs.is_some() {
                    println!("ℹ️  Ignoring --jobs for single repository update");
                }
                if failed_only {
                    println!("ℹ️  Ignoring --failed-only for single repository update");
                }
                update(&n, oxidize)
            } else {
                update_all(oxidize, jobs, failed_only)
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
        sparse: Vec<String>,
        no_oxidize: bool,
    },
    List {
        query: Option<String>,
        status: bool,
        json: bool,
    },
    Update {
        name: Option<String>,
        oxidize: bool,
        jobs: Option<usize>,
        failed_only: bool,
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
            sparse: vec!["design/mvp".to_string()],
            no_oxidize: false,
        };
        assert!(matches!(add, RepoCommand::Add { .. }));

        let list = RepoCommand::List {
            query: Some("flu".to_string()),
            status: false,
            json: true,
        };
        assert!(matches!(list, RepoCommand::List { .. }));
    }

    fn repo_entry(name: &str) -> RepoEntry {
        RepoEntry {
            name: name.to_string(),
            path: format!("/tmp/repos/{name}"),
            github: name.to_string(),
            sparse: Vec::new(),
            contrib: false,
            fork: None,
            registered: "2026-05-14T00:00:00Z".to_string(),
            synced_commit: Some("abc123".to_string()),
            domains: vec!["typescript".to_string()],
        }
    }

    #[test]
    fn test_repo_list_partial_name_filter_is_case_insensitive() {
        let repos = vec![
            repo_entry("withastro/flue"),
            repo_entry("google-gemini/gemini-cli"),
            repo_entry("anomalyco/opencode"),
        ];

        let filtered = filter_repos_by_name_query(repos, Some("FLU"));

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "withastro/flue");
    }

    #[test]
    fn test_repo_list_partial_name_filter_no_match_is_empty() {
        let repos = vec![repo_entry("withastro/flue")];

        let filtered = filter_repos_by_name_query(repos, Some("missing"));

        assert!(filtered.is_empty());
    }

    #[test]
    fn test_repo_list_json_includes_stable_metadata_and_path() {
        let repos = vec![repo_entry("withastro/flue")];

        let json = repo_list_json(&repos, Some("flu"), false);

        assert_eq!(json["schema"], "patina.repo.list.v1");
        assert_eq!(json["query"], "flu");
        assert_eq!(json["count"], 1);
        assert_eq!(json["repositories"][0]["name"], "withastro/flue");
        assert_eq!(json["repositories"][0]["path"], "/tmp/repos/withastro/flue");
        assert_eq!(json["repositories"][0]["mode"], "full");
        assert_eq!(json["repositories"][0]["contrib"], false);
        assert_eq!(
            json["repositories"][0]["registered"],
            "2026-05-14T00:00:00Z"
        );
        assert_eq!(json["repositories"][0]["synced_commit"], "abc123");
    }

    #[test]
    fn test_repo_list_json_no_match_shape() {
        let repos: Vec<RepoEntry> = Vec::new();

        let json = repo_list_json(&repos, Some("missing"), false);

        assert_eq!(json["schema"], "patina.repo.list.v1");
        assert_eq!(json["query"], "missing");
        assert_eq!(json["count"], 0);
        assert_eq!(json["repositories"].as_array().unwrap().len(), 0);
    }
}
