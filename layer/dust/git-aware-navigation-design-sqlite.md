---
id: git-aware-navigation-design-sqlite
version: 2
status: archived
archived_date: 2025-08-04
created_date: 2025-08-04
oxidizer: nicabar
references: [surface/git-aware-navigation-design.md, core/layer-architecture.md, core/escape-hatch-philosophy.md, external/no-boilerplate-async-rust]
tags: [architecture, navigation, git-integration, sqlite, crdt, synchronous, local-first, automerge, archived]
superseded_by: surface/git-aware-navigation-design.md
archive_reason: Merged into main git-aware-navigation-design.md document
---

# Git-Aware Navigation Design (SQLite + Automerge CRDT) [ARCHIVED]

> **ARCHIVED**: This document has been merged into `surface/git-aware-navigation-design.md`. Kept for historical reference and to preserve the implementation journey.

A fundamental redesign of Patina's navigation system using synchronous, local-first SQLite for storage with Automerge CRDT layer for distributed features, eliminating async complexity while maintaining flexibility.

## Executive Summary

This design reimagines Patina's navigation system based on a key insight: **we don't need async**. By using SQLite for storage and Automerge for CRDT capabilities, we achieve:
- **Simplicity** - No async runtime, no static lifetime infections
- **Performance** - Microsecond local SQLite queries
- **Flexibility** - CRDT layer is optional and replaceable
- **True Rust** - Pure Rust dependencies, borrow checker works as intended
- **Escape Hatches** - SQLite works without CRDT, can swap CRDT libraries

## The Async Trap

Our original design used async because rqlite required network I/O. But this infected our entire codebase with unnecessary complexity:
- `tokio::Runtime` for simple file reads
- `'static` lifetime requirements breaking borrowing
- Complex error handling for simple operations
- Runtime state instead of compile-time guarantees

## The Realization

Patina's workload is inherently synchronous:
- **Local file I/O** - Reading markdown files
- **SQLite queries** - Microsecond operations
- **Git commands** - Subprocess calls
- **Pattern indexing** - CPU-bound work

No network I/O in the hot path = no need for async!

## The New Architecture

### 1. Hybrid Storage: SQLite + Automerge
- **SQLite** - Fast local storage and queries
- **Automerge** - CRDT layer for distributed sync
- **Synchronous API** - Direct queries, no awaits
- **Optional CRDT** - Works without Automerge, enhances with it
- **Replaceable** - Can swap Automerge for other CRDT libraries

### 2. Concurrency Model
- **Rayon** - For parallel file indexing
- **Thread scopes** - For borrowed data in background tasks
- **No async runtime** - The OS already provides one!
- **Simple mutexes** - For shared state protection

### 3. Benefits
- **No static infections** - Borrowing works normally
- **Simpler code** - What you see is what runs
- **Better errors** - No async stack traces
- **True zero-cost** - No runtime overhead

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
│  │  - Direct git CLI calls                         │    │
│  │  - Synchronous state updates                    │    │
│  │  - No await points!                             │    │
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
└─────────────────────┬───────────────────────────────────┘
                      ▼
┌─────────────────────────────────────────────────────────┐
│          Future: Background Sync Thread                  │
│  - Runs separately from queries                         │
│  - Uses thread::spawn, not tokio                        │
│  - Exchanges Automerge documents with peers             │
└─────────────────────────────────────────────────────────┘
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

### 3. Synchronous PatternIndexer

```rust
// src/indexer/mod.rs
use std::sync::{Arc, Mutex};
use rayon::prelude::*;

pub struct PatternIndexer {
    /// In-memory navigation cache for fast queries
    cache: Arc<Mutex<GitAwareNavigationMap>>,
    /// Hybrid database (SQLite + optional Automerge)
    db: Arc<HybridDatabase>,
    /// Git state machine (synchronous)
    state_machine: Arc<Mutex<GitNavigationStateMachine>>,
}

impl PatternIndexer {
    /// Create indexer with hybrid database
    pub fn new(project_root: &Path, enable_crdt: bool) -> Result<Self> {
        let db_path = project_root.join(".patina/navigation.db");
        let db = Arc::new(HybridDatabase::new(&db_path, enable_crdt)?);;
        db.initialize_schema()?;
        
        // Load existing data into cache
        let cache = Arc::new(Mutex::new(GitAwareNavigationMap::new()));
        let state_machine = Arc::new(Mutex::new(GitNavigationStateMachine::new()?));
        
        Ok(Self { cache, db, state_machine })
    }
    
    /// Navigate - pure synchronous function!
    pub fn navigate(&self, query: &str) -> NavigationResponse {
        let cache = self.cache.lock().unwrap();
        let mut response = cache.navigate(query);
        
        // Enrich with git states - no await!
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
    
    /// Index a single document - synchronous
    pub fn index_document(&self, path: &Path) -> Result<()> {
        // Simple, synchronous file read
        let content = std::fs::read_to_string(path)?;
        let doc_info = self.analyze_document(path, &content)?;
        
        // Update cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert_document(doc_info.clone());
        }
        
        // Persist to database
        self.db.store_document(&doc_info)?;
        
        // Update git state
        {
            let mut state_machine = self.state_machine.lock().unwrap();
            state_machine.track_document(&doc_info.path);
        }
        
        Ok(())
    }
}
```

