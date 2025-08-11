//! Git navigation state machine for tracking pattern lifecycle

use super::git_state::PRState;
use super::{GitAwareNavigationMap, GitEvent, GitState};
use crate::workspace_client::WorkspaceClient;
use anyhow::Result;
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;

/// State machine that processes git events and updates navigation state
pub struct GitNavigationStateMachine {
    /// Navigation map to update
    navigation_map: Arc<Mutex<GitAwareNavigationMap>>,

    /// Workspace client for git operations
    workspace_client: Option<WorkspaceClient>,

    /// State transition history
    state_transitions: Vec<StateTransition>,

    /// Current states by file path
    file_states: HashMap<PathBuf, GitState>,
}

/// Record of a state transition
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub workspace_id: String,
    pub file_path: Option<PathBuf>,
    pub from_state: String,
    pub to_state: String,
    pub timestamp: chrono::DateTime<Utc>,
    pub metadata: serde_json::Value,
}

impl GitNavigationStateMachine {
    /// Create a new state machine
    pub fn new() -> Result<Self> {
        Ok(Self {
            navigation_map: Arc::new(Mutex::new(GitAwareNavigationMap::new())),
            workspace_client: None,
            state_transitions: Vec::new(),
            file_states: HashMap::new(),
        })
    }

    /// Create with shared navigation map
    pub fn with_navigation_map(map: Arc<Mutex<GitAwareNavigationMap>>) -> Self {
        Self {
            navigation_map: map,
            workspace_client: None,
            state_transitions: Vec::new(),
            file_states: HashMap::new(),
        }
    }

    /// Set workspace client for git operations
    pub fn set_workspace_client(&mut self, client: WorkspaceClient) {
        self.workspace_client = Some(client);
    }

    /// Process a git event and update navigation state
    pub fn process_event(&mut self, event: GitEvent) -> Result<()> {
        match event {
            GitEvent::FileCreated { path, workspace_id } => {
                self.handle_file_created(path, workspace_id)?;
            }
            GitEvent::FileModified { path, workspace_id } => {
                self.handle_file_modified(path, workspace_id)?;
            }
            GitEvent::FileStaged {
                files,
                workspace_id,
            } => {
                self.handle_files_staged(files, workspace_id)?;
            }
            GitEvent::Commit {
                sha,
                message,
                files,
                workspace_id,
            } => {
                self.handle_commit(sha, message, files, workspace_id)?;
            }
            GitEvent::Push {
                remote,
                branch,
                workspace_id,
            } => {
                self.handle_push(remote, branch, workspace_id)?;
            }
            GitEvent::PROpened {
                number,
                url,
                workspace_id,
            } => {
                self.handle_pr_opened(number, url, workspace_id)?;
            }
            GitEvent::Merged {
                into_branch,
                workspace_id,
            } => {
                self.handle_merge(into_branch, workspace_id)?;
            }
        }
        Ok(())
    }

    /// Handle file creation
    fn handle_file_created(&mut self, path: PathBuf, workspace_id: String) -> Result<()> {
        let new_state = GitState::Untracked {
            detected_at: Utc::now(),
            files: vec![path.clone()],
        };

        // Update state
        self.file_states.insert(path.clone(), new_state.clone());

        // Update navigation map
        {
            let mut map = self.navigation_map.lock().unwrap();
            map.update_git_state(&path, new_state);
        } // Drop the lock here

        // Record transition
        self.record_transition(
            workspace_id,
            Some(path),
            "none",
            "untracked",
            json!({"event": "file_created"}),
        );

        Ok(())
    }

    /// Handle file modification
    fn handle_file_modified(&mut self, path: PathBuf, workspace_id: String) -> Result<()> {
        let current_state = self.file_states.get(&path).cloned();

        let new_state = GitState::Modified {
            files: vec![path.clone()],
            has_staged: false,
            last_change: Utc::now(),
        };

        // Update state
        self.file_states.insert(path.clone(), new_state.clone());

        // Update navigation map
        {
            let mut map = self.navigation_map.lock().unwrap();
            map.update_git_state(&path, new_state);
        } // Drop the lock here

        // Record transition
        self.record_transition(
            workspace_id,
            Some(path),
            &self.state_name(&current_state),
            "modified",
            json!({"event": "file_modified"}),
        );

        Ok(())
    }

