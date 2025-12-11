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
    add_all, add_remote, branch_exists, branch_rename, checkout, checkout_new_branch, commit,
    commits_behind, current_branch, default_branch, fetch, has_remote, is_clean, is_git_repo,
    rebase, rebase_abort, remote_url, repo_name, stash_push, status_count,
};
pub use validation::ensure_patina_branch;

use chrono::Utc;

/// Get current timestamp for backups
pub fn timestamp() -> String {
    Utc::now().format("%Y%m%d-%H%M%S").to_string()
}
