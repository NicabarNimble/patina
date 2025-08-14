---
id: workspace-monitoring
status: experimental
created: 2025-08-11
references: [oxidized-knowledge, dependable-rust]
tags: [monitoring, workspace, git, synchronous, pattern]
---

# Workspace Monitoring Pattern

**Purpose:** Monitor workspace directories for file changes and git status updates to trigger pattern extraction and layer updates.

---

## Alignment with Core

Per `oxidized-knowledge.md`:
- Monitors the **Surface** layer for active oxidation
- Detects **Git** events (Time) that create patterns  
- Triggers **Pattern Extraction** from the data flow
- Maintains **Isolation** between workspaces

## Synchronous Design

No async runtime - uses threads and channels per `dependable-rust.md`:

```rust
// In internal/monitor.rs (not exposed in mod.rs)
pub(crate) struct WorkspaceMonitor {
    workspace_id: String,
    worktree_path: PathBuf,
    event_tx: mpsc::Sender<MonitorEvent>,
}
```

## File System Watching

```rust
// Internal implementation detail
fn start_file_watcher(&self) -> Result<thread::JoinHandle<()>> {
    let path = self.worktree_path.clone();
    let tx = self.event_tx.clone();
    
    thread::spawn(move || {
        let (notify_tx, notify_rx) = mpsc::channel();
        let mut watcher = notify::RecommendedWatcher::new(
            move |res| { 
                if let Ok(event) = res {
                    let _ = notify_tx.send(event);
                }
            },
            Default::default(),
        ).expect("Failed to create watcher");
        
        watcher.watch(&path, RecursiveMode::Recursive)
            .expect("Failed to watch path");
        
        for event in notify_rx {
            if should_process(&event) {
                let _ = tx.send(process_event(event));
            }
        }
    })
}
```

## Git Status Polling  

```rust
fn poll_git_status(&self) -> thread::JoinHandle<()> {
    let path = self.worktree_path.clone();
    let tx = self.event_tx.clone();
    
    thread::spawn(move || {
        let mut last_hash = None;
        loop {
            thread::sleep(Duration::from_secs(5));
            
            if let Ok(status) = get_git_status(&path) {
                let hash = calculate_hash(&status);
                if last_hash != Some(hash) {
                    let _ = tx.send(MonitorEvent::GitChanged(status));
                    last_hash = Some(hash);
                }
            }
        }
    })
}
```

## Integration Points

Per `oxidized-knowledge.md` data flow:

```rust
// Git Commit → Pattern Extraction
MonitorEvent::GitCommit { sha, message, diff } => {
    // Extract patterns from commit
    let patterns = extract_patterns_from_diff(&diff);
    
    // Add to Surface layer
    for pattern in patterns {
        indexer.add_to_surface(pattern)?;
    }
}

// Session Capture → Context Log
MonitorEvent::SessionUpdate { session_id, content } => {
    // Link session to workspace
    indexer.link_session_workspace(session_id, workspace_id)?;
}
```

## Path Filtering

Only monitor relevant paths:

```rust
fn should_process(path: &Path) -> bool {
    // Only pattern files
    if !path.extension().map_or(false, |e| e == "md") {
        return false;
    }
    
    // Only in layer directories
    let path_str = path.to_string_lossy();
    path_str.contains("/layer/") && 
        (path_str.contains("/surface/") || 
         path_str.contains("/core/") || 
         path_str.contains("/dust/"))
}
```

## Pattern Lifecycle Triggers

```rust
pub enum MonitorEvent {
    // Surface → Core promotion check
    PatternUsedSuccessfully { pattern_id: String, context: String },
    
    // Surface → Dust demotion
    PatternFailed { pattern_id: String, error: String },
    
    // Git events for pattern extraction
    GitCommit { sha: String, message: String, diff: String },
    
    // Session events for context
    SessionUpdate { session_id: String, workspace_id: String },
}
```

## Future Connection Points

When implemented, this will connect:
- **Sessions** → Workspace monitoring for real-time context
- **Git worktrees** → Per-workspace isolation
- **Pattern promotion** → Track usage across workspaces
- **CRDT sync** → Distribute pattern updates

## Status: Experimental

This pattern is not yet implemented. It describes how workspace monitoring would work within Patina's architecture when needed.