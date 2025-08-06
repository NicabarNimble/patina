---
id: git-aware-navigation-design
version: 2
status: active
created_date: 2025-08-03
updated_date: 2025-08-04
oxidizer: nicabar
references: [surface/indexer-design.md, core/layer-architecture.md, core/escape-hatch-philosophy.md, surface/git-aware-navigation-design-sqlite.md, external/no-boilerplate-async-rust]
tags: [architecture, navigation, git-integration, sqlite, crdt, synchronous, local-first, automerge]
supersedes: [surface/git-aware-navigation-design-sqlite.md]
---

# Git-Aware Navigation Design (SQLite + Automerge CRDT)

A fundamental redesign of Patina's navigation system using synchronous, local-first SQLite for storage with Automerge CRDT layer for distributed features, eliminating async complexity while maintaining flexibility.

> **Note**: This document supersedes `git-aware-navigation-design-sqlite.md` and represents the complete, merged design including all implementation learnings and architectural decisions.

## Executive Summary

This design extends Patina's indexer (from `indexer-design.md`) with git state awareness, allowing navigation queries to consider the maturity and lifecycle of patterns. By tracking git states (untracked → staged → committed → merged), we can provide confidence scores that reflect real-world pattern adoption.

**Updated Implementation (v2)**: Now uses synchronous SQLite + optional Automerge CRDT instead of async rqlite, eliminating complexity while preserving all core functionality.

## Core Concepts

### 1. Git States as Confidence Signals

Git operations naturally map to pattern confidence levels:

```
UNTRACKED → STAGED → COMMITTED → PUSHED → MERGED → ARCHIVED
    ↓         ↓         ↓          ↓        ↓         ↓
(experimental) (low)  (medium)   (high)  (verified) (historical)
```

### 2. Workspace-Driven Discovery

Following the container-use pattern, each pattern exploration happens in an isolated git worktree:
- Workspace = Git worktree + Dagger container
- Changes tracked in real-time
- Pattern evolution visible through git history

### 3. Navigation Confidence Scoring

Navigation results are ranked by combining:
- Layer position (Core > Surface > Dust)
- Git state (Merged > Committed > Modified)
- Workspace activity (Active exploration boosts visibility)

## Implementation Approach (v2)

### The Async Trap

Our original design used async because rqlite required network I/O. But this infected our entire codebase with unnecessary complexity:
- `tokio::Runtime` for simple file reads
- `'static` lifetime requirements breaking borrowing
- Complex error handling for simple operations
- Runtime state instead of compile-time guarantees

### The Realization

Patina's workload is inherently synchronous:
- **Local file I/O** - Reading markdown files
- **SQLite queries** - Microsecond operations
- **Git commands** - Subprocess calls
- **Pattern indexing** - CPU-bound work

No network I/O in the hot path = no need for async!

### The New Architecture

Since Patina's workload is inherently synchronous, we've redesigned to use:
- **SQLite** for local storage (microsecond queries)
- **Automerge** for optional CRDT capabilities
- **Rayon** for parallel processing
- **No async runtime** - simpler, more idiomatic Rust

```
UNTRACKED → STAGED → COMMITTED → PUSHED → MERGED → ARCHIVED
    ↓         ↓         ↓          ↓        ↓         ↓
(experimental) (low)  (medium)   (high)  (verified) (historical)
```

### 2. Workspace-Driven Discovery

Following the container-use pattern, each pattern exploration happens in an isolated git worktree:
- Workspace = Git worktree + Dagger container
- Changes tracked in real-time
- Pattern evolution visible through git history

### 3. Navigation Confidence Scoring

Navigation results are ranked by combining:
- Layer position (Core > Surface > Dust)
- Git state (Merged > Committed > Modified)
- Workspace activity (Active exploration boosts visibility)

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                   Navigation Query                       │
└─────────────────────┬───────────────────────────────────┘
                      ▼
┌─────────────────────────────────────────────────────────┐
│          PatternIndexer (Pure Synchronous Rust)         │
│  ┌─────────────────────────────────────────────────┐    │
│  │      NavigationMap (In-Memory Cache)            │    │
│  │  - Concept mappings (HashMap)                   │    │
│  │  - Document metadata (No async locks!)          │    │
│  │  - Git state cache (Simple Mutex)              │    │
│  │  - Workspace states (Direct access)            │    │
│  └─────────────────────┬───────────────────────────┘    │
│                        │                                 │
│  ┌─────────────────────┴───────────────────────────┐    │
│  │    GitNavigationStateMachine (Sync)             │    │
│  │  - Direct git CLI calls (default)               │    │
│  │  - Optional workspace client integration        │    │
│  │  - Synchronous state updates                    │    │
│  └─────────────────────┬───────────────────────────┘    │
└────────────────────────┼─────────────────────────────────┘
                         ▼
┌─────────────────────────────────────────────────────────┐
│        SQLite Database (.patina/navigation.db)          │
│  - Local SQLite for persistent storage                  │
│  - Microsecond queries (no network!)                    │
│  - Standard SQL tables for all data                     │
│  - Works independently of CRDT layer                    │
└─────────────────────┬───────────────────────────────────┘
                      │
