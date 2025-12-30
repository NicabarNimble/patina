//! Internal implementation for repo command
//!
//! Central storage at `~/.patina/cache/repos/` with registry at `~/.patina/registry.yaml`

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

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
    #[serde(default)]
    pub domains: Vec<String>,
}

impl Registry {
    /// Load registry from default location
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

        serde_yaml::from_str(&contents)
            .with_context(|| format!("Failed to parse registry: {}", path.display()))
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
pub fn add_repo(url: &str, contrib: bool, with_issues: bool) -> Result<()> {
    // Parse GitHub URL
    let (owner, repo_name) = parse_github_url(url)?;
    let github = format!("{}/{}", owner, repo_name);

    println!("🚀 Adding repository: {}\n", github);

    // Check if already registered
    let mut registry = Registry::load()?;
    if registry.repos.contains_key(&repo_name) {
        let existing = &registry.repos[&repo_name];
        if contrib && !existing.contrib {
            println!("📌 Repository exists, upgrading to contributor mode...");
            // TODO: Add fork logic here
            return upgrade_to_contrib(&repo_name, &mut registry);
        }
        bail!(
            "Repository '{}' already registered. Use 'patina repo update {}' to refresh.",
            repo_name,
            repo_name
        );
    }

    // Ensure repos cache directory exists
    let repos_path = paths::repos::cache_dir();
    fs::create_dir_all(&repos_path)?;

    let repo_path = repos_path.join(&repo_name);

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

    // Register in registry
    let timestamp = chrono::Utc::now().to_rfc3339();
    let domains = detect_domains(&repo_path);

    registry.repos.insert(
        repo_name.clone(),
        RepoEntry {
            name: repo_name.clone(),
            path: repo_path.to_string_lossy().to_string(),
            github,
            contrib: fork.is_some(),
            fork,
            registered: timestamp,
            domains,
        },
    );

    registry.save()?;

    println!("\n✅ Repository added successfully!");
    println!("   Path: {}", repo_path.display());
    println!("   Code events: {}", event_count);
    if issue_count > 0 {
        println!("   GitHub issues: {}", issue_count);
        println!(
            "\n   Query with: patina scry \"your query\" --repo {} --include-issues",
            repo_name
        );
    } else {
        println!(
            "\n   Query with: patina scry \"your query\" --repo {}",
            repo_name
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
    let registry = Registry::load()?;
    let entry = registry
        .repos
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", name))?;

    println!("🔄 Updating {}...\n", name);

    let repo_path = Path::new(&entry.path);

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

    println!("📚 Repository: {}\n", name);
    println!("  GitHub:     {}", entry.github);
    println!("  Path:       {}", entry.path);
    println!("  Contrib:    {}", if entry.contrib { "Yes" } else { "No" });
    if let Some(fork) = &entry.fork {
        println!("  Fork:       {}", fork);
    }
    println!("  Domains:    {}", entry.domains.join(", "));
    println!("  Registered: {}", entry.registered);

    // Show event count from database
    let db_path = Path::new(&entry.path).join(".patina/data/patina.db");
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

    let db_path = Path::new(&entry.path).join(".patina/data/patina.db");
    if !db_path.exists() {
        bail!(
            "Database not found for '{}'. Run 'patina repo update {}' to rebuild.",
            name,
            name
        );
    }

    Ok(db_path.to_string_lossy().to_string())
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

    let output = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            &clone_url,
            &target.to_string_lossy(),
        ])
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

    if !gitignore_content.contains(".patina/data") {
        let addition = "\n# Patina local data\n.patina/data/\n";
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
    // Reference repos only get dependency dimension (no sessions → no semantic, shallow clone → no temporal)
    let recipe_path = repo_path.join(".patina/oxidize.yaml");
    if !recipe_path.exists() {
        println!("   Creating oxidize.yaml recipe (dependency only)...");
        let recipe_content = r#"# Oxidize Recipe for reference repo
# Reference repos only support dependency dimension:
# - No layer/sessions/ → no semantic (no session pairs)
# - Shallow clone → no temporal (no co-change history)
# - Call graph from AST → dependency works
version: 1
embedding_model: e5-base-v2

projections:
  # Dependency projection - functions that call each other are related
  dependency:
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
fn scrape_github_issues(repo_path: &Path, github: &str) -> Result<usize> {
    use crate::commands::scrape::github::{run as github_run, GitHubScrapeConfig};

    let db_path = repo_path.join(".patina/data/patina.db");

    let config = GitHubScrapeConfig {
        repo: github.to_string(),
        limit: 500,
        force: true,
        db_path: db_path.to_string_lossy().to_string(),
    };

    let stats = github_run(config)?;
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
    // Use gh CLI to create fork
    let output = Command::new("gh")
        .args(["repo", "fork", "--clone=false"])
        .current_dir(repo_path)
        .output()
        .context("Failed to execute gh repo fork. Is GitHub CLI installed?")?;

    if !output.status.success() {
        bail!(
            "gh repo fork failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Get the fork name from gh
    let output = Command::new("gh")
        .args(["repo", "view", "--json", "name,owner"])
        .current_dir(repo_path)
        .output()?;

    // Parse output to get fork name
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Add fork as remote
    let _ = Command::new("git")
        .args([
            "remote",
            "add",
            "fork",
            &format!("git@github.com:{}", stdout.trim()),
        ])
        .current_dir(repo_path)
        .output();

    Ok(stdout.trim().to_string())
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

/// Check git status for a repo (behind/up-to-date)
///
/// Returns a human-readable status string for display.
/// Prioritizes "behind" status over "dirty" since dirty is expected
/// (patina scaffolding creates local changes).
pub fn check_repo_status(repo_path: &str) -> String {
    let path = Path::new(repo_path);

    if !path.exists() {
        return "✗ not found".to_string();
    }

    // Check if on a branch (not detached HEAD)
    let branch_output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["symbolic-ref", "-q", "HEAD"])
        .output();

    if branch_output.is_err() || !branch_output.unwrap().status.success() {
        return "⚠ detached HEAD".to_string();
    }

    // Fetch from origin (quietly)
    let fetch_result = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["fetch", "origin", "--quiet"])
        .output();

    if fetch_result.is_err() {
        return "✗ fetch failed".to_string();
    }

    // Check how many commits behind (primary concern)
    let behind_output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-list", "HEAD..@{u}", "--count"])
        .output();

    let behind_count = match behind_output {
        Ok(output) if output.status.success() => {
            let count_str = String::from_utf8_lossy(&output.stdout);
            count_str.trim().parse().unwrap_or(0)
        }
        _ => 0, // No upstream tracking, assume current
    };

    if behind_count > 0 {
        format!("⚠ {} behind", behind_count)
    } else {
        "✓ up to date".to_string()
    }
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
}
