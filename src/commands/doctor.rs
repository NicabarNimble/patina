use anyhow::{Context, Result};
use patina::environment::Environment;
use patina::session::SessionManager;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize, Deserialize)]
struct HealthCheck {
    status: String, // "healthy", "warning", "critical"
    environment_changes: EnvironmentChanges,
    project_config: ProjectStatus,
    recommendations: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct EnvironmentChanges {
    missing_tools: Vec<ToolChange>,
    new_tools: Vec<ToolChange>,
    version_changes: Vec<ToolChange>,
}

#[derive(Serialize, Deserialize)]
struct ToolChange {
    name: String,
    old_version: Option<String>,
    new_version: Option<String>,
    required: bool,
}

#[derive(Serialize, Deserialize)]
struct ProjectStatus {
    llm: String,
    adapter_version: Option<String>,
    layer_patterns: usize,
    sessions: usize,
}

pub fn execute(
    json_output: bool,
    check_repos: bool,
    update_repos: bool,
    audit_files: bool,
) -> Result<i32> {
    // Find project root first (needed for all subcommands)
    let project_root = SessionManager::find_project_root()
        .context("Not in a Patina project directory. Run 'patina init' first.")?;

    // If --audit flag is set, run file audit instead
    if audit_files {
        crate::commands::audit::execute(&project_root)?;
        return Ok(0);
    }

    // If --repos flag is set, handle repo management instead
    if check_repos {
        return handle_repos(update_repos);
    }

    let _non_interactive = json_output || std::env::var("PATINA_NONINTERACTIVE").is_ok();

    if !json_output {
        println!("🏥 Checking project health...");
    }

    // Read project config
    let config_path = project_root.join(".patina").join("config.json");
    let config_content =
        fs::read_to_string(&config_path).context("Failed to read project config")?;
    let config: serde_json::Value = serde_json::from_str(&config_content)?;

    // Get current environment
    let current_env = Environment::detect()?;

    // Get stored environment snapshot
    let stored_tools = config
        .get("environment_snapshot")
        .and_then(|s| s.get("detected_tools"))
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Compare environments
    let mut health_check = analyze_environment(&current_env, &stored_tools, &config)?;

    // Check project status
    let llm = config
        .get("llm")
        .and_then(|l| l.as_str())
        .unwrap_or("unknown");
    let adapter = patina::adapters::get_adapter(llm);
    let adapter_version = adapter
        .check_for_updates(&project_root)?
        .map(|(current, _)| current);

    // Count layer patterns
    let layer_path = project_root.join("layer");
    let pattern_count = count_patterns(&layer_path);

    // Count sessions
    let sessions_path = adapter.get_sessions_path(&project_root);
    let session_count = sessions_path
        .as_ref()
        .map(|path| count_sessions(path))
        .unwrap_or(0);

    health_check.project_config = ProjectStatus {
        llm: llm.to_string(),
        adapter_version,
        layer_patterns: pattern_count,
        sessions: session_count,
    };

    // Display results
    if json_output {
        println!("{}", serde_json::to_string_pretty(&health_check)?);
    } else {
        display_health_check(&health_check, &current_env)?;

        // Only provide recommendations, no auto-fixing
        if !health_check.environment_changes.missing_tools.is_empty()
            && !json_output
            && !health_check.recommendations.is_empty()
        {
            println!("\n💡 Run 'patina init .' to refresh your environment snapshot");
        }
    }

    // Determine exit code
    let exit_code = match health_check.status.as_str() {
        "healthy" => 0,
        "warning" => 2,
        "critical" => 3,
        _ => 1,
    };

    Ok(exit_code)
}

fn analyze_environment(
    current: &Environment,
    stored_tools: &[String],
    config: &serde_json::Value,
) -> Result<HealthCheck> {
    let mut missing_tools = Vec::new();
    let mut new_tools = Vec::new();
    let version_changes = Vec::new();
    let mut recommendations = Vec::new();

    // Check for missing tools
    for tool_name in stored_tools {
        if !current
            .tools
            .get(tool_name)
            .is_some_and(|info| info.available)
        {
            let required = is_tool_required(tool_name, config);
            missing_tools.push(ToolChange {
                name: tool_name.clone(),
                old_version: Some("detected".to_string()),
                new_version: None,
                required,
            });

            if required {
                recommendations.push(format!(
                    "Install {tool_name}: {}",
                    get_install_command(tool_name)
                ));
            }
        }
    }

    // Check for new tools
    for (name, info) in &current.tools {
        if info.available && !stored_tools.contains(name) {
            new_tools.push(ToolChange {
                name: name.clone(),
                old_version: None,
                new_version: info.version.clone(),
                required: false,
            });
        }
    }

    // Determine overall status
    let status = if missing_tools.iter().any(|t| t.required) {
        "critical".to_string()
    } else if !missing_tools.is_empty() {
        "warning".to_string()
    } else {
        "healthy".to_string()
    };

    Ok(HealthCheck {
        status,
        environment_changes: EnvironmentChanges {
            missing_tools,
            new_tools,
            version_changes,
        },
        project_config: ProjectStatus {
            llm: String::new(),
            adapter_version: None,
            layer_patterns: 0,
            sessions: 0,
        },
        recommendations,
    })
}

fn is_tool_required(tool: &str, config: &serde_json::Value) -> bool {
    // Check if tool is required based on project type and configuration
    match tool {
        "cargo" | "rust" => true, // Always required for Patina
        "docker" => {
            // Required if dev environment is docker
            config
                .get("dev")
                .and_then(|d| d.as_str())
                .map(|d| d == "docker")
                .unwrap_or(false)
        }
        _ => false,
    }
}

fn get_install_command(tool: &str) -> &'static str {
    match tool {
        "cargo" | "rust" => "curl https://sh.rustup.rs -sSf | sh",
        "docker" => "Visit https://docker.com/get-started",
        "git" => "brew install git (macOS) or apt install git (Linux)",
        _ => "Check your package manager",
    }
}