┌─────────────────────┴───────────────────────────────────┐
│        Automerge CRDT Layer (Optional)                  │
│  - In-memory CRDT documents                            │
│  - Syncs selected data (patterns, workspace states)     │
│  - Persists changes back to SQLite                      │
│  - Can be disabled or replaced                          │
└─────────────────────────────────────────────────────────┘
                      
              ┌── Optional Integration ──┐
              ▼                          ▼
┌─────────────────────────────────────────────────────────┐
│      Workspace Service (Go + Dagger) - OPTIONAL         │
│  - Git worktree management                              │
│  - Container isolation for development                  │
│  - Advanced file change detection                       │
│  - HTTP API for workspace operations                    │
└─────────────────────────────────────────────────────────┘
```

## Detailed Design

### 1. Enhanced Navigation State

```rust
// src/indexer/navigation_state.rs
pub struct GitAwareNavigationMap {
    // Original NavigationMap fields
    concepts: HashMap<String, Vec<Location>>,
    documents: HashMap<String, DocumentInfo>,
    relationships: Graph<String, RelationType>,
    
    // Git state tracking
    git_states: HashMap<PathBuf, GitState>,
    workspace_states: HashMap<String, WorkspaceNavigationState>,
    
    // Performance tracking
    last_refresh: Instant,
    cache_generation: u64,
}

pub struct WorkspaceNavigationState {
    pub workspace_id: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub git_state: GitState,
    pub navigation_confidence: Confidence,
    pub discovered_patterns: Vec<DiscoveredPattern>,
    pub last_activity: Instant,
}

pub enum GitState {
    // Development states
    Untracked { 
        detected_at: Instant,
        files: Vec<PathBuf>,
    },
    Modified { 
        files: Vec<PathBuf>,
        has_staged: bool,
        last_change: Instant,
    },
    Staged {
        files: Vec<PathBuf>,
        staged_at: Instant,
    },
    
    // Integration states  
    Committed {
        sha: String,
        message: String,
        timestamp: DateTime<Utc>,
        files: Vec<PathBuf>,
    },
    Pushed {
        remote: String,
        branch: String,
        sha: String,
    },
    PullRequest {
        number: u32,
        url: String,
        base_branch: String,
        state: PRState, // open, closed, merged
    },
    Merged {
        into_branch: String,
        merge_sha: String,
        timestamp: DateTime<Utc>,
    },
    
    // Archive states
    Archived {
        reason: ArchiveReason,
        moved_to: Layer,
        archived_at: DateTime<Utc>,
    },
}

#[derive(Clone, Copy)]
pub enum Confidence {
    Experimental = 1,  // Untracked files
    Low = 2,          // Modified/staged  
    Medium = 3,       // Committed locally
    High = 4,         // Pushed/PR
    Verified = 5,     // Merged to main
    Historical = 0,   // Archived
}
```

### 2. State Machine Implementation (Synchronous)

```rust
// src/indexer/state_machine.rs
use std::sync::{Arc, Mutex};

pub struct GitNavigationStateMachine {
    navigation_map: Arc<Mutex<GitAwareNavigationMap>>,
    workspace_client: Option<WorkspaceClient>, // Optional integration
    state_transitions: Vec<StateTransition>,
    file_states: HashMap<PathBuf, GitState>,
}

impl GitNavigationStateMachine {
    pub fn new() -> Result<Self> {
        Ok(Self {
            navigation_map: Arc::new(Mutex::new(GitAwareNavigationMap::new())),
            workspace_client: None,
            state_transitions: vec![],
            file_states: HashMap::new(),
        })
    }
    
    /// Set workspace client for enhanced git operations (optional)
    pub fn set_workspace_client(&mut self, client: WorkspaceClient) {
        self.workspace_client = Some(client);
    }
    
    /// Process git event and update navigation state - no async!
    pub fn process_git_event(&mut self, event: GitEvent) -> Result<()> {
        match event {
            GitEvent::FileCreated { path, workspace_id } => {
                self.handle_new_file(path, workspace_id)?;
            },
            GitEvent::FileModified { path, workspace_id } => {
                self.handle_file_modified(path, workspace_id)?;
            },
            GitEvent::Commit { sha, message, files, workspace_id } => {
                self.handle_commit(sha, message, files, workspace_id)?;
            },
            GitEvent::Push { remote, branch, workspace_id } => {
                self.handle_push(remote, branch, workspace_id)?;
            },
            GitEvent::PROpened { number, url, workspace_id } => {
                self.handle_pr_opened(number, url, workspace_id)?;
            },
            GitEvent::Merged { into_branch, workspace_id } => {
                self.handle_merge(into_branch, workspace_id)?;
            },
        }
        Ok(())
    }
    
