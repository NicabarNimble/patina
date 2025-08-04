---
id: git-aware-navigation-design
version: 1
status: draft
created_date: 2025-08-03
oxidizer: nicabar
references: [surface/indexer-design.md, core/layer-architecture.md, topics/dagger/dagger-container-use.md, sessions/20250729-185446.md]
tags: [architecture, navigation, git-integration, state-machine, indexer]
---

# Git-Aware Navigation Design

A comprehensive design for integrating git state tracking with Patina's navigation system, enabling confidence-based pattern discovery and workspace-aware indexing.

## Executive Summary

This design extends Patina's indexer (from `indexer-design.md`) with git state awareness, allowing navigation queries to consider the maturity and lifecycle of patterns. By tracking git states (untracked → staged → committed → merged), we can provide confidence scores that reflect real-world pattern adoption.

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

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                   Navigation Query                       │
└─────────────────────┬───────────────────────────────────┘
                      ▼
┌─────────────────────────────────────────────────────────┐
│              PatternIndexer (Rust)                       │
│  ┌─────────────────────────────────────────────────┐    │
│  │          NavigationMap (In-Memory)              │    │
│  │  - Concept mappings                             │    │
│  │  - Document metadata                            │    │
│  │  - Git state cache  ← NEW                      │    │
│  │  - Workspace states ← NEW                      │    │
│  └─────────────────────┬───────────────────────────┘    │
│                        │                                 │
│  ┌─────────────────────┴───────────────────────────┐    │
│  │       GitNavigationStateMachine ← NEW           │    │
│  │  - Monitor workspace changes                    │    │
│  │  - Track state transitions                      │    │
│  │  - Update confidence scores                     │    │
│  └─────────────────────┬───────────────────────────┘    │
└────────────────────────┼─────────────────────────────────┘
                         ▼
┌─────────────────────────────────────────────────────────┐
│          Workspace Service (Go + Dagger)                 │
│  - Git worktree management                              │
│  - Container isolation                                  │
│  - File change detection                                │
└─────────────────────────────────────────────────────────┘
                         ▼
┌─────────────────────────────────────────────────────────┐
│               rqlite Database                           │
│  - Persistent pattern storage                           │
│  - Git state history                                    │
│  - Workspace exploration tracking                       │
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

### 2. State Machine Implementation

```rust
// src/indexer/git_state_machine.rs
pub struct GitNavigationStateMachine {
    indexer: Arc<RwLock<PatternIndexer>>,
    workspace_client: WorkspaceClient,
    git_monitor: GitMonitor,
    state_transitions: Vec<StateTransition>,
}

impl GitNavigationStateMachine {
    pub async fn new(
        indexer: Arc<RwLock<PatternIndexer>>, 
        workspace_client: WorkspaceClient
    ) -> Result<Self> {
        Ok(Self {
            indexer,
            workspace_client,
            git_monitor: GitMonitor::new()?,
            state_transitions: vec![],
        })
    }
    
    /// Process git event and update navigation state
    pub async fn process_git_event(&mut self, event: GitEvent) -> Result<()> {
        match event {
            GitEvent::FileCreated { path, workspace_id } => {
                self.handle_new_file(path, workspace_id).await?;
            },
            GitEvent::FileModified { path, workspace_id } => {
                self.handle_file_modified(path, workspace_id).await?;
            },
            GitEvent::Commit { sha, message, files, workspace_id } => {
                self.handle_commit(sha, message, files, workspace_id).await?;
            },
            GitEvent::Push { remote, branch, workspace_id } => {
                self.handle_push(remote, branch, workspace_id).await?;
            },
            GitEvent::PROpened { number, url, workspace_id } => {
                self.handle_pr_opened(number, url, workspace_id).await?;
            },
            GitEvent::Merged { into_branch, workspace_id } => {
                self.handle_merge(into_branch, workspace_id).await?;
            },
        }
        Ok(())
    }
    
    async fn handle_commit(
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
        
        // Update indexer
        let mut indexer = self.indexer.write().await;
        
        // Extract patterns from commit message
        let patterns = self.extract_patterns_from_commit(&message)?;
        
        // Update confidence for committed files
        for file in &files {
            indexer.update_document_confidence(file, Confidence::Medium)?;
            
            // If it's a pattern file, index it
            if Self::is_pattern_file(file) {
                indexer.index_document(file).await?;
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

### 4. Enhanced Database Schema

```sql
-- Extends schema from indexer-design.md

