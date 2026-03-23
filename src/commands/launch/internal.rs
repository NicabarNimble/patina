//! Internal implementation for launch command
//!
//! Handles the launch flow: workspace check → mother → project check → bootstrap → launch

use anyhow::{bail, Context, Result};
use std::env;
use std::fs;
use std::io::{self, Read as _, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use patina::git;
use patina::interface::launch as interfaces;
use patina::paths;
use patina::project;

use super::LaunchOptions;

/// Main launch entry point
pub fn launch(options: LaunchOptions) -> Result<()> {
    let project_path = resolve_project_path(options.path.as_deref())?;
    let explicit_adapter: Option<String> = options.adapter.clone();
    if let Some(ref name) = explicit_adapter {
        let adapter_info = interfaces::get(name)?;
        if !adapter_info.detected {
            bail!(
                "Adapter '{}' ({}) is not installed.\n\
                 Install it and try again, or use a different adapter.",
                name,
                adapter_info.display
            );
        }
    }

    let patina_dir = project_path.join(".patina");
    let adapter_name: String;

    if !patina_dir.exists() {
        if options.auto_init {
            // Pass explicit_adapter - if Some, skip selection prompt
            match prompt_are_you_lost(&project_path, explicit_adapter.as_deref())? {
                Some(selected) => {
                    // Update adapter_name to what user selected (or explicit)
                    adapter_name = selected;
                }
                None => {
                    // User declined
                    return Ok(());
                }
            }
        } else {
            bail!(
                "Not a patina project (no .patina/ directory).\n\
                 Run `patina init .` first."
            );
        }
    } else {
        // Existing project - resolve interface name
        // Priority: explicit flag > project default > global default
        let project_config = project::load_with_migration(&project_path)?;
        adapter_name = explicit_adapter.unwrap_or_else(|| {
            // Use project default if set, otherwise fall back to global
            if !project_config.adapters.default.is_empty() {
                project_config.adapters.default.clone()
            } else {
                interfaces::default_interface_name().unwrap_or_else(|_| "claude".to_string())
            }
        });

        // Validate interface is installed
        let adapter_info = interfaces::get(&adapter_name)?;
        if !adapter_info.detected {
            bail!(
                "Interface '{}' ({}) is not installed.\n\
                 Install it and try again, or use a different interface.",
                adapter_name,
                adapter_info.display
            );
        }

        println!(
            "🚀 Launching {} in {}",
            adapter_info.display,
            project_path.display()
        );
    }

    crate::commands::ai::surface::launch(crate::commands::ai::surface::AiLaunchRequest {
        interface_name: adapter_name,
        title: None,
        requested_session: None,
        persona: None,
        path: Some(project_path.display().to_string()),
        set_default: false,
        tmux: false,
        no_tmux: false,
    })
}

/// Resolve project path from options or current directory
pub(crate) fn resolve_project_path(path_opt: Option<&str>) -> Result<PathBuf> {
    let path = match path_opt {
        Some(p) => PathBuf::from(shellexpand::tilde(p).as_ref()),
        None => env::current_dir().context("Failed to get current directory")?,
    };

    // Canonicalize to resolve symlinks
    let canonical = fs::canonicalize(&path)
        .with_context(|| format!("Project path does not exist: {}", path.display()))?;

    Ok(canonical)
}

/// Check if mother is running via UDS health endpoint
pub fn check_mother_health() -> bool {
    let sock_path = paths::serve::socket_path();
    let mut stream = match std::os::unix::net::UnixStream::connect(&sock_path) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));

    let request = "GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n";
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }

    let mut buf = vec![0u8; 1024];
    match stream.read(&mut buf) {
        Ok(n) if n > 0 => {
            let response = String::from_utf8_lossy(&buf[..n]);
            response.contains("200")
        }
        _ => false,
    }
}

/// Ensure mother is running, start if needed
pub(crate) fn ensure_mother_running() -> Result<()> {
    if check_mother_health() {
        println!("  ✓ Mother running");
        return Ok(());
    }

    println!("  ⏳ Starting mother...");
    start_mother_daemon()?;

    // Wait for it to come up
    for _ in 0..10 {
        thread::sleep(Duration::from_millis(500));
        if check_mother_health() {
            println!("  ✓ Mother started");
            return Ok(());
        }
    }

    bail!("Failed to start mother daemon")
}