    /// Handle files being staged
    fn handle_files_staged(&mut self, files: Vec<PathBuf>, workspace_id: String) -> Result<()> {
        let new_state = GitState::Staged {
            files: files.clone(),
            staged_at: Utc::now(),
        };

        // Collect transitions to record after releasing lock
        let mut transitions = Vec::new();

        {
            let mut map = self.navigation_map.lock().unwrap();

            for file in &files {
                // Update state
                let current = self.file_states.get(file).cloned();
                self.file_states.insert(file.clone(), new_state.clone());

                // Update navigation map
                map.update_git_state(file, new_state.clone());

                // Prepare transition
                transitions.push((file.clone(), self.state_name(&current)));
            }
        } // Drop the lock here

        // Record transitions after lock is released
        for (file, from_state) in transitions {
            self.record_transition(
                workspace_id.clone(),
                Some(file),
                &from_state,
                "staged",
                json!({"event": "file_staged"}),
            );
        }

        Ok(())
    }

    /// Handle commit
    fn handle_commit(
        &mut self,
        sha: String,
        message: String,
        files: Vec<PathBuf>,
        workspace_id: String,
    ) -> Result<()> {
        let new_state = GitState::Committed {
            sha: sha.clone(),
            message: message.clone(),
            timestamp: Utc::now(),
            files: files.clone(),
        };

        // Extract patterns from commit message
        let patterns = self.extract_patterns_from_message(&message);

        // Collect transitions to record after releasing lock
        let mut transitions = Vec::new();

        {
            let mut map = self.navigation_map.lock().unwrap();

            for file in &files {
                // Update state
                let current = self.file_states.get(file).cloned();
                self.file_states.insert(file.clone(), new_state.clone());

                // Update navigation map with higher confidence
                map.update_git_state(file, new_state.clone());

                // Prepare transition
                transitions.push((file.clone(), self.state_name(&current)));
            }
        } // Drop the lock here

        // Record transitions after lock is released
        for (file, from_state) in transitions {
            self.record_transition(
                workspace_id.clone(),
                Some(file),
                &from_state,
                "committed",
                json!({
                    "sha": &sha,
                    "patterns": &patterns,
                }),
            );
        }

        Ok(())
    }

    /// Handle push to remote
    fn handle_push(&mut self, remote: String, branch: String, workspace_id: String) -> Result<()> {
        // Get files in the current commit
        let files: Vec<PathBuf> = self
            .file_states
            .iter()
            .filter_map(|(path, state)| match state {
                GitState::Committed { .. } => Some(path.clone()),
                _ => None,
            })
            .collect();

        let new_state = GitState::Pushed {
            remote: remote.clone(),
            branch: branch.clone(),
            sha: self.get_current_sha(&workspace_id)?,
        };

        {
            let mut map = self.navigation_map.lock().unwrap();

            for file in &files {
                self.file_states.insert(file.clone(), new_state.clone());
                map.update_git_state(file, new_state.clone());
            }
        } // Drop the lock here

        // Record transitions after lock is released
        for file in &files {
            self.record_transition(
                workspace_id.clone(),
                Some(file.clone()),
                "committed",
                "pushed",
                json!({
                    "remote": &remote,
                    "branch": &branch,
                }),
            );
        }

        Ok(())
    }

    /// Handle pull request opening
    fn handle_pr_opened(&mut self, number: u32, url: String, workspace_id: String) -> Result<()> {
        let new_state = GitState::PullRequest {
            number,
            url: url.clone(),
            base_branch: "main".to_string(), // TODO: Get from event
            state: PRState::Open,
        };

        {
            let mut map = self.navigation_map.lock().unwrap();

            // Update all files in workspace
            for file in self.file_states.keys() {
                map.update_git_state(file, new_state.clone());
            }
        } // Drop the lock here

        self.record_transition(
            workspace_id,
            None,
            "pushed",
            "pull_request",
            json!({
                "pr_number": number,
                "pr_url": url,
            }),
        );

        Ok(())
    }

    /// Handle merge
    fn handle_merge(&mut self, into_branch: String, workspace_id: String) -> Result<()> {
        let new_state = GitState::Merged {
            into_branch: into_branch.clone(),
            merge_sha: self.get_current_sha(&workspace_id)?,
            timestamp: Utc::now(),
        };

        {
            let mut map = self.navigation_map.lock().unwrap();

            // Update all files and boost confidence
            for file in self.file_states.keys() {
                map.update_git_state(file, new_state.clone());
            }

            // If merged to main, update workspace confidence
            if into_branch == "main" || into_branch == "master" {
                // TODO: Update workspace confidence when we have mutable access to workspace states
            }
        } // Drop the lock here

        self.record_transition(
            workspace_id,
            None,
            "pull_request",
            "merged",
            json!({
                "into_branch": into_branch,
            }),
        );

        Ok(())
    }

