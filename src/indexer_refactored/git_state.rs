//! Git state definitions and confidence scoring

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Git states that affect navigation confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitState {
    /// File exists but not tracked by git
    Untracked {
        detected_at: DateTime<Utc>,
        files: Vec<PathBuf>,
    },

    /// File has been modified
    Modified {
        files: Vec<PathBuf>,
        has_staged: bool,
        last_change: DateTime<Utc>,
    },

    /// Changes staged for commit
    Staged {
        files: Vec<PathBuf>,
        staged_at: DateTime<Utc>,
    },

    /// Changes committed locally
    Committed {
        sha: String,
        message: String,
        timestamp: DateTime<Utc>,
        files: Vec<PathBuf>,
    },

    /// Pushed to remote
    Pushed {
        remote: String,
        branch: String,
        sha: String,
    },

    /// Pull request opened
    PullRequest {
        number: u32,
        url: String,
        base_branch: String,
        state: PRState,
    },

    /// Merged into another branch
    Merged {
        into_branch: String,
        merge_sha: String,
        timestamp: DateTime<Utc>,
    },

    /// Archived/deprecated
    Archived {
        reason: ArchiveReason,
        moved_to: crate::indexer::Layer,
        archived_at: DateTime<Utc>,
    },
}

/// Pull request states
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PRState {
    Open,
    Closed,
    Merged,
}

/// Reasons for archiving patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchiveReason {
    Deprecated,
    Superseded { by: String },
    Moved { to: String },
    Other(String),
}

/// Navigation confidence levels based on git state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Confidence {
    /// Archived/historical patterns
    Historical = 0,
    /// Experimental/untracked patterns
    Experimental = 1,
    /// Low confidence (modified/staged)
    Low = 2,
    /// Medium confidence (committed locally)
    Medium = 3,
    /// High confidence (pushed/PR)
    High = 4,
    /// Verified patterns (merged to main)
    Verified = 5,
}

impl GitState {
    /// Calculate navigation confidence from git state
    pub fn confidence(&self) -> Confidence {
        match self {
            GitState::Untracked { .. } => Confidence::Experimental,
            GitState::Modified { .. } | GitState::Staged { .. } => Confidence::Low,
            GitState::Committed { .. } => Confidence::Medium,
            GitState::Pushed { .. } => Confidence::High,
            GitState::PullRequest {
                state: PRState::Open,
                ..
            } => Confidence::High,
            GitState::PullRequest {
                state: PRState::Merged,
                ..
            } => Confidence::Verified,
            GitState::Merged { into_branch, .. } => {
                if into_branch == "main" || into_branch == "master" {
                    Confidence::Verified
                } else {
                    Confidence::High
                }
            }
            GitState::Archived { .. } => Confidence::Historical,
            _ => Confidence::Low,
        }
    }

    /// Check if this state makes the pattern searchable
    pub fn is_searchable(&self) -> bool {
        !matches!(self, GitState::Untracked { .. })
    }

    /// Get a human-readable description of the state
    pub fn description(&self) -> String {
        match self {
            GitState::Untracked { .. } => "Untracked (experimental)".to_string(),
            GitState::Modified {
                has_staged: true, ..
            } => "Modified with staged changes".to_string(),
            GitState::Modified {
                has_staged: false, ..
            } => "Modified (unstaged)".to_string(),
            GitState::Staged { .. } => "Staged for commit".to_string(),
            GitState::Committed { message, .. } => format!("Committed: {message}"),
            GitState::Pushed { branch, .. } => format!("Pushed to {branch}"),
            GitState::PullRequest { number, state, .. } => {
                format!(
                    "PR #{} ({})",
                    number,
                    match state {
                        PRState::Open => "open",
                        PRState::Closed => "closed",
                        PRState::Merged => "merged",
                    }
                )
            }
            GitState::Merged { into_branch, .. } => format!("Merged into {into_branch}"),
            GitState::Archived { reason, .. } => format!("Archived: {reason:?}"),
        }
    }
}

/// Git events that trigger state transitions
#[derive(Debug, Clone)]
pub enum GitEvent {
    FileCreated {
        path: PathBuf,
        workspace_id: String,
    },
    FileModified {
        path: PathBuf,
        workspace_id: String,
    },
    FileStaged {
        files: Vec<PathBuf>,
        workspace_id: String,
    },
    Commit {
        sha: String,
        message: String,
        files: Vec<PathBuf>,
        workspace_id: String,
    },
    Push {
        remote: String,
        branch: String,
        workspace_id: String,
    },
    PROpened {
        number: u32,
        url: String,
        workspace_id: String,
    },
    Merged {
        into_branch: String,
        workspace_id: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_ordering() {
        assert!(Confidence::Experimental < Confidence::Low);
        assert!(Confidence::Low < Confidence::Medium);
        assert!(Confidence::Medium < Confidence::High);
        assert!(Confidence::High < Confidence::Verified);
        assert!(Confidence::Historical < Confidence::Experimental);
    }

    #[test]
    fn test_git_state_confidence() {
        let untracked = GitState::Untracked {
            detected_at: Utc::now(),
            files: vec![],
        };
        assert_eq!(untracked.confidence(), Confidence::Experimental);

        let merged = GitState::Merged {
            into_branch: "main".to_string(),
            merge_sha: "abc123".to_string(),
            timestamp: Utc::now(),
        };
        assert_eq!(merged.confidence(), Confidence::Verified);
    }
}
