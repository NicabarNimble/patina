//! Internal implementation for hook commands
//!
//! Forks `patina scrape` as a detached child process so git commit returns
//! instantly. Logs output to `.patina/local/hook.log`.
//!
//! Git sets PWD to the worktree root before invoking post-commit hooks —
//! std::env::current_dir() finds .patina/ automatically. No --repo-root
//! flag or git rev-parse needed.

use anyhow::Result;
use std::fs::{self, OpenOptions};
use std::process::{Command, Stdio};

/// Log file for hook output (inside .patina/local/, gitignored)
const HOOK_LOG: &str = ".patina/local/hook.log";

/// Fork scrape to background and emit hook event.
pub fn handle_post_commit() -> Result<()> {
    fork_scrape("post-commit")
}

/// Fork scrape to background and emit hook event.
pub fn handle_post_merge() -> Result<()> {
    fork_scrape("post-merge")
}

/// Shared implementation: fork `patina scrape` to background, emit event.
fn fork_scrape(hook_name: &str) -> Result<()> {
    // Ensure log directory exists
    if let Some(parent) = std::path::Path::new(HOOK_LOG).parent() {
        fs::create_dir_all(parent)?;
    }

    // Open log file (append mode, create if missing)
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(HOOK_LOG)?;

    let stderr_file = log_file.try_clone()?;

    // Emit hook event before forking (fast — single DB insert)
    emit_hook_event(hook_name);

    // Fork patina scrape as detached child
    // stdout/stderr go to hook.log, stdin is null
    match Command::new("patina")
        .arg("scrape")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
    {
        Ok(_child) => {
            // Child is detached — we don't wait for it.
            // On Unix, the child continues running after parent exits.
        }
        Err(e) => {
            eprintln!("patina: hook: failed to spawn scrape: {e}");
        }
    }

    Ok(())
}

/// Emit a hook event to events.db (best-effort, never blocks git).
fn emit_hook_event(hook_name: &str) {
    let metrics = serde_json::json!({
        "hook": hook_name,
    });
    // Use "capture" verb — hooks trigger scrape which is a capture operation.
    // Use emit_or_warn so a DB error doesn't block git commit.
    patina::measure::emit_or_warn("capture", "hook", hook_name, &metrics);
}