fn count_patterns(layer_path: &std::path::Path) -> usize {
    let mut count = 0;
    if layer_path.exists() {
        for dir in ["core", "topics", "projects"] {
            let path = layer_path.join(dir);
            if let Ok(entries) = fs::read_dir(path) {
                count += entries
                    .filter_map(Result::ok)
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                    .count();
            }
        }
    }
    count
}

fn count_sessions(sessions_path: &std::path::Path) -> usize {
    if let Ok(entries) = fs::read_dir(sessions_path) {
        entries
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .count()
    } else {
        0
    }
}

fn display_health_check(health: &HealthCheck, _env: &Environment) -> Result<()> {
    println!("\nEnvironment Changes Since Init:");

    // Display missing tools
    for tool in &health.environment_changes.missing_tools {
        let marker = if tool.required { "⚠️ " } else { "  " };
        let old_version = tool.old_version.as_deref().unwrap_or("unknown");
        let required_msg = if tool.required { " (required!)" } else { "" };
        println!(
            "  {marker} {}: {old_version} → NOT FOUND{required_msg}",
            tool.name
        );
    }

    // Display new tools
    for tool in &health.environment_changes.new_tools {
        let version = tool.new_version.as_deref().unwrap_or("detected");
        println!("  ✓ New tool: {} {version}", tool.name);
    }

    println!("\nProject Configuration:");
    let adapter_version = health
        .project_config
        .adapter_version
        .as_deref()
        .unwrap_or("unknown");
    println!(
        "  ✓ LLM: {} (adapter {adapter_version})",
        health.project_config.llm
    );
    println!(
        "  ✓ Layer: {} patterns stored",
        health.project_config.layer_patterns
    );
    println!("  ✓ Sessions: {} recorded", health.project_config.sessions);

    if !health.recommendations.is_empty() {
        println!("\nRecommendations:");
        for (i, rec) in health.recommendations.iter().enumerate() {
            println!("  {}. {rec}", i + 1);
        }
    }

    Ok(())
}

