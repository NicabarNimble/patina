# Spec: Scrape Pipeline

**Architecture Foundation:** [spec-eventlog-architecture.md](./spec-eventlog-architecture.md)

## Overview

**Key Insight:** Git commits and session files ARE event sources. No separate event layer needed.

Following event-sourcing principles (like LiveStore, see architecture spec), we treat existing artifacts as the append-only event log:
- **Git commits** = temporal events (who changed what, when, why)
- **Session files** = development events (decisions, observations, goals)
- **Code files** = current state (not events, but git tracks changes as events)

Scrape materializes these event sources into a unified event log + derived views:

```
Event Sources (git-synced)       Scrape (materialize)      Unified Database (local)
─────────────────────────────    ────────────────────      ────────────────────────
.git/ (commits)               →                         →  patina.db
layer/sessions/*.md           →  patina scrape         →  ├── eventlog (unified)
src/**/*                      →                         →  └── materialized views
```

**Database structure (LiveStore pattern):**
```
patina.db
│
├── eventlog                         ← Source of truth (ALL events)
│   ├── seq (global order)
│   ├── event_type (git.commit, session.decision, etc)
│   ├── timestamp
│   ├── source_id (sha, session_id, etc)
│   └── data (JSON payload)
│
└── Materialized Views               ← Derived from eventlog
    ├── commits, commit_files, co_changes
    ├── sessions, observations, goals
    └── functions, classes, imports, call_graph
```

**Why this works:**
- Events sync via `git pull/push` (no custom sync needed)
- Single database rebuilt locally from event sources (not shared)
- Unified eventlog enables cross-cutting queries
- Can re-scrape with different schemas anytime

## Status: Implementing Unified Database

**Current state:**
- ✓ Unified `patina.db` schema created (eventlog + scrape_meta tables)
- ✓ Git scraper refactored to use unified eventlog
- ✓ Sessions scraper refactored to use unified eventlog
- ⚠ Code scraper still uses separate database (complex, needs careful refactor)

**Completed:**
- [x] Create unified `eventlog` table (2025-11-21)
- [x] Git scraper populates eventlog with git.commit events (2025-11-21)
- [x] Materialized views for git (commits, commit_files, co_changes)
- [x] Sessions scraper populates eventlog with session.* events (2025-11-21)
- [x] Materialized views for sessions (sessions, observations, goals)
- [x] Cross-cutting queries validated (time-based, event correlation)

**Next: Code Scraper Refactor**
- [ ] Minimal integration approach (preserve existing functionality)
- [ ] Switch from code.db to patina.db
- [ ] Add eventlog inserts for key code events
- [ ] Preserve all existing materialized views
- [ ] Full testing with multi-language codebase

**Stats from unified implementation:**
- Git: 702 commits → 702 git.commit events, 112K co-change relationships
- Sessions: 295 sessions → 2,159 session.* events (started, goal, decision, pattern, work, context)
- Total eventlog: 2,861 events across 8 event types
- Database: 31MB patina.db (was 30MB git.db + 455KB sessions.db + 3.1MB code.db)
- Cross-cutting queries working (decisions near commits, events in time windows)

## Components

### 1. Scrape Code
**Command:** `patina scrape code`

**Location:** `src/commands/scrape/code/`

**Output:** `.patina/data/patina.db`

**Populates eventlog with:**
```sql
event_type: 'code.function', 'code.class', 'code.import'
data: {name, signature, path, line, ...}
```

**Creates materialized views:**
- `functions` - extracted functions with signatures
- `classes` - class definitions
- `imports` - dependency relationships
- `call_graph` - caller/callee edges

### 2. Scrape Git ✓
**Command:** `patina scrape git`

**Location:** `src/commands/scrape/git/mod.rs`

**What it does:**
1. Run `git log` with structured format
2. Parse commits, authors, timestamps, files changed
3. Build co-change relationships (files changed together)
4. Store in SQLite

**Output:** `.patina/data/patina.db`

**Populates eventlog with:**
```sql
event_type: 'git.commit'
timestamp: commit timestamp
source_id: commit sha
data: {message, author, email, files: [...]}
```

