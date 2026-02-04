//! Internal implementation for repo command
//!
//! Central storage at `~/.patina/cache/repos/` with registry at `~/.patina/registry.yaml`

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use patina::forge::{ForgeWriter, GitHubWriter};
use patina::paths;

/// Registry schema (persisted to ~/.patina/registry.yaml)
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Registry {
    pub version: u32,
    #[serde(default)]
    pub projects: HashMap<String, ProjectEntry>,
    #[serde(default)]
    pub repos: HashMap<String, RepoEntry>,
}

/// A primary project (user's own code)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    pub path: String,
    #[serde(rename = "type")]
    pub project_type: String,
    pub registered: String,
    #[serde(default)]
    pub domains: Vec<String>,
}

/// An external repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoEntry {
    #[serde(skip)]
    #[serde(default)]
    pub name: String,
    pub path: String,
    pub github: String,
    #[serde(default)]
    pub contrib: bool,
    #[serde(default)]
    pub fork: Option<String>,
    pub registered: String,
    /// SHA of HEAD when we last synced (add or update)
    #[serde(default)]
    pub synced_commit: Option<String>,
    #[serde(default)]
    pub domains: Vec<String>,
}

impl Registry {
    /// Load registry from default location, validating paths on load.
    pub fn load() -> Result<Self> {
        let path = paths::registry_path();
        if !path.exists() {
            return Ok(Registry {
                version: 1,
                ..Default::default()
            });
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read registry: {}", path.display()))?;

        let registry: Registry = serde_yaml::from_str(&contents)
            .with_context(|| format!("Failed to parse registry: {}", path.display()))?;

        // Validate repo paths against expected cache prefix
        let cache_prefix = paths::repos::cache_dir();
        for (name, entry) in &registry.repos {
            validate_repo_path(&entry.path, &cache_prefix, name)?;
        }

        Ok(registry)
    }

    /// Save registry to default location
    pub fn save(&self) -> Result<()> {
        let path = paths::registry_path();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_yaml::to_string(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }
}

/// Add a repository
pub fn add_repo(url: &str, contrib: bool, with_issues: bool, no_oxidize: bool) -> Result<()> {
    // Parse GitHub URL
    let (owner, repo_name) = parse_github_url(url)?;
    let github = format!("{}/{}", owner, repo_name);

    println!("🚀 Adding repository: {}\n", github);

    // Check if already registered
    let mut registry = Registry::load()?;
    if registry.repos.contains_key(&github) {
        let existing = &registry.repos[&github];
        if contrib && !existing.contrib {
            println!("📌 Repository exists, upgrading to contributor mode...");
            // TODO: Add fork logic here
            return upgrade_to_contrib(&github, &mut registry);
        }
        bail!(
            "Repository '{}' already registered. Use 'patina repo update {}' to refresh.",
            github,
            github
        );
    }

    // Ensure repos cache directory exists
    let repos_path = paths::repos::cache_dir();
    fs::create_dir_all(&repos_path)?;

    let repo_path = repos_path.join(&github);

    // Clone repository
    println!("📥 Cloning {}...", github);
    clone_repo(url, &repo_path)?;

    // Create patina branch
    println!("🌿 Creating patina branch...");
    create_patina_branch(&repo_path)?;

    // Scaffold .patina directory
    println!("📁 Scaffolding .patina structure...");
    scaffold_patina(&repo_path)?;

    // Run scrape
    println!("🔍 Scraping codebase...");
    let event_count = scrape_repo(&repo_path)?;

    // Scrape GitHub issues if requested
    let issue_count = if with_issues {
        println!("🐙 Fetching GitHub issues...");
        match scrape_github_issues(&repo_path, &github) {
            Ok(count) => {
                println!("  💰 Indexed {} issues", count);
                count
            }
            Err(e) => {
                println!(
                    "  ⚠️  GitHub scrape failed: {}. Continuing without issues.",
                    e
                );
                0
            }
        }
    } else {
        0
    };

    // Handle fork if contrib mode
    let fork = if contrib {
        println!("🍴 Creating fork...");
        match create_fork(&repo_path, &owner, &repo_name) {
            Ok(fork_name) => Some(fork_name),
            Err(e) => {
                println!("⚠️  Fork creation failed: {}. Continuing without fork.", e);
                None
            }
        }
    } else {
        None
    };

    // Build semantic indices unless skipped
    let oxidize_success = if no_oxidize {
        println!("\n⏭️  Skipping semantic indices (--no-oxidize)");
        false
    } else {
        println!("\n🧪 Building semantic indices...");
        match oxidize_repo(&repo_path) {
            Ok(()) => {
                println!("   ✅ Semantic search enabled");
                true
            }
            Err(e) => {
                println!("   ⚠️  Oxidize failed: {}. Semantic search unavailable.", e);
                println!(
                    "      Run 'patina repo update {} --oxidize' to retry.",
                    github
                );
                false
            }
        }
    };

    // Register in registry
    let timestamp = chrono::Utc::now().to_rfc3339();
    let domains = detect_domains(&repo_path);
    let synced_commit = get_head_sha(&repo_path);

    registry.repos.insert(
        github.clone(),
        RepoEntry {
            name: github.clone(),
            path: repo_path.to_string_lossy().to_string(),
            github: github.clone(),
            contrib: fork.is_some(),
            fork,
            registered: timestamp,
            synced_commit,
            domains,
        },
    );

    registry.save()?;

    let search_mode = if oxidize_success {
        "semantic + lexical"
    } else {
        "lexical only"
    };

    println!("\n✅ Repository added successfully!");
    println!("   Path: {}", repo_path.display());
    println!("   Code events: {}", event_count);
    println!("   Search: {}", search_mode);
    if issue_count > 0 {
        println!("   GitHub issues: {}", issue_count);
        println!(
            "\n   Query with: patina scry \"your query\" --repo {} --include-issues",
            github
        );
    } else {
        println!(
            "\n   Query with: patina scry \"your query\" --repo {}",
            github
        );
    }

    Ok(())
}

/// List all repositories
pub fn list_repos() -> Result<Vec<RepoEntry>> {
    let registry = Registry::load()?;
    let mut repos: Vec<RepoEntry> = registry
        .repos
        .into_iter()
        .map(|(name, mut entry)| {
            entry.name = name;
            entry
        })
        .collect();
    repos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(repos)
}

/// Update a specific repository
pub fn update_repo(name: &str, oxidize: bool) -> Result<()> {
    let mut registry = Registry::load()?;
    let entry = registry
        .repos
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", name))?
        .clone();

    println!("🔄 Updating {}...\n", name);

    let repo_path = Path::new(&entry.path);

    // Ensure UID exists (migration for existing ref repos)
    patina::project::create_uid_if_missing(repo_path)?;

    // Git pull
    println!("📥 Pulling latest changes...");
    git_pull(repo_path)?;

    // Re-scrape
    println!("🔍 Re-scraping codebase...");
    let event_count = scrape_repo(repo_path)?;

    // Oxidize if requested
    if oxidize {
        println!("\n🧪 Building semantic indices...");
        oxidize_repo(repo_path)?;
    }

    // Record synced commit
    if let Some(entry) = registry.repos.get_mut(name) {
        entry.synced_commit = get_head_sha(repo_path);
        registry.save()?;
    }

    println!("\n✅ Updated {} ({} events)", name, event_count);
    if oxidize {
        println!("   Semantic indices built - scry will use vector search");
    }

    Ok(())
}

/// Update all repositories
pub fn update_all_repos(oxidize: bool) -> Result<()> {
    let repos = list_repos()?;

    if repos.is_empty() {
        println!("No repositories to update.");
        return Ok(());
    }

    println!("🔄 Updating {} repositories...\n", repos.len());

    let mut success = 0;
    for repo in &repos {
        print!("  {} ... ", repo.name);
        match update_repo(&repo.name, oxidize) {
            Ok(_) => {
                println!("✓");
                success += 1;
            }
            Err(e) => println!("✗ {}", e),
        }
    }

    println!("\n✅ Updated {}/{} repositories", success, repos.len());

    Ok(())
}

/// Remove a repository
pub fn remove_repo(name: &str) -> Result<()> {
    let mut registry = Registry::load()?;
    let entry = registry
        .repos
        .remove(name)
        .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", name))?;

    println!("🗑️  Removing {}...", name);

    // Remove from filesystem
    let repo_path = Path::new(&entry.path);
    if repo_path.exists() {
        fs::remove_dir_all(repo_path)
            .with_context(|| format!("Failed to remove directory: {}", repo_path.display()))?;
    }

    registry.save()?;

    println!("✅ Removed {}", name);

    Ok(())
}

/// Show details about a repository
pub fn show_repo(name: &str) -> Result<()> {
    let registry = Registry::load()?;
    let entry = registry
        .repos
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", name))?;

    let repo_path = Path::new(&entry.path);

    println!("📚 Repository: {}\n", name);
    println!("  GitHub:     {}", entry.github);
    println!("  Path:       {}", entry.path);
    println!("  Contrib:    {}", if entry.contrib { "Yes" } else { "No" });
    if let Some(fork) = &entry.fork {
        println!("  Fork:       {}", fork);
    }
    println!("  Domains:    {}", entry.domains.join(", "));
    println!("  Registered: {}", format_timestamp(&entry.registered));

    // Show upstream status
    if let Some(upstream) = get_upstream_head(repo_path) {
        let commit_date =
            get_commit_date_relative(repo_path, &upstream).unwrap_or_else(|| "unknown".to_string());
        println!("  Last commit: {}", commit_date);

        // Check if synced
        let synced = entry
            .synced_commit
            .as_ref()
            .map(|s| s == &upstream)
            .unwrap_or(false);

        if synced {
            println!("  Synced:     ✓ up to date");
        } else {
            // Count commits behind
            let behind = count_commits_behind(repo_path, entry.synced_commit.as_deref());
            if behind > 0 {
                println!("  Synced:     ⚠ {} commits behind", behind);
            } else {
                println!("  Synced:     ⚠ needs update");
            }
        }
    }

    // Show event count from database
    let db_path = repo_path.join(".patina/local/data/patina.db");
    if db_path.exists() {
        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
            if let Ok(count) = conn.query_row("SELECT COUNT(*) FROM eventlog", [], |row| {
                row.get::<_, i64>(0)
            }) {
                println!("  Events:     {}", count);
            }
        }
    }

    Ok(())
}

/// Get database path for a repo
pub fn get_repo_db_path(name: &str) -> Result<String> {
    let registry = Registry::load()?;
    let entry = registry
        .repos
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", name))?;

    let db_path = Path::new(&entry.path).join(".patina/local/data/patina.db");
    if !db_path.exists() {
        bail!(
            "Database not found for '{}'. Run 'patina repo update {}' to rebuild.",
            name,
            name
        );
    }

    Ok(db_path.to_string_lossy().to_string())
}

/// Get the filesystem path for a registered repo
pub fn get_repo_path(name: &str) -> Result<std::path::PathBuf> {
    let registry = Registry::load()?;
    let entry = registry.repos.get(name).ok_or_else(|| {
        anyhow::anyhow!(
            "Repository '{}' not found. Use 'patina repo list' to see registered repos.",
            name
        )
    })?;

    let path = std::path::PathBuf::from(&entry.path);
    if !path.exists() {
        bail!(
            "Repository path '{}' not found. It may have been moved or deleted.",
            entry.path
        );
    }

    Ok(path)
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper functions
// ─────────────────────────────────────────────────────────────────────────────

/// Parse GitHub URL to extract owner and repo name
fn parse_github_url(url: &str) -> Result<(String, String)> {
    // Handle various formats:
    // - https://github.com/owner/repo
    // - https://github.com/owner/repo.git
    // - git@github.com:owner/repo.git
    // - owner/repo

    let url = url.trim();

    // Handle short form (owner/repo)
    if !url.contains("://") && !url.contains('@') && url.contains('/') {
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() == 2 {
            return Ok((
                parts[0].to_string(),
                parts[1].trim_end_matches(".git").to_string(),
            ));
        }
    }

    // Handle git@github.com:owner/repo.git
    if url.starts_with("git@github.com:") {
        let path = url.trim_start_matches("git@github.com:");
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Ok((
                parts[0].to_string(),
                parts[1].trim_end_matches(".git").to_string(),
            ));
        }
    }

    // Handle https://github.com/owner/repo
    if url.contains("github.com") {
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() >= 5 {
            let owner = parts[3].to_string();
            let repo = parts[4].trim_end_matches(".git").to_string();
            return Ok((owner, repo));
        }
    }

    bail!(
        "Could not parse GitHub URL: {}. Expected format: https://github.com/owner/repo",
        url
    )
}

/// Clone a repository
fn clone_repo(url: &str, target: &Path) -> Result<()> {
    if target.exists() {
        bail!("Target directory already exists: {}", target.display());
    }

    // Convert short form (owner/repo) to full GitHub URL
    let clone_url = if url.contains("://") || url.contains('@') {
        url.to_string()
    } else {
        format!("https://github.com/{}", url)
    };

    // Full clone - we want commit history for knowledge extraction
    // Commit messages are rich "why" context, especially in LLM-assisted codebases
    let output = Command::new("git")
        .args(["clone", &clone_url, &target.to_string_lossy()])
        .output()
        .context("Failed to execute git clone")?;

    if !output.status.success() {
        bail!(
            "git clone failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Create patina branch in repo
fn create_patina_branch(repo_path: &Path) -> Result<()> {
    // Check if patina branch exists
    let output = Command::new("git")
        .args(["branch", "--list", "patina"])
        .current_dir(repo_path)
        .output()?;

    let branch_exists = !String::from_utf8_lossy(&output.stdout).trim().is_empty();

    if branch_exists {
        // Checkout existing branch
        let output = Command::new("git")
            .args(["checkout", "patina"])
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            bail!(
                "Failed to checkout patina branch: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    } else {
        // Create new branch
        let output = Command::new("git")
            .args(["checkout", "-b", "patina"])
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            bail!(
                "Failed to create patina branch: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    Ok(())
}

/// Scaffold .patina directory structure
fn scaffold_patina(repo_path: &Path) -> Result<()> {
    let patina_dir = repo_path.join(".patina");
    let data_dir = patina_dir.join("data");

    fs::create_dir_all(&data_dir)?;

    // Create UID if not already present (preserves existing from clone)
    patina::project::create_uid_if_missing(repo_path)?;

    // Create minimal config
    let config_path = patina_dir.join("config.toml");
    if !config_path.exists() {
        fs::write(
            &config_path,
            r#"# Patina configuration for external repo
[project]
type = "external"

[scrape]
include = ["**/*.rs", "**/*.cairo", "**/*.sol", "**/*.ts", "**/*.js", "**/*.py", "**/*.go"]
exclude = ["target/", "node_modules/", ".git/"]

[embeddings]
model = "e5-base-v2"
"#,
        )?;
    }

    // Create layer/sessions directory for learning sessions
    let sessions_dir = repo_path.join("layer/sessions");
    fs::create_dir_all(&sessions_dir)?;

    // Add .patina to gitignore if not already
    let gitignore_path = repo_path.join(".gitignore");
    let gitignore_content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)?
    } else {
        String::new()
    };

    if !gitignore_content.contains(".patina/local") {
        let addition = "\n# Patina local state (derived, not committed)\n.patina/local/\n";
        fs::write(
            &gitignore_path,
            format!("{}{}", gitignore_content, addition),
        )?;
    }

    Ok(())
}

/// Run scrape on a repo
fn scrape_repo(repo_path: &Path) -> Result<usize> {
    use crate::commands::scrape;

    // Save current directory
    let original_dir = std::env::current_dir()?;

    // Change to repo directory
    std::env::set_current_dir(repo_path)?;

    // Run code scrape
    let config = scrape::ScrapeConfig::new(true);
    let stats = scrape::code::run(config)?;

    // Run git scrape
    let _ = scrape::git::run(true);

    // Restore directory
    std::env::set_current_dir(original_dir)?;

    Ok(stats.items_processed)
}

/// Run oxidize on a repo to build semantic indices
fn oxidize_repo(repo_path: &Path) -> Result<()> {
    use crate::commands::oxidize;
    use std::os::unix::fs::symlink;

    // Save current directory (where patina project with models lives)
    let original_dir = std::env::current_dir()?;
    let resources_path = original_dir.join("resources");

    // Change to repo directory
    std::env::set_current_dir(repo_path)?;

    // Ensure config.toml has embeddings section (fix for older scaffolds)
    let config_path = repo_path.join(".patina/config.toml");
    if config_path.exists() {
        let config_content = fs::read_to_string(&config_path)?;
        if !config_content.contains("[embeddings]") {
            println!("   Adding embeddings config...");
            let updated = format!("{}\n[embeddings]\nmodel = \"e5-base-v2\"\n", config_content);
            fs::write(&config_path, updated)?;
        }
    }

    // Create oxidize.yaml if it doesn't exist
    // Reference repos get all three dimensions:
    // - dependency: call graph from AST
    // - temporal: co-change history from git
    // - semantic: commit messages as training signal (NL → code pairs)
    let recipe_path = repo_path.join(".patina/oxidize.yaml");
    if !recipe_path.exists() {
        println!("   Creating oxidize.yaml recipe (dependency + temporal + semantic)...");
        let recipe_content = r#"# Oxidize Recipe for reference repo
# Reference repos support all three dimensions:
# - dependency: call graph from AST (functions that call each other)
# - temporal: co-change history from git (files that change together)
# - semantic: commit messages as training signal (NL → code similarity)
version: 1
embedding_model: e5-base-v2

projections:
  # Dependency projection - functions that call each other are related
  dependency:
    layers: [768, 1024, 256]
    epochs: 10
    batch_size: 32

  # Temporal projection - files that change together are related
  temporal:
    layers: [768, 1024, 256]
    epochs: 10
    batch_size: 32

  # Semantic projection - commit messages train NL → code similarity
  semantic:
    layers: [768, 1024, 256]
    epochs: 10
    batch_size: 32
"#;
        fs::write(&recipe_path, recipe_content)?;
    }

    // Symlink resources directory if it doesn't exist (for embedding models)
    let repo_resources = repo_path.join("resources");
    if !repo_resources.exists() && resources_path.exists() {
        println!("   Linking model resources...");
        symlink(&resources_path, &repo_resources).context("Failed to create resources symlink")?;
    }

    // Run oxidize
    let result = oxidize::oxidize();

    // Remove symlink after oxidize (clean up)
    if repo_resources.is_symlink() {
        let _ = fs::remove_file(&repo_resources);
    }

    // Restore directory
    std::env::set_current_dir(original_dir)?;

    result
}

/// Scrape GitHub issues for a repo
fn scrape_github_issues(repo_path: &Path, _github: &str) -> Result<usize> {
    use crate::commands::scrape::forge::{run, ForgeScrapeConfig};

    let config = ForgeScrapeConfig {
        force: true,
        working_dir: Some(repo_path.to_path_buf()),
        ..Default::default()
    };

    let stats = run(config)?;
    Ok(stats.items_processed)
}

/// Git pull in a repo
fn git_pull(repo_path: &Path) -> Result<()> {
    // First, stash any local changes
    let _ = Command::new("git")
        .args(["stash"])
        .current_dir(repo_path)
        .output();

    // Pull from origin
    let output = Command::new("git")
        .args(["pull", "origin", "HEAD"])
        .current_dir(repo_path)
        .output()
        .context("Failed to execute git pull")?;

    if !output.status.success() {
        // Try pulling from main/master
        let output2 = Command::new("git")
            .args(["pull", "origin", "main"])
            .current_dir(repo_path)
            .output();

        if output2.is_err() || !output2.unwrap().status.success() {
            let _ = Command::new("git")
                .args(["pull", "origin", "master"])
                .current_dir(repo_path)
                .output();
        }
    }

    Ok(())
}

/// Create a GitHub fork
fn create_fork(repo_path: &Path, _owner: &str, _repo: &str) -> Result<String> {
    let writer = GitHubWriter;
    let fork_url = writer.fork(repo_path)?;

    // Add fork as remote
    let _ = Command::new("git")
        .args(["remote", "add", "fork", &fork_url])
        .current_dir(repo_path)
        .output();

    Ok(fork_url)
}

/// Upgrade existing repo to contributor mode
fn upgrade_to_contrib(name: &str, registry: &mut Registry) -> Result<()> {
    let entry = registry
        .repos
        .get_mut(name)
        .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", name))?;

    let repo_path = Path::new(&entry.path);

    println!("🍴 Creating fork...");
    match create_fork(repo_path, "", "") {
        Ok(fork_name) => {
            entry.contrib = true;
            entry.fork = Some(fork_name);
            registry.save()?;
            println!("✅ Upgraded to contributor mode");
            Ok(())
        }
        Err(e) => bail!("Failed to create fork: {}", e),
    }
}

/// Detect project domains from file extensions and content
fn detect_domains(repo_path: &Path) -> Vec<String> {
    let mut domains = Vec::new();

    // Check for common file extensions
    let extensions_to_domains = [
        ("rs", "rust"),
        ("cairo", "cairo"),
        ("sol", "solidity"),
        ("ts", "typescript"),
        ("tsx", "typescript"),
        ("js", "javascript"),
        ("py", "python"),
        ("go", "go"),
        ("java", "java"),
        ("cpp", "cpp"),
        ("c", "c"),
    ];

    for (ext, domain) in extensions_to_domains {
        let pattern = format!("**/*.{}", ext);
        if let Ok(entries) = glob::glob(&repo_path.join(&pattern).to_string_lossy()) {
            if entries.take(1).count() > 0 {
                domains.push(domain.to_string());
            }
        }
    }

    // Check for specific frameworks/tools
    if repo_path.join("Scarb.toml").exists() && !domains.contains(&"starknet".to_string()) {
        domains.push("starknet".to_string());
    }
    if repo_path.join("foundry.toml").exists() && !domains.contains(&"ethereum".to_string()) {
        domains.push("ethereum".to_string());
    }
    if repo_path.join("Cargo.toml").exists() && !domains.contains(&"rust".to_string()) {
        domains.push("rust".to_string());
    }

    domains.sort();
    domains.dedup();
    domains
}

/// Get HEAD commit SHA for a repo
fn get_head_sha(repo_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Get the date of a commit as relative time (e.g., "2 hours ago")
fn get_commit_date_relative(repo_path: &Path, commit: &str) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["log", "-1", "--format=%cr", commit])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Count how many commits we're behind upstream
fn count_commits_behind(repo_path: &Path, synced_commit: Option<&str>) -> usize {
    let Some(synced) = synced_commit else {
        return 0;
    };

    // Count commits from synced to upstream
    for remote_ref in ["origin/HEAD", "origin/main", "origin/master"] {
        if let Ok(output) = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .args([
                "rev-list",
                "--count",
                &format!("{}..{}", synced, remote_ref),
            ])
            .output()
        {
            if output.status.success() {
                let count_str = String::from_utf8_lossy(&output.stdout);
                return count_str.trim().parse().unwrap_or(0);
            }
        }
    }
    0
}

/// Get upstream HEAD after fetching
fn get_upstream_head(repo_path: &Path) -> Option<String> {
    // Fetch first
    let _ = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["fetch", "origin", "--quiet"])
        .output();

    // Try origin/HEAD, then origin/main, then origin/master
    for remote_ref in ["origin/HEAD", "origin/main", "origin/master"] {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .args(["rev-parse", remote_ref])
            .output()
            .ok()?;

        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }
    None
}

/// Format an ISO timestamp as a human-readable relative time
fn format_timestamp(iso: &str) -> String {
    use chrono::{DateTime, Utc};

    let Ok(dt) = iso.parse::<DateTime<Utc>>() else {
        return iso.to_string(); // Fallback to raw if parse fails
    };

    let now = Utc::now();
    let duration = now.signed_duration_since(dt);

    if duration.num_days() > 30 {
        dt.format("%Y-%m-%d").to_string()
    } else if duration.num_days() > 0 {
        format!("{} days ago", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{} minutes ago", duration.num_minutes())
    } else {
        "just now".to_string()
    }
}

/// Check git status for a repo (behind/up-to-date)
///
/// Returns a human-readable status string for display.
/// Compares synced_commit against current upstream HEAD.
pub fn check_repo_status(repo_path: &str, synced_commit: Option<&str>) -> String {
    let path = Path::new(repo_path);

    if !path.exists() {
        return "✗ not found".to_string();
    }

    // Fetch and get upstream HEAD
    let Some(upstream) = get_upstream_head(path) else {
        return "✗ fetch failed".to_string();
    };

    // Get last commit date
    let commit_date = get_commit_date_relative(path, &upstream).unwrap_or_else(|| "?".to_string());

    // Check if synced
    let is_synced = synced_commit.map(|s| s == upstream).unwrap_or(false);

    if is_synced {
        format!("✓ synced ({})", commit_date)
    } else {
        let behind = count_commits_behind(path, synced_commit);
        if behind > 0 {
            format!("⚠ {} behind ({})", behind, commit_date)
        } else {
            format!("⚠ needs sync ({})", commit_date)
        }
    }
}

/// Validate that a repo path is under the expected cache prefix.
///
/// Canonicalizes the path (resolving symlinks, `..`, etc.) and verifies it
/// starts with the cache directory. Rejects path traversal attacks from
/// tampered registry files.
fn validate_repo_path(path: &str, cache_prefix: &Path, repo_name: &str) -> Result<()> {
    let repo_path = Path::new(path);

    // Reject paths containing traversal components regardless of existence
    if path.contains("..") {
        bail!(
            "Registry path for '{}' contains path traversal: {}",
            repo_name,
            path
        );
    }

    // If the path doesn't exist yet (not cloned), validate the raw string.
    // A legitimate path should start with the cache prefix string.
    if !repo_path.exists() {
        let prefix_str = cache_prefix.to_string_lossy();
        if !path.starts_with(prefix_str.as_ref()) {
            bail!(
                "Registry path for '{}' is outside cache directory: {}",
                repo_name,
                path
            );
        }
        return Ok(());
    }

    // For existing paths, canonicalize to resolve symlinks and ..
    let canonical = repo_path
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize path for '{}': {}", repo_name, path))?;
    let canonical_prefix = cache_prefix
        .canonicalize()
        .unwrap_or_else(|_| cache_prefix.to_path_buf());

    if !canonical.starts_with(&canonical_prefix) {
        bail!(
            "Registry path for '{}' resolves outside cache directory: {} -> {}",
            repo_name,
            path,
            canonical.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url_https() {
        let (owner, repo) = parse_github_url("https://github.com/dojoengine/dojo").unwrap();
        assert_eq!(owner, "dojoengine");
        assert_eq!(repo, "dojo");
    }

    #[test]
    fn test_parse_github_url_https_git() {
        let (owner, repo) = parse_github_url("https://github.com/dojoengine/dojo.git").unwrap();
        assert_eq!(owner, "dojoengine");
        assert_eq!(repo, "dojo");
    }

    #[test]
    fn test_parse_github_url_ssh() {
        let (owner, repo) = parse_github_url("git@github.com:dojoengine/dojo.git").unwrap();
        assert_eq!(owner, "dojoengine");
        assert_eq!(repo, "dojo");
    }

    #[test]
    fn test_parse_github_url_short() {
        let (owner, repo) = parse_github_url("dojoengine/dojo").unwrap();
        assert_eq!(owner, "dojoengine");
        assert_eq!(repo, "dojo");
    }

    #[test]
    fn test_registry_default() {
        let registry = Registry::default();
        assert_eq!(registry.version, 0);
        assert!(registry.repos.is_empty());
    }

    #[test]
    fn test_validate_repo_path_good() {
        let cache = Path::new("/home/user/.patina/cache/repos");
        assert!(validate_repo_path(
            "/home/user/.patina/cache/repos/owner/repo",
            cache,
            "owner/repo"
        )
        .is_ok());
    }

    #[test]
    fn test_validate_repo_path_traversal() {
        let cache = Path::new("/home/user/.patina/cache/repos");
        assert!(validate_repo_path(
            "/home/user/.patina/cache/repos/../../etc/passwd",
            cache,
            "evil"
        )
        .is_err());
    }

    #[test]
    fn test_validate_repo_path_outside() {
        let cache = Path::new("/home/user/.patina/cache/repos");
        assert!(validate_repo_path("/tmp/evil", cache, "evil").is_err());
    }
}