// ============================================================================
// REFERENCE REPOSITORY MANAGEMENT
// ============================================================================

#[derive(Debug)]
struct RepoInfo {
    name: String,
    path: PathBuf,
}

#[derive(Debug)]
enum RepoStatus {
    UpToDate,
    Behind(usize),
    Dirty,
    DetachedHead,
    Error(String),
}

fn handle_repos(update: bool) -> Result<i32> {
    println!("🔍 Checking reference repositories in layer/dust/repos/...\n");

    let repos_dir = Path::new("layer/dust/repos");
    if !repos_dir.exists() {
        println!("No reference repositories directory found.");
        println!("Create one with: mkdir -p layer/dust/repos");
        return Ok(0);
    }

    // Discover all git repositories
    let repos = discover_repos(repos_dir)?;
    if repos.is_empty() {
        println!("No git repositories found in layer/dust/repos/");
        return Ok(0);
    }

    // Check status of each repo
    let mut stale_repos = Vec::new();
    let mut total_repos = 0;

    for repo in &repos {
        total_repos += 1;
        let status = check_repo_status(repo)?;

        match &status {
            RepoStatus::UpToDate => {
                println!("✓ {} - up to date", repo.name);
                log_repo_status(&repo.name, "CHECK", "up to date")?;
            }
            RepoStatus::Behind(count) => {
                println!("⚠ {} - {} commits behind origin", repo.name, count);
                stale_repos.push((repo, *count));
                log_repo_status(&repo.name, "CHECK", &format!("behind: {} commits", count))?;
            }
            RepoStatus::Dirty => {
                println!("✗ {} - has local changes, skipping", repo.name);
                log_repo_status(&repo.name, "SKIP", "local changes present")?;
            }
            RepoStatus::DetachedHead => {
                println!("⚠ {} - on detached HEAD, skipping", repo.name);
                log_repo_status(&repo.name, "SKIP", "detached HEAD")?;
            }
            RepoStatus::Error(err) => {
                println!("✗ {} - error: {}", repo.name, err);
                log_repo_status(&repo.name, "ERROR", err)?;
            }
        }
    }

    // Summary
    println!();
    if stale_repos.is_empty() {
        println!("✅ All {} repos up to date", total_repos);
        return Ok(0);
    }

    println!(
        "{} of {} repos need updates",
        stale_repos.len(),
        total_repos
    );

    if !update {
        println!("\n💡 Run: patina doctor --repos --update");
        return Ok(2); // Warning exit code
    }

    // Perform updates
    println!("\n🔄 Updating reference repositories...\n");
    let mut updated_count = 0;
    let mut stale_dbs = Vec::new();

    for (repo, _count) in stale_repos {
        print!("⚠ {} - pulling changes... ", repo.name);
        match update_repo(repo) {
            Ok(commits) => {
                println!("✓ Updated: {}", commits);
                updated_count += 1;
                log_repo_status(&repo.name, "UPDATE", &format!("pulled: {}", commits))?;

                // Mark database as stale
                let db_name = format!("{}.db", repo.name);
                stale_dbs.push(db_name.clone());
                log_repo_status(&db_name, "STALE", "needs rescrape")?;
            }
            Err(e) => {
                println!("✗ Failed: {}", e);
                log_repo_status(&repo.name, "ERROR", &format!("update failed: {}", e))?;
            }
        }
    }

    // Summary
    println!();
    if updated_count > 0 {
        println!(
            "✅ Updated {} repos. Their databases may be stale:",
            updated_count
        );
        for db in &stale_dbs {
            println!("  • layer/dust/repos/{}", db);
        }
        println!("\n💡 To refresh: patina scrape code --repo <repo-name> --force");
        println!("💡 Or batch update later with: patina scrape --sync-updated-repos (coming soon)");
    }

    Ok(0)
}