**Creates materialized views:**
```sql
-- Commits view
CREATE TABLE commits (
    sha TEXT PRIMARY KEY,
    message TEXT,
    author_name TEXT,
    author_email TEXT,
    timestamp TEXT,
    branch TEXT
);

-- Files changed per commit
CREATE TABLE commit_files (
    sha TEXT,
    file_path TEXT,
    change_type TEXT,  -- added, modified, deleted
    lines_added INTEGER,
    lines_removed INTEGER,
    PRIMARY KEY (sha, file_path)
);

-- Co-change relationships (derived)
CREATE TABLE co_changes (
    file_a TEXT,
    file_b TEXT,
    count INTEGER,  -- times changed together
    PRIMARY KEY (file_a, file_b)
);

-- Scrape metadata
CREATE TABLE scrape_meta (
    key TEXT PRIMARY KEY,
    value TEXT
);
-- key: 'last_sha', value: 'abc123' (for incremental)
```

**Implementation:**
```rust
// src/commands/scrape/git/mod.rs
pub fn scrape_git(incremental: bool) -> Result<()> {
    let last_sha = if incremental {
        read_last_scraped_sha()?
    } else {
        None
    };

    let commits = parse_git_log(last_sha)?;

    for commit in &commits {
        insert_commit(&commit)?;
        for file in &commit.files {
            insert_commit_file(&commit.sha, file)?;
        }
    }

    rebuild_co_changes()?;  // Or incremental update
    update_last_sha(&commits.last())?;
    Ok(())
}
```

**CLI:**
```bash
patina scrape git              # Incremental from last scrape
patina scrape git --full       # Full history rebuild
patina scrape git --since 2025-01-01  # Since date
```

### 3. Scrape Sessions ✓
**Command:** `patina scrape sessions`

**Location:** `src/commands/scrape/sessions/mod.rs`

**What it does:**
1. Scan `layer/sessions/*.md`
2. Parse YAML frontmatter (id, started, branch, tags)
3. Extract sections: Goals, Activity Log, Key Decisions
4. Parse observations from activity log
5. Store in SQLite

**Output:** `.patina/data/patina.db`

**Populates eventlog with:**
```sql
event_type: 'session.started', 'session.decision', 'session.observation', 'session.goal'
timestamp: session/observation timestamp
source_id: session_id
source_file: path to .md file
data: {content, type, session_id, ...}
```

**Creates materialized views:**
```sql
-- Sessions view
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,           -- 20251121-065812
    title TEXT,
    started_at TEXT,
    ended_at TEXT,
    branch TEXT,
    classification TEXT,           -- pattern-work, feature, exploration
    files_changed INTEGER,
    commits_made INTEGER
);

-- Observations extracted from sessions
CREATE TABLE observations (
    id INTEGER PRIMARY KEY,
    session_id TEXT,
    content TEXT,
    observation_type TEXT,         -- insight, decision, challenge, pattern
    domains TEXT,                  -- JSON array: ["rust", "embeddings"]
    code_refs TEXT,                -- JSON array of file:line references
    timestamp TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- Goals per session
CREATE TABLE goals (
    id INTEGER PRIMARY KEY,
    session_id TEXT,
    content TEXT,
    completed BOOLEAN,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- Scrape metadata
CREATE TABLE scrape_meta (
    key TEXT PRIMARY KEY,
    value TEXT
);
```

**Parsing logic:**
```rust
// src/commands/scrape/sessions/parser.rs
pub struct ParsedSession {
    pub frontmatter: SessionFrontmatter,
    pub goals: Vec<Goal>,
    pub observations: Vec<Observation>,
    pub classification: Option<String>,
}

pub fn parse_session_file(path: &Path) -> Result<ParsedSession> {
    let content = fs::read_to_string(path)?;
    let (frontmatter, body) = split_frontmatter(&content)?;

    let goals = extract_goals(&body)?;
    let observations = extract_observations(&body)?;
    let classification = extract_classification(&body)?;

    Ok(ParsedSession { frontmatter, goals, observations, classification })
}

fn extract_observations(body: &str) -> Result<Vec<Observation>> {
    // Look for patterns:
    // - "Key Decisions:" section
    // - "**Key insight:**" markers
    // - Bullet points under "Work Completed:"
    // - /session-note entries in Activity Log
}
```

**CLI:**
```bash
patina scrape sessions            # All sessions
patina scrape sessions --since 2025-11-01  # Recent only
patina scrape sessions --file layer/sessions/20251121-065812.md  # Single file
```

### 4. Unified Eventlog Table

