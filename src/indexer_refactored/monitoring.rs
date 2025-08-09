//! Workspace monitoring for real-time pattern discovery

use super::{GitEvent, GitNavigationStateMachine};
use anyhow::Result;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

/// Monitors a workspace for file and git changes
pub struct WorkspaceMonitor {
    workspace_id: String,
    worktree_path: PathBuf,
    event_tx: mpsc::Sender<MonitorEvent>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

/// Events detected by the monitor
#[derive(Debug, Clone)]
pub enum MonitorEvent {
    FileChanged {
        path: PathBuf,
        kind: FileChangeKind,
    },
    GitStatusChanged {
        workspace_id: String,
        status: GitStatusSnapshot,
    },
    Error(String),
}

/// Types of file changes
#[derive(Debug, Clone, Copy)]
pub enum FileChangeKind {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// Snapshot of git status
#[derive(Debug, Clone)]
pub struct GitStatusSnapshot {
    pub modified_files: Vec<PathBuf>,
    pub staged_files: Vec<PathBuf>,
    pub untracked_files: Vec<PathBuf>,
    pub current_branch: String,
    pub has_changes: bool,
}

impl WorkspaceMonitor {
    /// Create a new workspace monitor
    pub fn new(
        workspace_id: String,
        worktree_path: PathBuf,
    ) -> (Self, mpsc::Receiver<MonitorEvent>) {
        let (event_tx, event_rx) = mpsc::channel(100);

        (
            Self {
                workspace_id,
                worktree_path,
                event_tx,
                shutdown_tx: None,
            },
            event_rx,
        )
    }

    /// Start monitoring the workspace
    pub async fn start(mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Start file watcher
        self.start_file_watcher()?;

        // Start git status monitor
        let _git_monitor_handle = self.start_git_monitor();

        // Wait for shutdown signal
        shutdown_rx.recv().await;

        Ok(())
    }

    /// Start file system watcher
    fn start_file_watcher(&self) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);
        let event_tx = self.event_tx.clone();
        let _workspace_id = self.workspace_id.clone();

        // Create watcher
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                }
            },
            Config::default(),
        )?;

        // Watch the worktree path
        watcher.watch(&self.worktree_path, RecursiveMode::Recursive)?;

        // Process file events
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Some(monitor_event) = Self::process_file_event(event) {
                    let _ = event_tx.send(monitor_event).await;
                }
            }
        });

        // Keep watcher alive
        std::mem::forget(watcher);

        Ok(())
    }

    /// Start git status monitor
    fn start_git_monitor(&self) -> tokio::task::JoinHandle<()> {
        let event_tx = self.event_tx.clone();
        let workspace_id = self.workspace_id.clone();
        let worktree_path = self.worktree_path.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));
            let mut last_status = None;

            loop {
                interval.tick().await;

                // Get current git status
                if let Ok(status) = Self::get_git_status(&worktree_path).await {
                    // Check if status changed
                    let changed = match &last_status {
                        None => true,
                        Some(last) => !Self::status_equal(last, &status),
                    };

                    if changed && status.has_changes {
                        let _ = event_tx
                            .send(MonitorEvent::GitStatusChanged {
                                workspace_id: workspace_id.clone(),
                                status: status.clone(),
                            })
                            .await;

                        last_status = Some(status);
                    }
                }
            }
        })
    }

    /// Process file system event
    fn process_file_event(event: Event) -> Option<MonitorEvent> {
        // Skip git directory and other non-relevant files
        let path = event.paths.first()?;
        if Self::should_ignore_path(path) {
            return None;
        }

        let kind = match event.kind {
            EventKind::Create(_) => FileChangeKind::Created,
            EventKind::Modify(_) => FileChangeKind::Modified,
            EventKind::Remove(_) => FileChangeKind::Deleted,
            _ => return None,
        };

        Some(MonitorEvent::FileChanged {
            path: path.clone(),
            kind,
        })
    }

    /// Check if path should be ignored
    fn should_ignore_path(path: &Path) -> bool {
        // Ignore git internals
        if path.components().any(|c| c.as_os_str() == ".git") {
            return true;
        }

        // Ignore common build/temp files
        if let Some(name) = path.file_name() {
            let name = name.to_string_lossy();
            if name.starts_with('.') || name.ends_with('~') || name == "target" {
                return true;
            }
        }

        false
    }

    /// Get git status snapshot
    async fn get_git_status(_worktree_path: &Path) -> Result<GitStatusSnapshot> {
        // TODO: Implement actual git status check
        // For now, return a mock status
        Ok(GitStatusSnapshot {
            modified_files: vec![],
            staged_files: vec![],
            untracked_files: vec![],
            current_branch: "main".to_string(),
            has_changes: false,
        })
    }

    /// Compare two git status snapshots
    fn status_equal(a: &GitStatusSnapshot, b: &GitStatusSnapshot) -> bool {
        a.modified_files == b.modified_files
            && a.staged_files == b.staged_files
            && a.untracked_files == b.untracked_files
            && a.current_branch == b.current_branch
    }

    /// Stop monitoring
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
    }
}

/// Connect monitor events to state machine
pub async fn connect_monitor_to_state_machine(
    mut event_rx: mpsc::Receiver<MonitorEvent>,
    state_machine: Arc<tokio::sync::RwLock<GitNavigationStateMachine>>,
    workspace_id: String,
) {
    while let Some(event) = event_rx.recv().await {
        match event {
            MonitorEvent::FileChanged { path, kind } => {
                let git_event = match kind {
                    FileChangeKind::Created => GitEvent::FileCreated {
                        path,
                        workspace_id: workspace_id.clone(),
                    },
                    FileChangeKind::Modified => GitEvent::FileModified {
                        path,
                        workspace_id: workspace_id.clone(),
                    },
                    _ => continue,
                };

                let mut machine = state_machine.write().await;
                if let Err(e) = machine.process_event(git_event).await {
                    eprintln!("Failed to process git event: {e}");
                }
            }

            MonitorEvent::GitStatusChanged {
                workspace_id: _ws_id,
                status: _,
            } => {
                // Process git status changes
                // TODO: Convert status snapshot to appropriate git events
            }

            MonitorEvent::Error(e) => {
                eprintln!("Monitor error: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_ignore_path() {
        assert!(WorkspaceMonitor::should_ignore_path(Path::new(
            ".git/config"
        )));
        assert!(WorkspaceMonitor::should_ignore_path(Path::new("target")));
        assert!(WorkspaceMonitor::should_ignore_path(Path::new(".DS_Store")));
        assert!(!WorkspaceMonitor::should_ignore_path(Path::new(
            "src/main.rs"
        )));
        assert!(!WorkspaceMonitor::should_ignore_path(Path::new(
            "README.md"
        )));
    }
}