fn discover_repos(repos_dir: &Path) -> Result<Vec<RepoInfo>> {
    let mut repos = Vec::new();

    for entry in fs::read_dir(repos_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let git_dir = path.join(".git");
            if git_dir.exists() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    repos.push(RepoInfo {
                        name: name.to_string(),
                        path,
                    });
                }
            }
        }
    }

    repos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(repos)
}

fn check_repo_status(repo: &RepoInfo) -> Result<RepoStatus> {
    // Check if working tree is clean
    let status_output = Command::new("git")
        .arg("-C")
        .arg(&repo.path)
        .arg("status")
        .arg("--porcelain")
        .output()?;

    if !status_output.stdout.is_empty() {
        return Ok(RepoStatus::Dirty);
    }

    // Check if on a branch (not detached HEAD)
    let branch_output = Command::new("git")
        .arg("-C")
        .arg(&repo.path)
        .arg("symbolic-ref")
        .arg("-q")
        .arg("HEAD")
        .output();

    if branch_output.is_err() || !branch_output.unwrap().status.success() {
        return Ok(RepoStatus::DetachedHead);
    }

    // Fetch from origin (quietly)
    let fetch_result = Command::new("git")
        .arg("-C")
        .arg(&repo.path)
        .arg("fetch")
        .arg("origin")
        .arg("--quiet")
        .output();

    if let Err(e) = fetch_result {
        return Ok(RepoStatus::Error(format!("fetch failed: {}", e)));
    }

    // Check how many commits behind
    let behind_output = Command::new("git")
        .arg("-C")
        .arg(&repo.path)
        .arg("rev-list")
        .arg("HEAD..@{u}")
        .arg("--count")
        .output()?;

    if !behind_output.status.success() {
        return Ok(RepoStatus::Error(
            "failed to check commit count".to_string(),
        ));
    }

    let count_str = String::from_utf8_lossy(&behind_output.stdout);
    let count: usize = count_str.trim().parse().unwrap_or(0);

    if count > 0 {
        Ok(RepoStatus::Behind(count))
    } else {
        Ok(RepoStatus::UpToDate)
    }
}

fn update_repo(repo: &RepoInfo) -> Result<String> {
    // Get current commit before pull
    let before_output = Command::new("git")
        .arg("-C")
        .arg(&repo.path)
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .output()?;
    let before = String::from_utf8_lossy(&before_output.stdout)
        .trim()
        .to_string();

    // Pull with fast-forward only
    let pull_output = Command::new("git")
        .arg("-C")
        .arg(&repo.path)
        .arg("pull")
        .arg("--ff-only")
        .output()?;

    if !pull_output.status.success() {
        let stderr = String::from_utf8_lossy(&pull_output.stderr);
        anyhow::bail!("git pull failed: {}", stderr);
    }

    // Get commit after pull
    let after_output = Command::new("git")
        .arg("-C")
        .arg(&repo.path)
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .output()?;
    let after = String::from_utf8_lossy(&after_output.stdout)
        .trim()
        .to_string();

    // Count commits pulled
    let count_output = Command::new("git")
        .arg("-C")
        .arg(&repo.path)
        .arg("rev-list")
        .arg("--count")
        .arg(format!("{}..{}", before, after))
        .output()?;
    let count = String::from_utf8_lossy(&count_output.stdout)
        .trim()
        .to_string();

    Ok(format!("{}..{} ({} commits)", before, after, count))
}

fn log_repo_status(name: &str, action: &str, message: &str) -> Result<()> {
    use std::io::Write;
    use std::time::SystemTime;

    let log_path = Path::new("layer/dust/repos/.patina-update.log");

    // Ensure directory exists
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    // Use ISO 8601 format
    let datetime =
        chrono::DateTime::from_timestamp(timestamp as i64, 0).unwrap_or_else(chrono::Utc::now);

    writeln!(
        file,
        "{} | {:8} | {:20} | {}",
        datetime.format("%Y-%m-%dT%H:%M:%SZ"),
        action,
        name,
        message
    )?;

    Ok(())
}