**Core Schema:**
```sql
CREATE TABLE eventlog (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,  -- Global ordering
    event_type TEXT NOT NULL,                -- e.g. 'git.commit', 'session.decision'
    timestamp TEXT NOT NULL,                 -- ISO8601 when event occurred
    source_id TEXT NOT NULL,                 -- sha, session_id, function_name, etc
    source_file TEXT,                        -- Original file path
    data JSON NOT NULL,                      -- Event-specific payload
    CHECK(json_valid(data))
);

-- Indexes for common queries
CREATE INDEX idx_eventlog_type ON eventlog(event_type);
CREATE INDEX idx_eventlog_timestamp ON eventlog(timestamp);
CREATE INDEX idx_eventlog_source ON eventlog(source_id);
CREATE INDEX idx_eventlog_type_time ON eventlog(event_type, timestamp);
```

**Event Types:**
- `git.commit` - A commit was made
- `session.started` - Development session began
- `session.decision` - Architectural decision made
- `session.observation` - Pattern or insight observed
- `session.goal` - Goal defined or completed
- `code.function` - Function discovered in codebase
- `code.class` - Class discovered
- `code.import` - Import relationship found

### 5. Unified Scrape Command
**Command:** `patina scrape`

**Runs all three in sequence:**
```bash
patina scrape                     # Equivalent to:
                                  #   patina scrape code
                                  #   patina scrape git
                                  #   patina scrape sessions
```

**With options:**
```bash
patina scrape --full              # Full rebuild all
patina scrape --until 2025-03-15  # Time travel: only events before date
patina scrape --only code,git     # Subset
```

## File Structure

```
.patina/
└── data/
    └── patina.db         # Unified database
        ├── eventlog      # Source of truth (all events)
        └── views         # Materialized views (commits, sessions, functions, etc)
```

## Code Scraper Refactor Plan (Next Session)

**Complexity:** High - 881 lines across extract_v2.rs + database.rs with sophisticated multi-language extraction pipeline

**Approach:** Minimal integration (Option A) - preserve all existing functionality while adding eventlog support

### Current Architecture
```
src/commands/scrape/code/
├── mod.rs              - Main entry point, initialization
├── database.rs         - Custom Database struct with 7 insert methods
├── extract_v2.rs       - Language-agnostic extraction pipeline
├── extracted_data.rs   - Domain types (ExtractedData)
├── types.rs            - CallGraphEntry, etc.
└── languages/          - Modular language processors
    ├── rust.rs
    ├── go.rs
    ├── python.rs
    └── ... (11 languages total)
```

### Refactor Steps (Conservative)

**Step 1: Update Database Path**
- File: `src/commands/scrape/code/mod.rs`
- Change: `initialize_database()` to use `database::PATINA_DB` instead of code.db
- Risk: Low - simple path change

**Step 2: Initialize Unified DB + Code Views**
- File: `src/commands/scrape/code/mod.rs`
- Add: Call `super::database::initialize()` to create eventlog table
- Add: Call `create_code_materialized_views()` to create existing tables
- Risk: Low - additive only

**Step 3: Add Light Eventlog Inserts**
- File: `src/commands/scrape/code/database.rs`
- Modify: Each `insert_*` method to dual-write:
  - Insert code.function/code.class/code.import event into eventlog
  - Keep all existing table inserts unchanged
- Pattern: Same as git/sessions scrapers (eventlog + materialized views)
- Risk: Medium - touches 7 insert methods

**Step 4: Event Type Mapping**
```rust
// Map existing insert methods to event types:
insert_symbols()    → code.symbol events
insert_functions()  → code.function events
insert_types()      → code.class events (structs, enums, etc)
insert_imports()    → code.import events
insert_call_edges() → code.call events
insert_constants()  → code.constant events
insert_members()    → code.member events
```

**Step 5: Preserve All Existing Views**
- Keep all tables from database.rs:init_schema():
  - code_search (symbols with FTS)
  - function_facts (rich metadata)
  - type_vocabulary (types)
  - imports (dependencies)
  - call_graph (edges)
  - constants (extracted values)
  - members (struct fields, methods)
- These become "materialized views" semantically (derived from eventlog)
- No schema changes needed - just dual-write

**Step 6: Testing Strategy**
```bash
# Test with current patina codebase (multi-language)
rm -f .patina/data/patina.db
patina scrape code --full

# Verify eventlog
sqlite3 .patina/data/patina.db "SELECT event_type, COUNT(*) FROM eventlog WHERE event_type LIKE 'code.%' GROUP BY event_type"

# Verify materialized views (should match previous counts)
sqlite3 .patina/data/patina.db "SELECT COUNT(*) FROM code_search"
sqlite3 .patina/data/patina.db "SELECT COUNT(*) FROM function_facts"
sqlite3 .patina/data/patina.db "SELECT COUNT(*) FROM call_graph"

# Test cross-cutting query: functions defined near recent commits
sqlite3 .patina/data/patina.db "
  SELECT c.message, f.file, f.name
  FROM eventlog c
  JOIN eventlog f ON f.source_file = c.data->>'$.files[0].path'
  WHERE c.event_type = 'git.commit'
    AND f.event_type = 'code.function'
  LIMIT 5"
```