/// Start mother as background daemon
pub fn start_mother_daemon() -> Result<()> {
    let patina_bin = env::current_exe().context("getting current executable path")?;

    Command::new(&patina_bin)
        .args(["mother", "start"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("spawning mother daemon")?;

    Ok(())
}

/// "Are you lost?" prompt - show git context and offer to initialize.
///
/// Returns:
/// - Ok(None) - user declined to init
/// - Ok(Some(interface_name)) - user accepted, project initialized with this interface
///
/// If `explicit_adapter` is Some, uses that interface without prompting for selection.
/// If None, detects available interfaces and prompts user to choose.
pub(crate) fn prompt_are_you_lost(
    project_path: &Path,
    explicit_adapter: Option<&str>,
) -> Result<Option<String>> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Are you lost?");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("This is not a patina project.\n");

    // Show path
    println!("📁 Path: {}", project_path.display());

    // Show git context if available
    if git::is_git_repo().unwrap_or(false) {
        let branch = git::current_branch().unwrap_or_else(|_| "unknown".to_string());
        let clean = git::is_clean().unwrap_or(true);
        let status = if clean {
            "clean".to_string()
        } else {
            let count = git::status_count().unwrap_or(0);
            format!("{} files modified", count)
        };
        println!("🔀 Git:  {} ({})", branch, status);

        // Show remote if available
        if let Ok(url) = git::remote_url("origin") {
            let display_url = format_remote_url(&url);
            println!("🌐 Remote: {}", display_url);
        }
    } else {
        println!("🔀 Git:  not a git repository");
    }

    println!();
    print!("Initialize as patina project? [y/N]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let should_init = input.trim().to_lowercase() == "y";

    if !should_init {
        return Ok(None);
    }

    // User wants to init - determine which interface to use
    let adapter_name = if let Some(explicit) = explicit_adapter {
        // Flow A: explicit interface from --interface flag
        explicit.to_string()
    } else {
        // Flow B: detect available interfaces and let user choose
        let all_adapters = interfaces::list()?;
        let available: Vec<_> = all_adapters.into_iter().filter(|a| a.detected).collect();

        // Get global default as preference
        let preference = interfaces::default_interface_name().ok();

        interfaces::select_interface(&available, preference.as_deref())?
    };

    // Initialize the project with selected interface
    println!();
    if initialize_project(project_path, &adapter_name)? {
        Ok(Some(adapter_name))
    } else {
        Ok(None)
    }
}

/// Format remote URL for display (strip git@/https://, .git suffix)
fn format_remote_url(url: &str) -> String {
    url.trim()
        .strip_prefix("git@")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(url)
        .replace(":", "/")
        .strip_suffix(".git")
        .unwrap_or(url)
        .to_string()
}

/// Branch safety result indicating what action was taken
#[derive(Debug)]
pub enum BranchAction {
    /// Already on patina, no action needed
    AlreadyOnPatina,
    /// Switched to patina (was clean)
    Switched { _from: String },
    /// Stashed and switched to patina
    StashedAndSwitched { _from: String, _stash_name: String },
    /// Rebased patina onto main
    Rebased { _commits: usize },
    /// Conflicts during rebase - user must resolve
    RebaseConflicts,
    /// Not a git repo
    NotGitRepo,
    /// No patina branch exists
    NoPatinaExists,
}

/// Ensure we're on patina branch using "Do and Inform" model
/// Returns the action taken so caller can display appropriate message
pub(crate) fn ensure_on_patina_branch() -> Result<BranchAction> {
    // Check if this is a git repo
    if !git::is_git_repo()? {
        return Ok(BranchAction::NotGitRepo);
    }

    let current = git::current_branch()?;

    // Check if patina branch exists
    if !git::branch_exists("patina")? {
        return Ok(BranchAction::NoPatinaExists);
    }

    // Already on patina?
    if current == "patina" {
        // Try to fetch to get latest
        let _ = git::fetch("origin"); // Ignore fetch errors (might be offline)

        // Check if behind origin/patina (not main!) and auto-rebase
        // Rebasing onto main was wrong - it linearizes history and breaks merges
        // We only want to sync local patina with remote patina
        let behind = git::commits_behind("patina", "origin/patina").unwrap_or(0);

        if behind > 0 {
            println!(
                "\n📥 Patina branch is {} commits behind origin/patina",
                behind
            );
            println!("   Rebasing onto origin/patina...");

            if git::rebase("origin/patina")? {
                println!("   ✓ Rebased ({} commits)", behind);
                return Ok(BranchAction::Rebased { _commits: behind });
            } else {
                println!("   ✗ Rebase failed (conflicts)");
                println!();
                println!("   To resolve:");
                println!("   1. Fix conflicts");
                println!("   2. git add <files>");
                println!("   3. git rebase --continue");
                println!();
                println!("   Or abort: git rebase --abort");
                return Ok(BranchAction::RebaseConflicts);
            }
        }

        return Ok(BranchAction::AlreadyOnPatina);
    }

    // On another branch, patina exists - need to switch
    let clean = git::is_clean()?;

    if clean {
        // Clean working tree - just switch
        println!("\n🔀 Switching to patina branch...");
        git::checkout("patina")?;
        println!("   ✓ Switched to patina");
        return Ok(BranchAction::Switched { _from: current });
    }

    // Dirty working tree - stash first
    let timestamp = git::timestamp();
    let stash_name = format!("patina-autostash-{}", timestamp);

    println!("\n📦 Stashing changes on '{}'...", current);
    git::stash_push(&stash_name)?;
    println!("   ✓ Stashed: \"{}\"", stash_name);

    println!("🔀 Switching to patina branch...");
    git::checkout("patina")?;
    println!("   ✓ Switched to patina");

    println!();
    println!("────────────────────────────────────────────────");
    println!("💡 Your changes on '{}' are stashed.", current);
    println!("   To restore: git checkout {} && git stash pop", current);
    println!("────────────────────────────────────────────────");

    Ok(BranchAction::StashedAndSwitched {
        _from: current,
        _stash_name: stash_name,
    })
}

/// Initialize project from the "Are you lost?" prompt
fn initialize_project(project_path: &Path, adapter_name: &str) -> Result<bool> {
    // Change to project directory for init
    let original_dir = env::current_dir()?;
    env::set_current_dir(project_path)?;

    // Step 1: Create skeleton
    let init_result = crate::commands::init::execute(
        ".".to_string(), // Use "." to trigger commit step in init
        false,           // force
        true,            // local (skip GitHub integration for quick init)
        false,           // no_commit (allow auto-commit)
    );

    if let Err(e) = init_result {
        env::set_current_dir(original_dir)?;
        eprintln!("\n❌ Failed to initialize: {}", e);
        return Ok(false);
    }

    // Step 2: Prepare the Patina AI surface and establish the selected default
    let adapter_result =
        crate::commands::ai::surface::setup(crate::commands::ai::surface::AiSetupRequest {
            interface: Some(adapter_name.to_string()),
            path: Some(project_path.display().to_string()),
            force: false,
        });

    if let Err(e) = adapter_result {
        env::set_current_dir(original_dir)?;
        eprintln!("\n❌ Failed to prepare Patina AI setup: {}", e);
        eprintln!("   Run 'patina ai setup' to retry manually");
        return Ok(false);
    }

    // Restore original directory
    env::set_current_dir(original_dir)?;

    println!(
        "\n✓ Initialized as patina project with {} as the default AI interface",
        adapter_name
    );
    Ok(true) // Continue to launch
}

#[cfg_attr(not(test), allow(dead_code))]
/// Try to get the Claude OAuth token from the global secrets vault.
///
/// Checks conflict guards first (ANTHROPIC_API_KEY, CLAUDE_CODE_OAUTH_TOKEN),
/// then attempts vault lookup. Returns None on any failure — never propagates errors.
fn try_get_claude_token() -> Option<String> {
    // Conflict guard: ANTHROPIC_API_KEY takes priority
    if env::var("ANTHROPIC_API_KEY").is_ok() {
        eprintln!("patina: ANTHROPIC_API_KEY set — skipping vault token injection (API key takes priority)");
        return None;
    }

    // Conflict guard: CLAUDE_CODE_OAUTH_TOKEN already set externally
    if env::var("CLAUDE_CODE_OAUTH_TOKEN").is_ok() {
        eprintln!("patina: CLAUDE_CODE_OAUTH_TOKEN already set — skipping vault token injection");
        return None;
    }

    // Attempt vault lookup — catch all errors
    match patina::secrets::get_global_secret("claude-oauth") {
        Ok(Some(token)) => Some(token),
        Ok(None) => None,
        Err(e) => {
            eprintln!("patina: failed to read claude-oauth from vault — {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    // Serialize env-var tests to avoid races (env is process-global)
    #[test]
    fn test_resolve_current_dir() {
        let path = resolve_project_path(None);
        assert!(path.is_ok());
        assert!(path.unwrap().is_absolute());
    }

    #[test]
    fn test_resolve_tilde_path() {
        let path = resolve_project_path(Some("~"));
        // This should work if home dir exists
        if let Ok(p) = path {
            assert!(p.is_absolute());
        }
    }

    // --- try_get_claude_token conflict guards ---

    #[test]
    fn test_claude_token_blocked_by_anthropic_api_key() {
        let _lock = patina::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        // Save and set
        let prev = env::var("ANTHROPIC_API_KEY").ok();
        env::set_var("ANTHROPIC_API_KEY", "sk-ant-test");
        // Clear the other to avoid interference
        let prev_oauth = env::var("CLAUDE_CODE_OAUTH_TOKEN").ok();
        env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");

        let result = try_get_claude_token();
        assert!(
            result.is_none(),
            "should skip when ANTHROPIC_API_KEY is set"
        );

        // Restore
        match prev {
            Some(v) => env::set_var("ANTHROPIC_API_KEY", v),
            None => env::remove_var("ANTHROPIC_API_KEY"),
        }
        match prev_oauth {
            Some(v) => env::set_var("CLAUDE_CODE_OAUTH_TOKEN", v),
            None => env::remove_var("CLAUDE_CODE_OAUTH_TOKEN"),
        }
    }

    #[test]
    fn test_claude_token_blocked_by_existing_oauth_token() {
        let _lock = patina::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        // Save and set
        let prev_api = env::var("ANTHROPIC_API_KEY").ok();
        env::remove_var("ANTHROPIC_API_KEY");
        let prev = env::var("CLAUDE_CODE_OAUTH_TOKEN").ok();
        env::set_var("CLAUDE_CODE_OAUTH_TOKEN", "sk-ant-oat01-existing");

        let result = try_get_claude_token();
        assert!(
            result.is_none(),
            "should skip when CLAUDE_CODE_OAUTH_TOKEN already set"
        );

        // Restore
        match prev {
            Some(v) => env::set_var("CLAUDE_CODE_OAUTH_TOKEN", v),
            None => env::remove_var("CLAUDE_CODE_OAUTH_TOKEN"),
        }
        match prev_api {
            Some(v) => env::set_var("ANTHROPIC_API_KEY", v),
            None => env::remove_var("ANTHROPIC_API_KEY"),
        }
    }

    #[test]
    fn test_claude_token_clean_env_attempts_vault() {
        let _lock = patina::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        // Save and clear both
        let prev_api = env::var("ANTHROPIC_API_KEY").ok();
        let prev_oauth = env::var("CLAUDE_CODE_OAUTH_TOKEN").ok();
        env::remove_var("ANTHROPIC_API_KEY");
        env::remove_var("CLAUDE_CODE_OAUTH_TOKEN");

        // With clean env, function should reach the vault lookup path
        // without panicking. Result depends on environment:
        // - No vault → None
        // - Vault with claude-oauth → Some(token)
        // - Vault without claude-oauth → None
        // The key assertion: no conflict guard fires, no panic.
        let _result = try_get_claude_token();

        // Restore
        match prev_api {
            Some(v) => env::set_var("ANTHROPIC_API_KEY", v),
            None => env::remove_var("ANTHROPIC_API_KEY"),
        }
        match prev_oauth {
            Some(v) => env::set_var("CLAUDE_CODE_OAUTH_TOKEN", v),
            None => env::remove_var("CLAUDE_CODE_OAUTH_TOKEN"),
        }
    }

    #[test]
    fn initialize_project_prepares_unified_ai_surface_and_default() {
        let _lock = patina::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp = TempDir::new().unwrap();

        let status = Command::new("git")
            .args(["init"])
            .current_dir(temp.path())
            .status()
            .unwrap();
        assert!(status.success());

        let status = Command::new("git")
            .args(["config", "user.email", "patina@example.com"])
            .current_dir(temp.path())
            .status()
            .unwrap();
        assert!(status.success());

        let status = Command::new("git")
            .args(["config", "user.name", "Patina Tests"])
            .current_dir(temp.path())
            .status()
            .unwrap();
        assert!(status.success());

        let old_dir = env::current_dir().ok();
        let old_patina_home = env::var_os("PATINA_HOME");
        let patina_home = temp.path().join("patina-home");
        fs::create_dir_all(&patina_home).unwrap();

        unsafe {
            env::set_var("PATINA_HOME", &patina_home);
        }
        env::set_current_dir(temp.path()).unwrap();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            initialize_project(temp.path(), "gemini")
        }));

        if let Some(path) = old_dir {
            let _ = env::set_current_dir(path);
        }
        match old_patina_home {
            Some(value) => unsafe {
                env::set_var("PATINA_HOME", value);
            },
            None => unsafe {
                env::remove_var("PATINA_HOME");
            },
        }

        let result = match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        };
        assert!(result.unwrap());

        let config = project::load_with_migration(temp.path()).unwrap();
        assert_eq!(config.adapters.default, "gemini");
        assert!(config.adapters.allowed.iter().any(|name| name == "claude"));
        assert!(config
            .adapters
            .allowed
            .iter()
            .any(|name| name == "opencode"));
        assert!(config.adapters.allowed.iter().any(|name| name == "gemini"));
        assert!(temp.path().join("AGENTS.md").exists());
        assert!(temp.path().join("CLAUDE.md").exists());
        assert!(temp.path().join("GEMINI.md").exists());
        assert!(temp
            .path()
            .join(".claude/commands/session-start.md")
            .exists());
        assert!(temp
            .path()
            .join(".opencode/commands/session-start.md")
            .exists());
        assert!(temp
            .path()
            .join(".gemini/commands/session-start.toml")
            .exists());
    }
}
