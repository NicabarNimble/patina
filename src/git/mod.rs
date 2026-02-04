//! Git repository management for Patina
//!
//! Handles:
//! - Repository detection and validation
//! - Fork detection and creation
//! - Branch management with edge cases
//! - Working tree status checks

mod fork;
mod operations;
mod validation;

pub use fork::{detect_fork_status, ensure_fork, ForkStatus};
pub use operations::{
    add_all, add_paths, add_remote, branch_exists, branch_rename, checkout, checkout_new_branch,
    commit, commits_ahead, commits_behind, commits_behind_upstream, commits_since_count,
    create_tag, current_branch, default_branch, diff_stat_summary, fetch, files_changed_since,
    has_remote, has_staged_changes, has_upstream, head_sha, is_clean, is_diverged, is_git_repo,
    last_commit_message, last_commit_relative_time, log_oneline, rebase, rebase_abort, remote_url,
    repo_name, short_sha, stash_push, status_count, status_porcelain, tag_exists,
};
pub use validation::ensure_patina_branch;

use chrono::Utc;

/// Get current timestamp for backups
pub fn timestamp() -> String {
    Utc::now().format("%Y%m%d-%H%M%S").to_string()
}