    /// Record a state transition
    fn record_transition(
        &mut self,
        workspace_id: String,
        file_path: Option<PathBuf>,
        from_state: &str,
        to_state: &str,
        metadata: serde_json::Value,
    ) {
        self.state_transitions.push(StateTransition {
            workspace_id,
            file_path,
            from_state: from_state.to_string(),
            to_state: to_state.to_string(),
            timestamp: Utc::now(),
            metadata,
        });
    }

    /// Get state name for display
    fn state_name(&self, state: &Option<GitState>) -> String {
        match state {
            Some(GitState::Untracked { .. }) => "untracked",
            Some(GitState::Modified { .. }) => "modified",
            Some(GitState::Staged { .. }) => "staged",
            Some(GitState::Committed { .. }) => "committed",
            Some(GitState::Pushed { .. }) => "pushed",
            Some(GitState::PullRequest { .. }) => "pull_request",
            Some(GitState::Merged { .. }) => "merged",
            Some(GitState::Archived { .. }) => "archived",
            None => "none",
        }
        .to_string()
    }

    /// Extract pattern references from commit message
    fn extract_patterns_from_message(&self, message: &str) -> Vec<String> {
        // Simple pattern extraction - look for "pattern:", "add:", "implement:", etc.
        let mut patterns = Vec::new();

        for line in message.lines() {
            let lower = line.to_lowercase();
            if lower.contains("pattern:") || lower.contains("implement:") || lower.contains("add:")
            {
                if let Some(pattern) = line.split(':').nth(1) {
                    patterns.push(pattern.trim().to_string());
                }
            }
        }

        patterns
    }

    /// Get current SHA for workspace
    fn get_current_sha(&self, workspace_id: &str) -> Result<String> {
        if let Some(client) = &self.workspace_client {
            let status = client.get_git_status(workspace_id)?;
            Ok(status.current_commit)
        } else {
            Ok("unknown".to_string())
        }
    }

    /// Get transition history
    pub fn get_transitions(&self) -> &[StateTransition] {
        &self.state_transitions
    }

    /// Get current file states
    pub fn get_file_states(&self) -> &HashMap<PathBuf, GitState> {
        &self.file_states
    }

    /// Get the git state for a specific path
    pub fn get_git_state(&self, path: &PathBuf) -> Option<&GitState> {
        self.file_states.get(path)
    }

    /// Track a document in the state machine with actual git state detection
    pub fn track_document(&mut self, path: &PathBuf) {
        // If not already tracked, detect actual git state
        if !self.file_states.contains_key(path) {
            // Try to find git repo root
            let repo_root = self.find_git_root(path);

            let git_state = if let Some(repo) = repo_root {
                // Detect actual git state
                match super::git_detection::detect_file_state(&repo, path) {
                    Ok(state) => state,
                    Err(_) => GitState::Untracked {
                        detected_at: Utc::now(),
                        files: vec![path.clone()],
                    },
                }
            } else {
                // Not in a git repo
                GitState::Untracked {
                    detected_at: Utc::now(),
                    files: vec![path.clone()],
                }
            };

            self.file_states.insert(path.clone(), git_state);
        }
    }

    /// Find the git repository root for a path
    fn find_git_root(&self, path: &Path) -> Option<PathBuf> {
        let mut current = if path.is_file() { path.parent()? } else { path };

        loop {
            if current.join(".git").exists() {
                return Some(current.to_path_buf());
            }

            current = current.parent()?;
        }
    }

    /// Process a git event (alias for process_event)
    pub fn process_git_event(&mut self, event: GitEvent) -> Result<()> {
        self.process_event(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine_creation() {
        let machine = GitNavigationStateMachine::new().unwrap();
        assert_eq!(machine.get_transitions().len(), 0);
        assert_eq!(machine.get_file_states().len(), 0);
    }

    #[test]
    fn test_file_created_event() {
        let mut machine = GitNavigationStateMachine::new().unwrap();

        let event = GitEvent::FileCreated {
            path: PathBuf::from("test.md"),
            workspace_id: "ws-123".to_string(),
        };

        machine.process_event(event).unwrap();

        assert_eq!(machine.get_transitions().len(), 1);
        assert_eq!(machine.get_file_states().len(), 1);

        let transition = &machine.get_transitions()[0];
        assert_eq!(transition.from_state, "none");
        assert_eq!(transition.to_state, "untracked");
    }
}