    fn handle_commit(
        &mut self, 
        sha: String, 
        message: String, 
        files: Vec<PathBuf>,
        workspace_id: String
    ) -> Result<()> {
        // Update git state
        let new_state = GitState::Committed {
            sha: sha.clone(),
            message: message.clone(),
            timestamp: Utc::now(),
            files: files.clone(),
        };
        
        // Update navigation map directly - no await!
        let mut nav_map = self.navigation_map.lock().unwrap();
        
        // Extract patterns from commit message
        let patterns = self.extract_patterns_from_commit(&message)?;
        
        // Update confidence for committed files
        for file in &files {
            nav_map.update_document_confidence(file, Confidence::Medium);
            
            // If it's a pattern file, mark for indexing
            if Self::is_pattern_file(file) {
                // Indexing happens separately, not inline
                nav_map.mark_for_reindex(file);
            }
        }
        
        // Record state transition
        self.state_transitions.push(StateTransition {
            workspace_id,
            from_state: "modified".to_string(),
            to_state: "committed".to_string(),
            timestamp: Utc::now(),
            metadata: json!({
                "sha": sha,
                "patterns": patterns,
            }),
        });
        
        Ok(())
    }
}
```

### 3. Workspace Integration

```go
// workspace/pkg/workspace/navigation_integration.go
package workspace

import (
    "context"
    "encoding/json"
    "time"
)

// NavigationIntegration handles pattern discovery in workspaces
type NavigationIntegration struct {
    manager     *Manager
    indexerURL  string
    gitMonitor  *GitMonitor
}

// MonitorWorkspace starts monitoring a workspace for navigation-relevant changes
func (ni *NavigationIntegration) MonitorWorkspace(ctx context.Context, workspaceID string) error {
    ws, err := ni.manager.Get(workspaceID)
    if err != nil {
        return err
    }
    
    // Set up git hooks
    if err := ni.installGitHooks(ws.WorktreePath); err != nil {
        return err
    }
    
    // Start file watcher
    watcher := &FileWatcher{
        Path: ws.WorktreePath,
        OnChange: func(event FileEvent) {
            ni.notifyIndexer(GitEvent{
                Type:        "file_" + event.Type,
                Path:        event.Path,
                WorkspaceID: workspaceID,
            })
        },
    }
    
    go watcher.Start(ctx)
    
    return nil
}

// Git hook notifications
func (ni *NavigationIntegration) HandleGitHook(ctx context.Context, hook GitHookEvent) error {
    switch hook.Type {
    case "post-commit":
        return ni.handlePostCommit(ctx, hook)
    case "post-merge":
        return ni.handlePostMerge(ctx, hook)
    case "post-checkout":
        return ni.handlePostCheckout(ctx, hook)
    }
    return nil
}

func (ni *NavigationIntegration) handlePostCommit(ctx context.Context, hook GitHookEvent) error {
    // Get commit details
    commit, err := ni.gitMonitor.GetCommitInfo(hook.WorktreePath, "HEAD")
    if err != nil {
        return err
    }
    
    // Notify indexer
    return ni.notifyIndexer(GitEvent{
        Type:        "commit",
        WorkspaceID: hook.WorkspaceID,
        Data: map[string]interface{}{
            "sha":     commit.SHA,
            "message": commit.Message,
            "files":   commit.Files,
        },
    })
}
```

### 4. Enhanced Database Schema (SQLite)

```sql
-- SQLite schema for navigation system

-- Documents table
CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    layer TEXT NOT NULL,
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    last_indexed TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Concepts table for navigation
CREATE TABLE IF NOT EXISTS concepts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id TEXT NOT NULL,
    concept TEXT NOT NULL,
    relevance REAL DEFAULT 1.0,
    FOREIGN KEY (document_id) REFERENCES documents(id)
);

-- Git state tracking
CREATE TABLE IF NOT EXISTS git_states (
    id INTEGER PRIMARY KEY,
    document_id TEXT NOT NULL,
    workspace_id TEXT,
    state TEXT NOT NULL,
    confidence_modifier REAL DEFAULT 1.0,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (document_id) REFERENCES documents(id)
);