### Detailed Code Changes

**File: src/commands/scrape/code/mod.rs**
```rust
// Change this:
const DB_PATH: &str = ".patina/data/code.db";  // OLD

// To this:
use super::database;  // Import unified database module

// In initialize_database():
fn initialize_database(db_path: &str) -> Result<()> {
    // Initialize unified eventlog
    let conn = super::database::initialize(Path::new(database::PATINA_DB))?;

    // Create code-specific materialized views
    create_code_materialized_views(&conn)?;

    Ok(())
}
```

**File: src/commands/scrape/code/database.rs**
```rust
// Add at top:
use crate::commands::scrape::database as unified_db;

// Modify each insert method (example for functions):
pub fn insert_functions(&self, functions: &[FunctionFact]) -> Result<usize> {
    let conn = self.db.connection_mut();
    let tx = conn.transaction()?;

    for func in functions {
        // 1. Insert into eventlog (source of truth)
        let event_data = json!({
            "file": &func.file,
            "name": &func.name,
            "is_async": func.is_async,
            "is_public": func.is_public,
            "parameters": &func.parameters,
            "return_type": &func.return_type,
            // ... all fields
        });

        unified_db::insert_event(
            &tx,
            "code.function",
            &chrono::Utc::now().to_rfc3339(),  // timestamp
            &format!("{}::{}", func.file, func.name),  // source_id
            Some(&func.file),  // source_file
            &event_data.to_string(),
        )?;

        // 2. Insert into materialized view (UNCHANGED - all existing logic preserved)
        tx.execute(
            "INSERT OR REPLACE INTO function_facts (...) VALUES (...)",
            params![/* all params */],
        )?;
    }

    tx.commit()?;
    Ok(functions.len())
}
```

### Success Criteria
- [ ] All existing code scraper tests pass
- [ ] Same number of functions/types/imports extracted as before
- [ ] code.* events appear in unified eventlog
- [ ] Cross-cutting queries work (code near commits, code + sessions)
- [ ] No performance degradation (<10% slowdown acceptable)
- [ ] Database size comparable (eventlog adds ~10-20% overhead)

### Rollback Plan
If refactor has issues:
1. Revert changes to mod.rs and database.rs
2. Keep using code.db separately
3. Document as "Phase 1.5" - complete later when needed

## Integration with Oxidize

Scrape produces unified eventlog. Oxidize consumes it:

```
scrape → eventlog + views → oxidize → USearch

Adapter         | Source
----------------|------------------
semantic        | eventlog WHERE event_type LIKE 'session.%'
temporal        | eventlog WHERE event_type = 'git.commit' (derive co_changes)
dependency      | eventlog WHERE event_type LIKE 'code.%' (derive call_graph)
syntactic       | functions view (derived from code.* events)
architectural   | eventlog (file paths + timestamps)
social          | commits view (derived from git.commit events)
```

**Benefits of unified eventlog:**
- Cross-cutting queries (e.g., "decisions near this commit")
- Time travel (filter by timestamp)
- Consistent ordering (global seq number)
- Single source for embeddings

## Acceptance Criteria

**Phase 1: Individual Scrapers (Complete)**
- [x] `patina scrape git` parses git history (702 commits)
- [x] `patina scrape sessions` parses sessions (295 sessions, 1,424 observations)
- [x] `patina scrape code` parses AST (functions, classes, call_graph)
- [x] `patina scrape` runs all three scrapers
- [x] Incremental scrape only processes new data

**Phase 2: Unified Database (In Progress - 2/3 Complete)**
- [x] Create unified `patina.db` with `eventlog` table (2025-11-21)
- [x] Git scraper populates eventlog with git.commit events (2025-11-21)
- [x] Sessions scraper populates eventlog with session.* events (2025-11-21)
- [x] Materialized views for git (commits, commit_files, co_changes)
- [x] Materialized views for sessions (sessions, observations, goals)
- [x] Cross-cutting queries work (decisions near commits, time windows)
- [ ] **Code scraper** populates eventlog with code.* events (next session)
- [ ] Time travel: `--until` flag filters by timestamp (after code scraper)
- [ ] Same scrape results regardless of order (deterministic)
