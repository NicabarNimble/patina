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
- ⚠ Sessions and code scrapers still use separate databases

**Completed:**
- [x] Create unified `eventlog` table (2025-11-21)
- [x] Git scraper populates eventlog with git.commit events (2025-11-21)
- [x] Materialized views for git (commits, commit_files, co_changes)

**In Progress:**
- [ ] Refactor sessions scraper to populate eventlog
- [ ] Refactor code scraper to populate eventlog
- [ ] Validate cross-cutting queries across all event types

**Stats from unified implementation:**
- Git: 702 commits → 702 git.commit events, 112K co-change relationships
- Sessions: 294 sessions (still in sessions.db, needs refactor)
- Code: AST, call_graph (still in code.db, needs refactor)

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
- [x] `patina scrape git` parses git history (689 commits)
- [x] `patina scrape sessions` parses sessions (294 sessions, 1,393 observations)
- [x] `patina scrape code` parses AST (functions, classes, call_graph)
- [x] `patina scrape` runs all three scrapers
- [x] Incremental scrape only processes new data

**Phase 2: Unified Database (Next)**
- [ ] Create unified `patina.db` with `eventlog` table
- [ ] All scrapers populate eventlog with typed events
- [ ] Materialized views derived from eventlog
- [ ] Cross-cutting queries work (e.g., decisions near commits)
- [ ] Time travel: `--until` flag filters by timestamp
- [ ] Same scrape results regardless of order (deterministic)
