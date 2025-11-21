# Spec: Scrape Pipeline

## Overview

Scrape extracts structure from existing sources of truth. No event emission needed - git, sessions, and code are already logs. We parse them into queryable SQLite tables.

```
Raw Sources              Scrape                   Structured Data
─────────────────────    ─────────────────────    ─────────────────────
.git/                 →  patina scrape git     →  git.db
layer/sessions/*.md   →  patina scrape sessions → sessions.db
src/**/*              →  patina scrape code    →  code.db (exists)
```

## Current State

- `patina scrape code` exists - parses AST, builds call_graph
- `code.db` has: functions, classes, imports, call_graph tables
- ~290 session files in `layer/sessions/`
- Full git history available

## Components

### 1. Scrape Code (exists)
**Command:** `patina scrape code`

**Location:** `src/commands/scrape/code/`

**Output:** `.patina/data/code.db`
- `functions` - extracted functions with signatures
- `classes` - class definitions
- `imports` - dependency relationships
- `call_graph` - caller/callee edges

**Already working** - no changes needed.

### 2. Scrape Git (new)
**Command:** `patina scrape git`

**Location:** `src/commands/scrape/git/mod.rs`

**What it does:**
1. Run `git log` with structured format
2. Parse commits, authors, timestamps, files changed
3. Build co-change relationships (files changed together)
4. Store in SQLite

**Output:** `.patina/data/git.db`

**Tables:**
```sql
-- Commits
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

### 3. Scrape Sessions (new)
**Command:** `patina scrape sessions`

**Location:** `src/commands/scrape/sessions/mod.rs`

**What it does:**
1. Scan `layer/sessions/*.md`
2. Parse YAML frontmatter (id, started, branch, tags)
3. Extract sections: Goals, Activity Log, Key Decisions
4. Parse observations from activity log
5. Store in SQLite

**Output:** `.patina/data/sessions.db`

**Tables:**
```sql
-- Sessions
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

### 4. Unified Scrape Command
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
patina scrape --only code,git     # Subset
```

## File Structure

```
.patina/
└── data/
    ├── code.db           # AST, call_graph (exists)
    ├── git.db            # commits, co-changes (new)
    └── sessions.db       # sessions, observations (new)
```

## Integration with Oxidize

Scrape produces SQLite tables. Oxidize consumes them:

```
scrape → SQLite → oxidize → USearch

Adapter         | Source Table
----------------|------------------
semantic        | sessions.observations
temporal        | git.co_changes
dependency      | code.call_graph
syntactic       | code.functions (AST)
architectural   | code.functions (paths)
social          | git.commits (authors)
```

## Acceptance Criteria

- [ ] `patina scrape git` parses git history into git.db
- [ ] `patina scrape sessions` parses all 290 sessions into sessions.db
- [ ] `patina scrape` runs all three scrapers
- [ ] Incremental scrape only processes new data
- [ ] `--full` flag rebuilds from scratch
- [ ] Scrape metadata tracks last processed item for incremental