-- Git state tracking
CREATE TABLE git_states (
    id INTEGER PRIMARY KEY,
    document_id TEXT NOT NULL,
    workspace_id TEXT,
    state TEXT NOT NULL,
    confidence_modifier REAL DEFAULT 1.0,
    metadata JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (document_id) REFERENCES documents(id)
);

-- State transition history
CREATE TABLE state_transitions (
    id INTEGER PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    document_id TEXT,
    from_state TEXT,
    to_state TEXT NOT NULL,
    transition_reason TEXT,
    metadata JSON,
    occurred_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Workspace exploration tracking
CREATE TABLE workspace_explorations (
    workspace_id TEXT PRIMARY KEY,
    branch TEXT NOT NULL,
    pattern_query TEXT NOT NULL,
    state TEXT NOT NULL, -- exploring, reviewing, promoted, abandoned
    discovered_patterns JSON,
    confidence_boost REAL DEFAULT 1.0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    promoted_at TIMESTAMP,
    archived_at TIMESTAMP
);

-- Pattern promotion tracking
CREATE TABLE pattern_promotions (
    id INTEGER PRIMARY KEY,
    pattern_id TEXT NOT NULL,
    from_layer TEXT NOT NULL,
    to_layer TEXT NOT NULL,
    from_workspace TEXT,
    promoted_by TEXT,
    promotion_reason TEXT,
    promoted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for performance
CREATE INDEX idx_git_states_workspace ON git_states(workspace_id);
CREATE INDEX idx_git_states_document ON git_states(document_id);
CREATE INDEX idx_transitions_workspace ON state_transitions(workspace_id);
CREATE INDEX idx_explorations_state ON workspace_explorations(state);
```

### 5. Navigation Query Enhancement

```rust
// src/indexer/navigation_query.rs
impl PatternIndexer {
    pub async fn navigate_with_git_context(&self, query: &str) -> NavigationResponse {
        // 1. Base navigation from memory cache
        let mut locations = self.cache.navigate(query);
        
        // 2. Enrich with git state
        for location in &mut locations {
            if let Some(git_state) = self.cache.git_states.get(&location.path) {
                location.confidence = self.calculate_git_confidence(
                    location.confidence,
                    git_state
                );
                
                location.metadata.insert(
                    "git_state".to_string(),
                    self.format_git_state(git_state)
                );
            }
        }
        
        // 3. Add workspace hints
        let workspace_hints = self.find_active_workspace_hints(query);
        
        // 4. Build response with git context
        NavigationResponse {
            query: query.to_string(),
            locations: self.group_by_layer_and_confidence(locations),
            workspace_hints,
            git_suggestions: self.generate_git_suggestions(query),
            confidence_explanation: self.explain_confidence_scoring(),
        }
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

### 6. Pattern Lifecycle Management

```rust
// src/indexer/pattern_lifecycle.rs
pub struct PatternLifecycle {
    indexer: Arc<RwLock<PatternIndexer>>,
    workspace_client: WorkspaceClient,
    promotion_rules: PromotionRules,
}

impl PatternLifecycle {
    /// Create exploration workspace for a pattern
    pub async fn start_exploration(&mut self, pattern_query: &str) -> Result<ExplorationSession> {
        // 1. Create workspace
        let ws = self.workspace_client.create(&format!("explore/{}", slug(pattern_query))).await?;
        
        // 2. Initialize in database
        self.indexer.write().await.db.execute(
            "INSERT INTO workspace_explorations (workspace_id, branch, pattern_query, state) 
             VALUES (?, ?, ?, 'exploring')",
            &[&ws.id, &ws.branch, &pattern_query]
        ).await?;
        
        // 3. Set up monitoring
        let monitor = WorkspaceMonitor::new(ws.id.clone(), ws.worktree_path.clone());
        monitor.start().await?;
        
        Ok(ExplorationSession {
            workspace_id: ws.id,
            branch: ws.branch,
            query: pattern_query.to_string(),
            monitor,
        })
    }
    
    /// Promote patterns from workspace to higher layer
    pub async fn promote_patterns(&mut self, workspace_id: &str, to_layer: Layer) -> Result<Vec<String>> {
        let mut promoted = vec![];
        
        // Get patterns from workspace
        let patterns = self.get_workspace_patterns(workspace_id).await?;
        
        for pattern in patterns {
            // Check promotion rules
            if self.promotion_rules.can_promote(&pattern, &to_layer)? {
                // Move file to new layer
                let new_path = self.move_pattern_to_layer(&pattern.path, &to_layer)?;
                
                // Update database
                self.record_promotion(&pattern.id, &to_layer, workspace_id).await?;
                
                // Update git state
                self.update_git_state(&new_path, GitState::Merged {
                    into_branch: "main".to_string(),
                    merge_sha: self.get_current_sha()?,
                    timestamp: Utc::now(),
                }).await?;
                
                promoted.push(pattern.id);
            }
        }
        
        // Mark workspace as promoted
        self.mark_workspace_promoted(workspace_id).await?;
        
        Ok(promoted)
    }
}
```

### 7. Real-time Monitoring

```rust
// src/indexer/monitoring.rs
pub struct WorkspaceMonitor {
    workspace_id: String,
    worktree_path: PathBuf,
    watcher: FileWatcher,
    git_monitor: GitMonitor,
    event_tx: mpsc::Sender<MonitorEvent>,
}

impl WorkspaceMonitor {
    pub async fn start(mut self) -> Result<()> {
        // Monitor file changes
        self.watcher.watch(&self.worktree_path, RecursiveMode::Recursive)?;
        
        // Monitor git state
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                // Check git status
                if let Ok(status) = self.git_monitor.get_status(&self.worktree_path).await {
                    if status.has_changes() {
                        let _ = self.event_tx.send(MonitorEvent::GitStatusChanged {
                            workspace_id: self.workspace_id.clone(),
                            status,
                        }).await;
                    }
                }
            }
        });
        
        Ok(())
    }
}
```

## Implementation Status

### ✅ Phase 1: Foundation (COMPLETED - 2025-08-03)
1. ✅ Created git state types and state machine structure (`src/indexer/git_state.rs`)
2. ✅ Extended NavigationMap with git state tracking (`src/indexer/navigation_state.rs`)
3. ✅ Created GitNavigationStateMachine (`src/indexer/state_machine.rs`)
4. ✅ Created basic workspace monitor (`src/indexer/monitoring.rs`)
5. ✅ Fixed all compilation errors - code builds successfully

### ✅ Phase 2: Integration (COMPLETED - 2025-08-03)
1. ✅ Set up rqlite database connection using rqlite-rs 0.6.1
2. ✅ Add database schema with all tables (documents, concepts, git_states, etc.)
3. ✅ Implement PatternIndexer with memory-first architecture
4. ✅ Add document analysis (frontmatter parsing, concept extraction)
5. ✅ Wire up git state tracking with confidence scoring
6. ⬜ Connect to existing workspace service (deferred)
7. ⬜ Implement git hook handlers (deferred)

### ✅ Phase 3: Navigation Enhancement (COMPLETED - 2025-08-04)
1. ✅ Create CLI command (`patina navigate <query>`)
2. ✅ Wire up navigation command to PatternIndexer
3. ✅ Display results with git state and confidence
4. ✅ Add layer filtering (`--layer core/surface/dust`)
5. ✅ Add JSON output support (`--json`)
6. ✅ Implement actual git state detection using shell commands
7. ✅ Connect to rqlite for persistence with graceful fallback
8. ⬜ Add workspace hints to results (deferred)
9. ⬜ Implement pattern promotion flow (deferred)
10. ⬜ Create git suggestion engine (deferred)

### ⬜ Phase 4: Testing & Refinement
1. ⬜ Unit tests for state machine
2. ⬜ Integration tests with real git repos
3. ⬜ Performance optimization
4. ⬜ Documentation and examples

## TODO for Next Session

### Continue Phase 3: Navigation Enhancement
1. **Implement Git State Detection**
   - Add actual git status checking for files
   - Use git2 crate or shell commands
   - Map git status to appropriate GitState enum variants

2. **Connect to rqlite Database**
   - Initialize rqlite connection in navigate command
   - Load existing index from database
   - Persist indexed documents to database

3. **Add Workspace Hints**
   - Implement workspace detection
   - Show active branches exploring patterns
   - Display workspace-specific confidence boosts

4. **Pattern Promotion Commands**
   - Add `patina promote` command
   - Move patterns between layers
   - Update git states on promotion

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

### Current Working State (2025-08-04)

The `patina navigate` command is fully functional with git state detection and database persistence:

```bash
# Start rqlite for persistence
docker compose up -d

# Search for patterns (with real git states)
patina navigate "unix philosophy"

# Filter by layer
patina navigate "architecture" --layer core

# Output as JSON
patina navigate "testing" --json

# Help
patina navigate --help
```

**What's Working:**
- ✅ CLI command with query parsing and options
- ✅ Indexes all markdown files from layer/ directory
- ✅ Real git state detection using shell commands
- ✅ rqlite persistence with automatic fallback to memory
- ✅ Confidence scoring based on actual git states
- ✅ Layer filtering and JSON output
- ✅ Git state indicators: "↑" (pushed), "?" (untracked), "M" (modified)

**Implementation Highlights:**
- **Shell-based git detection** - No git2 dependency, uses standard git CLI
- **Graceful degradation** - Works without rqlite or git
- **Memory-first architecture** - Sub-second queries with database backing
- **Docker-compose setup** - Easy rqlite deployment

### Implementation Decisions (2025-08-04)

1. **Git Detection via Shell Commands**
   - Chose shell commands over git2 crate for minimal dependencies
   - Aligns with Patina's escape hatch philosophy
   - Uses `git status --porcelain` for file states
   - Uses `git log` and `git branch -r` for commit history

2. **rqlite for Persistence**
   - Chose rqlite over embedded SQLite for future growth path
   - Single-node mode for development (no clustering complexity)
   - HTTP API enables tool ecosystem (VSCode, web UIs, etc.)
   - Currently requires manual Docker setup:
     ```bash
     docker compose up -d  # Start rqlite
     patina navigate      # Use navigation
     ```

3. **Performance Optimizations**
   - Batch git status detection for multiple files
   - Memory-first navigation (microsecond queries)
   - Database for persistence only, not real-time queries
   - Concept extraction optimized for common patterns

### Future rqlite Integration Improvements

The current implementation requires manual rqlite management. Future improvements should include:

1. **Auto-start rqlite** when navigation is used
2. **Multiple startup methods**:
   - Docker Compose (if Docker available)
   - Local rqlited binary (if installed)
   - Download rqlited on demand
3. **Project-local data** in `.patina/rqlite/`
4. **Process management** to start/stop with Patina
5. **Single-node optimization** - no clustering overhead for solo developers

This would make rqlite feel embedded while preserving the growth path to team collaboration.

### Key Implementation Notes (Phase 2)

1. **Database Choice**: Using rqlite-rs 0.6.1 for decentralized pattern sharing
   - Parameterized queries with `query!` macro
   - Type-safe results with `FromRow` derive
   - Connection pooling built-in

2. **Memory-First Architecture**: 
   - `GitAwareNavigationMap` provides sub-second queries
   - Database for persistence and cross-node sharing
   - Follows indexer-design.md patterns

3. **Document Analysis**:
   - YAML frontmatter parsing
   - Concept extraction from headings and tags
   - Layer detection from path structure

4. **Git State Integration**:
   - States map to confidence levels (Untracked→Experimental, Merged→Verified)
   - State machine tracks transitions
   - Navigation enriches results with git context

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

## Future Enhancements

1. **Machine Learning**: Learn confidence weights from user behavior
2. **Cross-Project Navigation**: Share git states across projects
3. **PR Review Integration**: Pull review comments into navigation
4. **Semantic Git Analysis**: Understand commit intent beyond keywords