-- State transition history
CREATE TABLE IF NOT EXISTS state_transitions (
    id INTEGER PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    document_id TEXT,
    from_state TEXT,
    to_state TEXT NOT NULL,
    transition_reason TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    occurred_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Patterns table (synced via CRDT if enabled)
CREATE TABLE IF NOT EXISTS patterns (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    layer TEXT NOT NULL,
    confidence TEXT,
    discovered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_modified TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Workspace states (synced via CRDT if enabled)
CREATE TABLE IF NOT EXISTS workspace_states (
    workspace_id TEXT PRIMARY KEY,
    navigation_state TEXT NOT NULL,
    last_query TEXT,
    active_patterns TEXT,
    last_modified TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for performance
CREATE INDEX idx_git_states_workspace ON git_states(workspace_id);
CREATE INDEX idx_git_states_document ON git_states(document_id);
CREATE INDEX idx_transitions_workspace ON state_transitions(workspace_id);
CREATE INDEX idx_concepts_document ON concepts(document_id);
CREATE INDEX idx_documents_layer ON documents(layer);
```

### 5. Navigation Query Enhancement (Synchronous)

```rust
// src/indexer/mod.rs
impl PatternIndexer {
    /// Navigate with git context - synchronous!
    pub fn navigate(&self, query: &str) -> NavigationResponse {
        // 1. Base navigation from memory cache
        let cache = self.cache.lock().unwrap();
        let mut response = cache.navigate(query);
        
        // 2. Enrich with git state using state machine
        let state_machine = self.state_machine.lock().unwrap();
        for location in &mut response.locations {
            if let Some(git_state) = state_machine.get_git_state(&location.path) {
                location.git_state = Some(git_state.clone());
                location.confidence = self.calculate_git_confidence(
                    location.confidence,
                    git_state
                );
            }
        }
        
        response
    }
    
    /// Index documents in parallel using Rayon
    pub fn index_directory(&self, dir: &Path) -> Result<()> {
        let markdown_files: Vec<_> = walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension() == Some("md".as_ref()))
            .collect();
        
        // Parallel indexing with Rayon!
        markdown_files
            .par_iter()
            .try_for_each(|entry| self.index_document(entry.path()))?;
        
        Ok(())
    }
    
    fn calculate_git_confidence(&self, base: Confidence, state: &GitState) -> Confidence {
        match state {
            GitState::Merged { into_branch, .. } => {
                if into_branch == "main" {
                    Confidence::Verified
                } else {
                    Confidence::High
                }
            },
            GitState::PullRequest { state: PRState::Open, .. } => Confidence::High,
            GitState::Committed { .. } => Confidence::Medium,
            GitState::Staged { .. } => Confidence::Low,
            GitState::Modified { .. } => Confidence::Low,
            GitState::Untracked { .. } => Confidence::Experimental,
            GitState::Archived { .. } => Confidence::Historical,
            _ => base,
        }
    }
}
```

### 6. Pattern Lifecycle Management (Optional Workspace Integration)

```rust
// src/indexer/pattern_lifecycle.rs
pub struct PatternLifecycle {
    indexer: Arc<PatternIndexer>,
    workspace_client: Option<WorkspaceClient>, // Optional!
    db: Arc<SqliteClient>,
}

impl PatternLifecycle {
    /// Create exploration workspace for a pattern (if workspace service available)
    pub fn start_exploration(&mut self, pattern_query: &str) -> Result<ExplorationSession> {
        if let Some(client) = &self.workspace_client {
            // 1. Create workspace via HTTP API
            let request = CreateWorkspaceRequest {
                name: format!("explore/{}", slug(pattern_query)),
                base_image: None,
                env: None,
            };
            let ws = client.create_workspace(request)?;
            
            // 2. Initialize in database
            self.db.execute(
                "INSERT INTO workspace_states (workspace_id, navigation_state, last_query) 
                 VALUES (?1, ?2, ?3)",
                params![&ws.id, "exploring", &pattern_query]
            )?;
            
            Ok(ExplorationSession {
                workspace_id: ws.id,
                branch: ws.branch_name,
                query: pattern_query.to_string(),
            })
        } else {
            // Fallback: Use regular git branches
            let branch_name = format!("explore/{}", slug(pattern_query));
            std::process::Command::new("git")
                .args(&["checkout", "-b", &branch_name])
                .output()?;
                
            Ok(ExplorationSession {
                workspace_id: branch_name.clone(),
                branch: branch_name,
                query: pattern_query.to_string(),
            })
        }
    }
    
    /// Promote patterns from workspace to higher layer
    pub fn promote_patterns(&mut self, workspace_id: &str, to_layer: Layer) -> Result<Vec<String>> {
        let mut promoted = vec![];
        
        // Get patterns from database
        let patterns = self.db.query_patterns_by_workspace(workspace_id)?;
        
        for pattern in patterns {
            // Move file to new layer
            let new_path = self.move_pattern_to_layer(&pattern.path, &to_layer)?;
            
            // Update database
            self.db.execute(
                "INSERT INTO pattern_promotions (pattern_id, from_layer, to_layer, from_workspace) 
                 VALUES (?1, ?2, ?3, ?4)",
                params![&pattern.id, &pattern.layer, &to_layer.to_string(), workspace_id]
            )?;
            
            promoted.push(pattern.id);
        }
        
        // Update workspace state
        self.db.execute(
            "UPDATE workspace_states SET navigation_state = 'promoted' WHERE workspace_id = ?1",
            params![workspace_id]
        )?;
        
        Ok(promoted)
    }
}
```

### 7. Real-time Monitoring (Future Enhancement)

```rust
// src/indexer/monitoring.rs - TO BE IMPLEMENTED
// Currently monitoring happens through:
// 1. Direct git status checks during navigation
// 2. Optional workspace service webhooks

pub struct WorkspaceMonitor {
    workspace_id: String,
    worktree_path: PathBuf,
    event_tx: std::sync::mpsc::Sender<MonitorEvent>,
}

impl WorkspaceMonitor {
    /// Future: Background thread for monitoring
    pub fn start(self) -> Result<()> {
        // Use std::thread instead of tokio
        std::thread::spawn(move || {
            loop {
                // Check git status using shell commands
                if let Ok(output) = std::process::Command::new("git")
                    .args(&["status", "--porcelain"])
                    .current_dir(&self.worktree_path)
                    .output() 
                {
                    // Parse and send events
                    // This is simpler than async and works just as well
                }
                
                std::thread::sleep(Duration::from_secs(5));
            }
        });
        
        Ok(())
    }
}
```

## Implementation Design

### 1. Hybrid Database Module with SQLite + Automerge

```rust
// src/indexer/hybrid_database.rs
use std::sync::{Arc, Mutex};
use rusqlite::{Connection, params};
use automerge::{Automerge, ObjType, transaction::Transactable};
use std::path::Path;

pub struct HybridDatabase {
    /// SQLite for persistent storage
    sqlite: Arc<Mutex<Connection>>,
    /// Automerge for CRDT operations (optional)
    crdt: Option<Arc<Mutex<NavigationCRDT>>>,
}

pub struct NavigationCRDT {
    /// Automerge document for patterns
    patterns_doc: Automerge,
    /// Automerge document for workspace states
    workspace_doc: Automerge,
    /// Site ID for this peer
    site_id: Vec<u8>,
}

impl HybridDatabase {
    /// Create hybrid database with optional CRDT support
    pub fn new(db_path: &Path, enable_crdt: bool) -> Result<Self> {
        // Ensure .patina directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Open SQLite connection
        let conn = Connection::open(db_path)?;
        
        // Configure for optimal performance
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA mmap_size = 30000000000;
        ")?;
        
        let sqlite = Arc::new(Mutex::new(conn));
        
        // Optionally initialize CRDT layer
        let crdt = if enable_crdt {
            Some(Arc::new(Mutex::new(NavigationCRDT::new()?))
        } else {
            None
        };
        
        Ok(Self { sqlite, crdt })
    }
    
    /// Initialize schema (SQLite only)
    pub fn initialize_schema(&self) -> Result<()> {
        let conn = self.sqlite.lock().unwrap();
        
        // Regular tables for all data
        conn.execute(
            "CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                layer TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                last_indexed TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        
        // Patterns table (synced via CRDT if enabled)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS patterns (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                content TEXT NOT NULL,
                layer TEXT NOT NULL,
                confidence TEXT,
                discovered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                last_modified TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        
        // Workspace states (synced via CRDT if enabled)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS workspace_states (
                workspace_id TEXT PRIMARY KEY,
                navigation_state TEXT NOT NULL,
                last_query TEXT,
                active_patterns TEXT,
                last_modified TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        
        Ok(())
    }
    
    /// Add pattern with optional CRDT sync
    pub fn add_pattern(&self, pattern: &Pattern) -> Result<()> {
        // Always store in SQLite
        let conn = self.sqlite.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO patterns 
             (id, name, content, layer, confidence) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![&pattern.id, &pattern.name, &pattern.content, 
                    &pattern.layer, &pattern.confidence],
        )?;
        
        // Update CRDT if enabled
        if let Some(crdt) = &self.crdt {
            let mut crdt_lock = crdt.lock().unwrap();
            crdt_lock.add_pattern(pattern)?;
        }
        
        Ok(())
    }
}
```

### 2. CRDT Implementation with Automerge

```rust
// src/indexer/navigation_crdt.rs
use automerge::{Automerge, ObjType, transaction::Transactable, ROOT};
use uuid::Uuid;

impl NavigationCRDT {
    pub fn new() -> Result<Self> {
        let patterns_doc = Automerge::new();
        let workspace_doc = Automerge::new();
        
        // Generate unique site ID
        let site_id = Uuid::new_v4().as_bytes().to_vec();
        
        Ok(Self {
            patterns_doc,
            workspace_doc,
            site_id,
        })
    }
    
    pub fn add_pattern(&mut self, pattern: &Pattern) -> Result<()> {
        // Store patterns at root level with prefixed keys
        let pattern_key = format!("pattern:{}", pattern.id);
        
        self.patterns_doc.transact(|tx| {
            let pattern_obj = tx.put_object(ROOT, &pattern_key, ObjType::Map)?;
            tx.put(&pattern_obj, "id", &pattern.id)?;
            tx.put(&pattern_obj, "name", &pattern.name)?;
            tx.put(&pattern_obj, "content", &pattern.content)?;
            tx.put(&pattern_obj, "layer", &pattern.layer)?;
            tx.put(&pattern_obj, "confidence", &pattern.confidence)?;
            tx.put(&pattern_obj, "timestamp", chrono::Utc::now().timestamp() as i64)?;
            Ok::<(), automerge::AutomergeError>(())
        })?;
        
        Ok(())
    }
    
    pub fn get_changes_since(&self, version: &[u8]) -> Result<Vec<u8>> {
        // Get changes for both documents
        let pattern_changes = self.patterns_doc.get_changes(version)?;
        let workspace_changes = self.workspace_doc.get_changes(version)?;
        
        // Combine changes (you'd serialize this properly)
        Ok([pattern_changes, workspace_changes].concat())
    }
    
    pub fn apply_changes(&mut self, changes: &[u8]) -> Result<()> {
        // Split and apply changes (inverse of get_changes_since)
        // This is simplified - you'd properly deserialize
        self.patterns_doc.apply_changes(changes)?;
        self.workspace_doc.apply_changes(changes)?;
        Ok(())
    }
}
```

### 3. Simple Navigate Command

```rust
// src/commands/navigate.rs - Look how simple!
pub fn execute(query: &str, layer: Option<String>, json_output: bool) -> Result<()> {
    let project_root = SessionManager::find_project_root()?;
    let layer_path = project_root.join("layer");
    
    // Create indexer - no async, no runtime!
    // CRDT is optional based on config
    let enable_crdt = std::env::var("PATINA_ENABLE_CRDT").is_ok();
    let indexer = PatternIndexer::new(&project_root, enable_crdt)?;
    
    if !json_output {
        println!("Indexing patterns from {}...", layer_path.display());
    }
    
    // Index in parallel with Rayon
    indexer.index_directory(&layer_path)?;
    
    // Navigate - simple function call
    let response = indexer.navigate(query);
    
    // Display results
    if json_output {
        display_json_results(&response)?;
    } else {
        display_human_results(&response, query)?;
    }
    
    Ok(())
}
```

## Implementation Notes

### What We Built

1. **Synchronous Architecture**
   - Completely removed async/await from navigation system
   - Using `std::sync::Mutex` for thread safety
   - No tokio runtime overhead
   - Clean, simple code that the borrow checker loves

2. **Hybrid Database with Automerge**
   - Created `HybridDatabase` module combining SQLite + Automerge
   - Uses pure Rust Automerge for CRDT operations
   - Optional CRDT layer - works without it
   - SQLite for persistence, Automerge for sync

3. **Parallel Indexing with Rayon**
   - Files indexed in parallel using thread pool
   - Configurable thread count support
   - Progress tracking with atomic counters
   - Thread-safe error collection with parking_lot

### Key Decisions Made

1. **Automerge over CR-SQLite**: Since cr-sqlite isn't available as a Rust crate, we chose Automerge for pure Rust CRDT support
2. **Parking Lot**: Used for faster mutexes in hot paths
3. **Graceful Fallback**: System works without CRDT layer, enables advanced features when activated

### Challenges Encountered

1. **CRDT Library Choice**
   - Discovery: CR-SQLite is not available as a Rust crate on crates.io
   - Solution: Switched to Automerge - a pure Rust CRDT library
   - Impact: Created a hybrid approach with SQLite for storage and Automerge for optional sync
   - Benefit: No external dependencies, pure Rust solution

2. **Async Removal Complexity**
   - Discovery: Async had infected more of the codebase than initially apparent (state_machine, monitoring modules)
   - Solution: Systematically removed all async functions and replaced with synchronous alternatives
   - Learning: The `'static` lifetime requirements of async were indeed problematic as predicted

3. **Database Constraint Violations**
   - Discovery: Parallel indexing revealed race conditions in database writes (UNIQUE constraint failures)
   - Root cause: Multiple threads trying to insert the same concept-document pairs
   - Solution: Wrapped document storage in SQLite transactions
   - Result: No more constraint violations, parallel indexing works perfectly

4. **Progress Reporting in Parallel Processing**
   - Challenge: Thread-safe progress tracking without locks in hot path
   - Solution: Used atomic counters (AtomicUsize) for lock-free progress updates
   - Result: Clean progress reporting without performance impact

5. **Automerge API Learning Curve**
   - Challenge: Automerge's API differs from traditional CRDT libraries
   - Solution: Simplified approach using prefixed keys at root level
   - Result: Clean, working implementation that's easy to understand

## Core Design Principles

### 1. Local-First, Always
- All queries are local (microseconds)
- Network is optional, never required
- Offline is the default, online is a bonus

### 2. Synchronous by Default
- Use OS threads when needed
- Rayon for data parallelism
- Background threads for sync
- No async runtime complexity

### 3. Separation of Concerns
- SQLite handles storage and queries
- Automerge handles distributed sync
- Clear boundaries between systems
- Either can be replaced independently

### 4. Respect the Borrow Checker
- No `'static` requirements
- Borrowing works as designed
- Scoped concurrency when needed
- The compiler remains our friend

### 5. Escape Hatches Everywhere
- Works without Automerge
- Can disable CRDT features
- Can swap CRDT libraries
- SQLite is always the source of truth

## Usage Guide

### Running Navigation

```bash
# Without CRDT (default)
cargo run -- navigate "query"

# With CRDT enabled
PATINA_ENABLE_CRDT=1 cargo run -- navigate "query"
```

### Understanding the Output

```
Using HybridDatabase at .patina/navigation.db (CRDT: enabled)
Indexing 60 markdown files in parallel...
  Progress: 10/60 files indexed
  Progress: 60/60 files indexed
Indexing complete!

🔍 Navigation results for: sqlite

Surface Patterns (Active Development):
  ? surface/git-aware-navigation-design-sqlite.md - Defines sqlite
      untracked
  → surface/git-aware-navigation-design.md - Defines sqlite
      committed: feat: implement git-aware navigation system
```

### CRDT Operations

When CRDT is enabled:
- Patterns are stored in both SQLite and Automerge
- Changes can be synced between peers (future feature)
- Works offline, syncs when connected
- No merge conflicts thanks to CRDT semantics

## Testing Strategy

### Synchronous Tests
```rust
#[test]
fn test_navigation_query() {
    // No async runtime needed!
    let indexer = PatternIndexer::new(temp_dir())?;
    indexer.index_document(&test_file)?;
    
    let results = indexer.navigate("test pattern");
    assert!(!results.locations.is_empty());
}
```

### CRDT Tests
```rust
#[test]
fn test_crdt_merge() {
    let db1 = HybridDatabase::new("db1.sqlite", true)?;
    let db2 = HybridDatabase::new("db2.sqlite", true)?;
    
    // Make changes in both
    db1.add_pattern("auth-pattern", "JWT refresh")?;
    db2.add_pattern("cache-pattern", "Redis TTL")?;
    
    // Sync changes
    let changes = db1.get_changes()?;
    db2.apply_changes(changes)?;
    
    // Both should have both patterns
    assert_eq!(db2.count_patterns()?, 2);
}
```

### Performance Tests
- Parallel indexing with Rayon
- Query performance (target: <1ms)
- Memory usage without async runtime
- CRDT sync overhead

## Conclusion

By removing async and embracing a hybrid SQLite + Automerge approach, we achieve the best of all worlds:
- **Simple code** that looks and feels like Rust
- **Fast performance** with local-first queries
- **Future-proof** with CRDT capabilities built-in
- **True to Rust** - the borrow checker works as intended

This design embodies Patina's philosophy: start simple (local SQLite), grow as needed (CRDT sync), with escape hatches at every level. No compromises, no async tax, just clean Rust code that scales from single-user to distributed teams.

## Dependencies

```toml
[dependencies]
# SQLite for storage
rusqlite = { version = "0.32", features = ["bundled", "chrono", "serde_json"] }

# CRDT support
automerge = "0.5"  # Pure Rust CRDT library

# Parallel processing (better than async for our use case)
rayon = "1.10"

# Simple synchronization
parking_lot = "0.12"  # Faster mutexes

# NO async dependencies needed!

# Keep these
walkdir = "2"
colored = "2"
```

## Implementation Status

### ✅ Phase 1: Synchronous Foundation (COMPLETED)
1. ✅ Removed all `async/await` from navigation code
2. ✅ Replaced `tokio::RwLock` with `std::sync::Mutex`
3. ✅ Used `rayon` for parallel file indexing
4. ✅ Direct SQLite access, no connection pools

### ✅ Phase 2: SQLite + Automerge Integration (COMPLETED)
1. ✅ Added SQLite embedded database (`rusqlite`)
2. ✅ Created HybridDatabase module for SQLite + optional Automerge
3. ✅ Implemented pattern storage in both SQLite and Automerge
4. ✅ Added environment variable control (`PATINA_ENABLE_CRDT`)
5. ✅ Fixed database constraint violations with transactions

### ✅ Phase 3: Navigation Enhancement (COMPLETED)
1. ✅ Create CLI command (`patina navigate <query>`)
2. ✅ Wire up navigation command to PatternIndexer
3. ✅ Display results with git state and confidence
4. ✅ Add layer filtering (`--layer core/surface/dust`)
5. ✅ Add JSON output support (`--json`)
6. ✅ Implement actual git state detection using shell commands
7. ✅ Connect to SQLite with HybridDatabase support
8. ✅ Parallel indexing with progress reporting
9. ⬜ Add workspace hints to results (deferred)
10. ⬜ Implement pattern promotion flow (deferred)

### ⬜ Phase 4: Testing & Refinement
1. ⬜ Unit tests for state machine
2. ⬜ Integration tests with real git repos
3. ⬜ Performance optimization
4. ⬜ Documentation and examples

## Future Enhancements

### Phase 4: P2P Sync with Automerge

#### Background Sync Thread
```rust
// Future: Background sync thread
std::thread::spawn(move || {
    loop {
        // Discover peers (mDNS, DHT, or config)
        let peers = discover_peers()?;
        
        // Exchange CRDT updates
        for peer in peers {
            let changes = db.get_crdt_changes(&last_sync_state)?;
            peer.send_changes(changes)?;
            
            let their_changes = peer.receive_changes()?;
            db.apply_crdt_changes(their_changes)?;
        }
        
        thread::sleep(Duration::from_secs(30));
    }
});
```

#### Selective Sync
```rust
// Sync only certain tables or patterns
db.enable_crdt_sync("patterns")?;
db.enable_crdt_sync("workspace_states")?;
// Keep some tables local-only
```

#### Conflict-Free Collaboration
- **Automatic merging** - CRDTs handle conflicts
- **No central server** - True P2P
- **Offline-first** - Sync when connected
- **Git-like semantics** - But automatic!

### Phase 5: Advanced Features  
1. **Workspace Integration**
   - Connect to workspace service when available
   - Add workspace hints to navigation results
   - Show active exploration branches
   
2. **Pattern Promotion**
   - Add `patina promote` command
   - Move patterns between layers
   - Track promotion history

3. **Git Hook Integration**
   - Install git hooks for real-time tracking
   - Update navigation index on commits
   - Track branch merges automatically

### Phase 4: Testing & Real Git Integration
1. **Integration Tests**
   - Test with real git repositories
   - Verify state transitions
   - Test confidence scoring

2. **File Watcher Implementation**
   - Complete monitoring module
   - Connect file changes to git state updates
   - Test real-time indexing

### Code Locations
- Indexer module: `src/indexer/`
- Database module: `src/indexer/database.rs` (rqlite-rs integration)
- Pattern indexer: `src/indexer/mod.rs` (main coordination)
- State machine: `src/indexer/state_machine.rs`
- Workspace client: `src/workspace_client.rs`
- Workspace service: `workspace/pkg/workspace/`
- Git integration: `workspace/pkg/workspace/git_integration.go`

## Current Working State (2025-08-04)

The `patina navigate` command is fully functional with git state detection and SQLite persistence:

```bash
# Search for patterns (with real git states)
patina navigate "unix philosophy"

# With CRDT support enabled
PATINA_ENABLE_CRDT=1 patina navigate "architecture"

# Filter by layer
patina navigate "architecture" --layer core

# Output as JSON
patina navigate "testing" --json

# Help
patina navigate --help
```

**What's Working:**
- ✅ CLI command with query parsing and options
- ✅ Indexes all markdown files from layer/ directory in parallel
- ✅ Real git state detection using shell commands
- ✅ SQLite persistence with optional Automerge CRDT
- ✅ Confidence scoring based on actual git states
- ✅ Layer filtering and JSON output
- ✅ Git state indicators: "↑" (pushed), "?" (untracked), "M" (modified)
- ✅ Progress reporting during parallel indexing
- ✅ No async runtime overhead!

**Implementation Highlights:**
- **Shell-based git detection** - No git2 dependency, uses standard git CLI
- **Graceful degradation** - Works without CRDT or git
- **Memory-first architecture** - Sub-second queries with SQLite backing
- **Pure Rust** - No external servers required

## Key Design Decisions

1. **Synchronous Architecture**
   - No async/await complexity
   - Standard Rust borrowing works perfectly
   - Rayon for CPU-bound parallel work
   - OS threads for background tasks

2. **SQLite + Optional Automerge**
   - SQLite for fast local storage
   - Automerge CRDT for future P2P sync
   - Works without CRDT layer
   - Pure Rust implementation

3. **Optional Workspace Service**
   - Navigation works standalone
   - Workspace service enhances with container isolation
   - HTTP API when advanced features needed
   - Graceful fallback to git branches

4. **Performance First**
   - Parallel indexing with Rayon
   - Memory cache for queries
   - SQLite for persistence only
   - Microsecond query times


## Key Design Decisions

### 1. Memory-First with Persistence
- Git states cached in memory for fast queries
- rqlite for persistence and history
- Follows indexer-design.md architecture

### 2. Workspace Isolation
- Each exploration gets its own git worktree
- Follows container-use pattern
- Clean separation of experiments

### 3. Confidence as First-Class Concept
- Every location has a confidence score
- Git states modify base confidence
- Transparent scoring explanation

### 4. Event-Driven Updates
- Git hooks trigger immediate updates
- File watchers for real-time tracking
- Async processing for performance

## Testing Strategy

### Unit Tests
- State machine transitions
- Confidence calculations
- Git event parsing

### Integration Tests
- Full workspace lifecycle
- Multi-workspace scenarios
- Pattern promotion flow

### Performance Tests
- Navigation query speed with git states
- Cache efficiency
- Database query optimization

## Success Metrics

1. **Navigation Speed**: <100ms for git-aware queries
2. **State Accuracy**: 100% consistency between git and cache
3. **Pattern Discovery**: 50% reduction in time to find relevant patterns
4. **Confidence Reliability**: User trust in confidence scores

## Key Reference Documents

### Core Architecture
- `layer/core/layer-architecture.md` - Three-layer system (Core/Surface/Dust)
- `layer/core/adapter-pattern.md` - Trait-based adapter design
- `layer/core/unix-philosophy.md` - One tool, one job principle
- `layer/core/pattern-evolution.md` - How patterns move between layers

### Indexer Foundation
- `layer/surface/indexer-design.md` - Base indexer architecture (MUST READ)
- `layer/core/context-orchestration.md` - How context flows through Patina

### Workspace & Git
- `layer/topics/dagger/dagger-container-use.md` - Container-use pattern for isolation
- `layer/sessions/20250729-185446.md` - Evolution from templates to Go service
- `workspace/pkg/workspace/git_integration.go` - Existing git worktree implementation

### Session Integration
- `.claude/bin/session-*.sh` - Session commands that could trigger indexing
- `src/adapters/claude.rs` - How session commands are integrated

## Implementation Notes

### Current State (2025-08-03)
- Foundation complete with all core types
- Code compiles but needs database integration
- State machine ready but not connected to workspace events
- Monitor implemented but needs git status detection

### Next Session Setup
1. Read this design doc first
2. Check `src/indexer/` for current implementation
3. Start with rqlite setup (Phase 2, Task 1)
4. Reference the database schema in this doc

## rqlite vs SQLite Decision

### Why rqlite Instead of Embedded SQLite

While embedded SQLite would be simpler initially, rqlite provides important advantages:

1. **HTTP API from Day One**
   - Any tool can query patterns (VSCode extensions, web UIs, etc.)
   - Language-agnostic access
   - No need to build our own API later

2. **Built-in Growth Path**
   - Single developer: One-node rqlite (current)
   - Small team: Multi-node cluster on same network
   - Large org: Geo-distributed read replicas
   - No architecture changes needed

3. **Operational Simplicity**
   - Despite being a "server", rqlite is designed for embedded use
   - Single binary, no dependencies
   - Can run as companion process (like language servers)

4. **Future Features Without Rework**
   - Pattern sharing between developers
   - Read-only nodes for CI/CD
   - Backup to cloud storage
   - All built into rqlite already

The tradeoff is initial complexity (managing a process) for long-term simplicity (no rearchitecting).