### 4. Simple Navigate Command

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

### 5. Dependencies for Hybrid Architecture

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

## Implementation Plan

### Phase 1: Synchronous Foundation ✓
1. Remove all `async/await` from navigation code
2. Replace `tokio::RwLock` with `std::sync::Mutex`
3. Use `rayon` for parallel file indexing
4. Direct SQLite access, no connection pools

### Phase 2: CRDT Integration
1. Add cr-sqlite dependency
2. Create regular tables for documents/concepts
3. Create CRDT table for patterns
4. Test local operations

### Phase 3: Background Sync (Future)
1. Add peer discovery mechanism
2. Create background sync thread
3. Exchange CRDT changes with peers
4. Monitor and merge in background

## Future Growth with CRDTs

### 1. P2P Pattern Sync
```rust
// Future: Background sync thread
std::thread::spawn(move || {
    loop {
        // Discover peers (mDNS, DHT, or config)
        let peers = discover_peers()?;
        
        // Exchange CRDT updates
        for peer in peers {
            let changes = db.changes_since(last_sync)?;
            peer.send_changes(changes)?;
            
            let their_changes = peer.receive_changes()?;
            db.apply_changes(their_changes)?;
        }
        
        thread::sleep(Duration::from_secs(30));
    }
});
```

### 2. Selective Sync
```rust
// Sync only certain tables or patterns
conn.execute("SELECT crsql_as_crdt('workspace_patterns')")?;
conn.execute("SELECT crsql_as_crdt('shared_decisions')")?;
// Keep some tables local-only
```

### 3. Conflict-Free Collaboration
- **Automatic merging** - CRDTs handle conflicts
- **No central server** - True P2P
- **Offline-first** - Sync when connected
- **Git-like semantics** - But automatic!

## Benefits of Synchronous + CRDT Design

### Immediate Benefits
1. **No async complexity** - Simple, readable code
2. **Borrowing works** - No `'static` lifetime infections
3. **Fast queries** - Microsecond local SQLite access
4. **Parallel indexing** - Rayon for CPU-bound work
5. **Easier debugging** - Standard tools, simple stack traces

### Future Benefits
1. **Distributed by design** - CRDT schema from day one
2. **Conflict-free sync** - No merge conflicts ever
3. **Offline-first** - Local-first with optional sync
4. **True P2P** - No servers, no cloud dependency

## Implementation Checklist

### Phase 1: Remove Async ✅
1. ✅ Replace all `async fn` with `fn`
2. ✅ Change `tokio::RwLock` to `std::sync::Mutex`
3. ✅ Remove `tokio::Runtime` from navigate command
4. ✅ Update PatternIndexer to be synchronous
5. ✅ Remove `.await` from all navigation code

### Phase 2: Add Automerge CRDT ✅
1. ✅ Add automerge dependency (pure Rust CRDT)
2. ✅ Create HybridDatabase module combining SQLite + Automerge
3. ✅ Initialize with optional CRDT support
4. ✅ Implement pattern storage and retrieval
5. ✅ Add basic sync capabilities

### Phase 3: Optimize with Rayon ✅
1. ✅ Add rayon dependency
2. ✅ Implement parallel file indexing
3. ✅ Add configurable thread pool support
4. ✅ Add progress reporting with atomic counters

### Phase 4: Automerge CRDT Implementation ✅
1. ✅ Implement pattern storage in Automerge documents
2. ✅ Add get_patterns() for CRDT retrieval
3. ✅ Implement basic change serialization
4. ✅ Fix database constraint violations with transactions
5. ✅ Test with CRDT enabled and disabled

### Phase 5: Future P2P Sync ⬜
1. ⬜ Design peer discovery mechanism
2. ⬜ Implement background sync thread
3. ⬜ Add sync status to UI
4. ⬜ Test P2P pattern sharing

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

### Current Status

- Navigation system is fully synchronous ✅
- Parallel indexing with Rayon implemented ✅
- Hybrid SQLite + Automerge database complete ✅
- Database constraint violations fixed ✅
- CRDT sync capabilities implemented ✅
- Both CRDT-enabled and disabled modes work ✅

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

3. **Database Constraint Violations** ✅
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
    let db1 = CrSqliteDatabase::new("db1.sqlite")?;
    let db2 = CrSqliteDatabase::new("db2.sqlite")?;
    
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

## References

1. [No Boilerplate: Async Rust Is A Bad Language](https://www.youtube.com/watch?v=5M4ZpUltPMk)
2. [cr-sqlite Documentation](https://github.com/vlcn-io/cr-sqlite)
3. [Rayon: Data Parallelism in Rust](https://github.com/rayon-rs/rayon)
4. [CRDTs: Conflict-free Replicated Data Types](https://crdt.tech/)