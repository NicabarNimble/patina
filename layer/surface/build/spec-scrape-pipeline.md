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

## Status: ✅ COMPLETE (2025-11-22)

**Unified database implementation complete - all three scrapers operational:**

**Completed:**
- [x] Create unified `eventlog` table (2025-11-21)
- [x] Git scraper populates eventlog with git.commit events (2025-11-21)
- [x] Materialized views for git (commits, commit_files, co_changes) (2025-11-21)
- [x] Sessions scraper populates eventlog with session.* events (2025-11-21)
- [x] Materialized views for sessions (sessions, observations, goals) (2025-11-21)
- [x] Code scraper refactored to unified eventlog (2025-11-22)
- [x] Dual-write pattern implemented in all 7 code insert methods (2025-11-22)
- [x] All existing materialized views preserved (2025-11-22)
- [x] Cross-cutting queries validated across all event types (2025-11-22)
- [x] All tests passing (80+ tests) (2025-11-22)

**Final Stats (patina codebase - 114 source files):**
- **Total events**: 16,027 across 17 event types
- **Git**: 707 commits → 707 git.commit events, 112K co-change relationships
- **Sessions**: 296 sessions → 2,174 session.* events (started, goal, decision, pattern, work, context)
- **Code**: 114 files → 13,146 code.* events across 10 types:
  - code.symbol: 1,423 events
  - code.function: 790 events
  - code.struct/enum/trait/class/interface/type: 161 events
  - code.import: 458 events
  - code.call: 9,634 events (60% of all events)
  - code.constant: 203 events
  - code.member: 477 events
- **Database**: 41MB unified patina.db (10MB code + 30MB git + 455KB sessions)
- **Performance**: Code extraction 281ms, Git 61s, Sessions 3s
- **Zero functionality lost**: All 11 language processors preserved, all materialized views intact

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

## Code Scraper Refactor - COMPLETED (2025-11-22)

**Complexity:** High - 881 lines across extract_v2.rs + database.rs with sophisticated multi-language extraction pipeline

**Approach Used:** Minimal integration - preserved all existing functionality while adding eventlog support

### Implementation Summary
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

### Completed Steps

**Step 1: Update Database Path** ✅
- File: `src/commands/scrape/code/mod.rs`
- Changed: `ScrapeConfig::new()` defaults to `database::PATINA_DB`
- Changed: `initialize_database()` now calls `super::database::initialize()` first

**Step 2: Initialize Unified DB + Code Views** ✅
- File: `src/commands/scrape/code/mod.rs:142-148`
- Added: Call to `super::database::initialize()` creates eventlog table
- Preserved: All existing `db.init_schema()` creates code-specific tables

**Step 3: Add Dual-Write Eventlog Inserts** ✅
- File: `src/commands/scrape/code/database.rs`
- Modified: All 7 `insert_*` methods to dual-write:
  - Insert event into eventlog (source of truth)
  - Keep all existing table inserts unchanged (materialized views)
- Pattern: Same as git/sessions scrapers (eventlog + materialized views)

**Step 4: Event Type Mapping** ✅
```rust
// Completed event type mapping:
insert_symbols()    → code.symbol events (1,423 events)
insert_functions()  → code.function events (790 events)
insert_types()      → code.struct/enum/trait/class/interface/type (161 events)
insert_imports()    → code.import events (458 events)
insert_call_edges() → code.call events (9,634 events)
insert_constants()  → code.constant events (203 events)
insert_members()    → code.member events (477 events)
```

**Step 5: All Existing Views Preserved** ✅
- Kept all tables from database.rs:init_schema():
  - code_search (1,423 symbols with FTS)
  - function_facts (790 functions with rich metadata)
  - type_vocabulary (161 types)
  - import_facts (458 dependencies)
  - call_graph (9,634 edges)
  - constant_facts (203 extracted values)
  - member_facts (477 struct fields, methods)
- All tables now "materialized views" semantically (derived from eventlog)
- No schema changes needed - dual-write pattern

**Step 6: Testing Completed** ✅
```bash
# Tested with patina codebase (114 source files, 11 languages)
rm -f .patina/data/patina.db
cargo build --release && cargo install --path .
patina scrape code --full

# Results:
# ✅ 13,146 code.* events in eventlog
# ✅ All materialized views match event counts
# ✅ All 80+ tests passing
# ✅ Cross-cutting queries working (code + git + sessions)
# ✅ Zero functionality lost
```

### Success Criteria - ALL MET ✅
- [x] All existing code scraper tests pass (80+ tests)
- [x] Same number of functions/types/imports extracted as before
- [x] code.* events appear in unified eventlog (13,146 events)
- [x] Cross-cutting queries work (code + git + sessions validated)
- [x] No performance degradation (281ms extraction time)
- [x] Database size acceptable (41MB total, 10MB for code events)

### Key Architectural Decisions

**Dual-Write Pattern:**
- Eventlog = immutable source of truth (append-only)
- Materialized views = query performance (rebuildable from eventlog)
- Best of both worlds: time travel + fast SQL queries

**Event Type Granularity:**
- Fine-grained: separate events for each code entity
- Enables rich queryability (filter by function vs struct vs import)
- Type mapping: `insert_types()` creates code.struct/enum/trait based on kind

**Transaction Semantics:**
- Each insert method uses single transaction for dual-write
- Ensures eventlog + materialized view consistency
- Rollback on error preserves data integrity

**Zero Breaking Changes:**
- All 11 language processors untouched
- All ExtractedData structs unchanged
- All existing queries continue working
- All tests passing without modification (except test setup)

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

## Acceptance Criteria - ALL MET ✅

**Phase 1: Individual Scrapers** ✅
- [x] `patina scrape git` parses git history (707 commits)
- [x] `patina scrape sessions` parses sessions (296 sessions, 1,437 observations)
- [x] `patina scrape code` parses AST (790 functions, 161 types, 9,634 call edges)
- [x] `patina scrape` runs all three scrapers
- [x] Incremental scrape only processes new data

**Phase 2: Unified Database** ✅ COMPLETE (2025-11-22)
- [x] Create unified `patina.db` with `eventlog` table (2025-11-21)
- [x] Git scraper populates eventlog with git.commit events (2025-11-21)
- [x] Sessions scraper populates eventlog with session.* events (2025-11-21)
- [x] Code scraper populates eventlog with code.* events (2025-11-22)
- [x] Materialized views for git (commits, commit_files, co_changes)
- [x] Materialized views for sessions (sessions, observations, goals)
- [x] Materialized views for code (code_search, function_facts, type_vocabulary, import_facts, call_graph, constant_facts, member_facts)
- [x] Cross-cutting queries work (code + git + sessions validated)
- [x] Dual-write pattern implemented (eventlog + materialized views)
- [x] All tests passing (80+ tests)
- [x] Zero functionality lost (all 11 language processors preserved)

**Future Enhancements (Phase 2+):**
- [ ] Time travel: `--until` flag filters eventlog by timestamp
- [ ] Deterministic rebuild: same scrape results regardless of order
- [ ] Incremental eventlog: only append new events on re-scrape
