# Neuro-Symbolic Design: SQLite + Scryer Prolog Integration

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    PATINA DEVELOPMENT DOMAIN                     │
│                   (Self-referential: building Patina)            │
└─────────────────────────────────────────────────────────────────┘
                                 │
                    ┌────────────┴────────────┐
                    │                         │
         ┌──────────▼─────────┐    ┌─────────▼──────────┐
         │  227 Session Files │    │  Git History       │
         │  layer/sessions/   │    │  Commits & Tags    │
         └──────────┬─────────┘    └─────────┬──────────┘
                    │                         │
                    └────────────┬────────────┘
                                 │
                        ┌────────▼────────┐
                        │  LLM Extraction │
                        │  (Neural)       │
                        └────────┬────────┘
                                 │
                    ┌────────────┴────────────┐
                    │                         │
         ┌──────────▼─────────┐    ┌─────────▼──────────┐
         │   facts.db         │    │   facts.pl         │
         │   (SQLite)         │◄───┤   (Prolog export)  │
         │   Efficient query  │    │   Logical inference│
         └──────────┬─────────┘    └─────────┬──────────┘
                    │                         │
                    └────────────┬────────────┘
                                 │
                        ┌────────▼────────┐
                        │  rules.pl       │
                        │  (Symbolic)     │
                        └────────┬────────┘
                                 │
                    ┌────────────┴────────────┐
                    │                         │
         ┌──────────▼─────────┐    ┌─────────▼──────────┐
         │  SQL Queries       │    │  Prolog Queries    │
         │  Fast aggregation  │    │  Inference & logic │
         └──────────┬─────────┘    └─────────┬──────────┘
                    │                         │
                    └────────────┬────────────┘
                                 │
                        ┌────────▼────────┐
                        │  Insights       │
                        │  Pattern Evo    │
                        │  Knowledge Graph│
                        └─────────────────┘
```

## Database Schema (facts.db)

### Schema Design

Follows existing patina pattern from `src/commands/scrape/code/database.rs`:

```sql
-- ============================================================================
-- SESSION KNOWLEDGE SCHEMA
-- ============================================================================

-- Core session metadata
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,                    -- e.g. '20251010-061739'
    started_at TIMESTAMP NOT NULL,          -- ISO 8601
    work_type TEXT NOT NULL,                -- exploration, pattern-work, experiment
    git_branch TEXT,
    starting_commit TEXT,
    session_tag TEXT,
    llm TEXT DEFAULT 'claude',
    commits_count INTEGER DEFAULT 0,
    files_changed INTEGER DEFAULT 0,
    duration_minutes INTEGER,               -- Session length
    INDEX idx_work_type (work_type),
    INDEX idx_branch (git_branch),
    INDEX idx_date (started_at)
);

-- Pattern observations (many-to-many with sessions)
CREATE TABLE IF NOT EXISTS patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    pattern_name TEXT NOT NULL,
    category TEXT NOT NULL,                 -- security, architecture, workflow, infrastructure
    description TEXT,
    first_seen TIMESTAMP,
    last_seen TIMESTAMP,
    observation_count INTEGER DEFAULT 1,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    INDEX idx_pattern_name (pattern_name),
    INDEX idx_category (category)
);

-- Technology usage
CREATE TABLE IF NOT EXISTS technologies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    tech_name TEXT NOT NULL,
    purpose TEXT NOT NULL,
    tech_category TEXT,                     -- language, tool, framework, service
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    INDEX idx_tech_name (tech_name)
);

-- Key decisions made during sessions
CREATE TABLE IF NOT EXISTS decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    choice TEXT NOT NULL,
    rationale TEXT NOT NULL,
    decision_type TEXT,                     -- philosophical, pragmatic, technical
    alternatives_considered TEXT,           -- JSON array of alternatives
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- Challenges and solutions
CREATE TABLE IF NOT EXISTS challenges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    problem TEXT NOT NULL,
    solution TEXT NOT NULL,
    challenge_category TEXT,                -- performance, security, architecture, tooling
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    INDEX idx_category (challenge_category)
);

-- Work completed (narrative entries)
CREATE TABLE IF NOT EXISTS work_completed (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    description TEXT NOT NULL,
    work_category TEXT,                     -- implementation, debugging, research, refactoring
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- Git commits during session
CREATE TABLE IF NOT EXISTS session_commits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    commit_hash TEXT NOT NULL,
    commit_message TEXT,
    timestamp TIMESTAMP,
    files_modified INTEGER,
    insertions INTEGER,
    deletions INTEGER,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    INDEX idx_commit_hash (commit_hash)
);

-- Goals (checkbox items from session)
CREATE TABLE IF NOT EXISTS goals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    description TEXT NOT NULL,
    completed BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- ============================================================================
-- KNOWLEDGE GRAPH: CROSS-DOMAIN LINKS
-- ============================================================================

-- Domain definitions
CREATE TABLE IF NOT EXISTS domains (
    name TEXT PRIMARY KEY,                  -- e.g. 'patina-dev', 'rust-development', 'security'
    description TEXT,
    bucket_path TEXT,                       -- Path to domain bucket
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Cross-domain relationships (the UNSOLVED PROBLEM)
CREATE TABLE IF NOT EXISTS domain_links (
    source_domain TEXT NOT NULL,
    target_domain TEXT NOT NULL,
    relationship TEXT NOT NULL,             -- 'implements-patterns-from', 'uses-tools-from', etc.
    strength REAL DEFAULT 1.0,              -- 0.0-1.0 confidence/importance
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (source_domain) REFERENCES domains(name),
    FOREIGN KEY (target_domain) REFERENCES domains(name),
    PRIMARY KEY (source_domain, target_domain, relationship)
);

-- Pattern relationships (patterns used together)
CREATE TABLE IF NOT EXISTS pattern_correlations (
    pattern1 TEXT NOT NULL,
    pattern2 TEXT NOT NULL,
    session_id TEXT NOT NULL,
    PRIMARY KEY (pattern1, pattern2, session_id),
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- Technology pairs (techs used together)
CREATE TABLE IF NOT EXISTS tech_pairs (
    tech1 TEXT NOT NULL,
    tech2 TEXT NOT NULL,
    session_id TEXT NOT NULL,
    PRIMARY KEY (tech1, tech2, session_id),
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- ============================================================================
-- EXTRACTION METADATA
-- ============================================================================

-- Track extraction state
CREATE TABLE IF NOT EXISTS extraction_state (
    session_file TEXT PRIMARY KEY,          -- layer/sessions/20251010-061739.md
    extracted_at TIMESTAMP,
    extractor_version TEXT,
    facts_count INTEGER,
    extraction_errors TEXT                  -- JSON array of errors if any
);
```

## Rust Implementation

### Module Structure

```
src/
├── commands/
│   ├── scrape/
│   │   └── code/
│   │       └── database.rs        # ✅ Existing (code facts)
│   └── session/                   # 🆕 New module
│       ├── mod.rs                 # Command: patina session extract
│       ├── database.rs            # Session facts database
│       ├── extractor.rs           # LLM-based fact extraction
│       ├── prolog_export.rs       # Export facts.db → facts.pl
│       └── query.rs               # Query interface
└── lib.rs
```

### Database Module

```rust
// src/commands/session/database.rs
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;

// ============================================================================
// DOMAIN TYPES
// ============================================================================

#[derive(Debug, Clone)]
pub struct SessionFact {
    pub id: String,
    pub started_at: String,
    pub work_type: String,
    pub git_branch: Option<String>,
    pub commits_count: i32,
    pub files_changed: i32,
}

#[derive(Debug, Clone)]
pub struct PatternObservation {
    pub session_id: String,
    pub pattern_name: String,
    pub category: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TechnologyUsage {
    pub session_id: String,
    pub tech_name: String,
    pub purpose: String,
}

#[derive(Debug, Clone)]
pub struct Decision {
    pub session_id: String,
    pub choice: String,
    pub rationale: String,
    pub decision_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Challenge {
    pub session_id: String,
    pub problem: String,
    pub solution: String,
    pub category: Option<String>,
}

// ============================================================================
// DATABASE
// ============================================================================

pub struct SessionDatabase {
    conn: Connection,
}

impl SessionDatabase {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn init_schema(&mut self) -> Result<()> {
        // Create all tables from schema above
        // (See full SQL in schema section)
        Ok(())
    }

    /// Bulk insert sessions
    pub fn insert_sessions(&self, sessions: &[SessionFact]) -> Result<usize> {
        if sessions.is_empty() {
            return Ok(0);
        }

        let mut stmt = self.conn.prepare(
            "INSERT OR REPLACE INTO sessions (id, started_at, work_type, git_branch, commits_count, files_changed)
             VALUES (?, ?, ?, ?, ?, ?)"
        )?;

        for session in sessions {
            stmt.execute(params![
                &session.id,
                &session.started_at,
                &session.work_type,
                &session.git_branch,
                session.commits_count,
                session.files_changed,
            ])?;
        }

        Ok(sessions.len())
    }

    /// Bulk insert pattern observations
    pub fn insert_patterns(&self, patterns: &[PatternObservation]) -> Result<usize> {
        if patterns.is_empty() {
            return Ok(0);
        }

        let mut stmt = self.conn.prepare(
            "INSERT OR REPLACE INTO patterns (session_id, pattern_name, category, description)
             VALUES (?, ?, ?, ?)"
        )?;

        for pattern in patterns {
            stmt.execute(params![
                &pattern.session_id,
                &pattern.pattern_name,
                &pattern.category,
                &pattern.description,
            ])?;
        }

        Ok(patterns.len())
    }

    // ... similar for technologies, decisions, challenges
}
```

### LLM Extractor

```rust
// src/commands/session/extractor.rs
use anyhow::Result;
use std::path::Path;

/// Extract structured facts from session markdown using LLM
pub struct SessionExtractor {
    // Could use Claude, Gemini, or local model
}

impl SessionExtractor {
    pub fn extract_from_file(&self, session_path: &Path) -> Result<ExtractedFacts> {
        // 1. Read session markdown
        let content = std::fs::read_to_string(session_path)?;

        // 2. Send to LLM with structured extraction prompt
        let prompt = format!(
            r#"Extract structured facts from this Patina session:

{content}

Return JSON with:
- session metadata (id, date, work_type, branch, commits, files_changed)
- patterns observed (pattern_name, category, description)
- technologies used (tech_name, purpose)
- decisions made (choice, rationale, type)
- challenges faced (problem, solution, category)
- work completed (description, category)

JSON:"#
        );

        // 3. Parse LLM response into structs
        // 4. Validate and return
        todo!("LLM integration")
    }
}

#[derive(Debug)]
pub struct ExtractedFacts {
    pub session: SessionFact,
    pub patterns: Vec<PatternObservation>,
    pub technologies: Vec<TechnologyUsage>,
    pub decisions: Vec<Decision>,
    pub challenges: Vec<Challenge>,
}
```

### Prolog Export

```rust
// src/commands/session/prolog_export.rs
use anyhow::Result;
use rusqlite::Connection;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Export SQLite facts to Prolog format
pub struct PrologExporter {
    conn: Connection,
}

impl PrologExporter {
    pub fn export_to_file(&self, output_path: &Path) -> Result<()> {
        let mut file = File::create(output_path)?;

        writeln!(file, "% Facts exported from facts.db")?;
        writeln!(file, "% Generated: {}", chrono::Utc::now())?;
        writeln!(file)?;

        // Export sessions
        writeln!(file, "% Sessions")?;
        let mut stmt = self.conn.prepare(
            "SELECT id, started_at, work_type, git_branch, commits_count, files_changed FROM sessions"
        )?;

        let sessions = stmt.query_map([], |row| {
            Ok(format!(
                "session('{}', '{}', {}, '{}', {}, {}).",
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                row.get::<_, i32>(4)?,
                row.get::<_, i32>(5)?,
            ))
        })?;

        for session in sessions {
            writeln!(file, "{}", session?)?;
        }
        writeln!(file)?;

        // Export patterns
        writeln!(file, "% Pattern observations")?;
        let mut stmt = self.conn.prepare(
            "SELECT session_id, pattern_name, category FROM patterns"
        )?;

        let patterns = stmt.query_map([], |row| {
            Ok(format!(
                "pattern_observed('{}', '{}', {}).",
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        for pattern in patterns {
            writeln!(file, "{}", pattern?)?;
        }

        // ... similar for technologies, decisions, challenges

        Ok(())
    }
}
```

## CLI Commands

```bash
# Extract facts from session markdown files
patina session extract [SESSION_FILE]
patina session extract --all              # Extract all 227 sessions

# Query facts database (SQL)
patina session query --sql "SELECT pattern_name, COUNT(*) FROM patterns GROUP BY pattern_name"

# Export to Prolog
patina session export-prolog              # Generate facts.pl from facts.db

# Query with Prolog
patina session query --prolog "recurring_pattern(P)"
patina session query --prolog "security_session(S)"

# Update domain bucket
patina session sync                       # Extract → DB → Prolog export
```

## Query Patterns

### SQL Queries (Fast Aggregation)

```sql
-- Most common patterns by category
SELECT category, pattern_name, COUNT(*) as occurrences
FROM patterns
GROUP BY category, pattern_name
ORDER BY occurrences DESC;

-- Security-focused sessions
SELECT s.id, s.started_at, COUNT(p.id) as security_patterns
FROM sessions s
JOIN patterns p ON s.id = p.session_id
WHERE p.category = 'security'
GROUP BY s.id
ORDER BY security_patterns DESC;

-- Technology adoption over time
SELECT tech_name,
       DATE(started_at) as date,
       COUNT(*) as usage_count
FROM technologies t
JOIN sessions s ON t.session_id = s.id
GROUP BY tech_name, DATE(started_at)
ORDER BY date DESC;

-- Pattern evolution timeline
SELECT pattern_name,
       MIN(started_at) as first_seen,
       MAX(started_at) as last_seen,
       COUNT(DISTINCT session_id) as session_count
FROM patterns p
JOIN sessions s ON p.session_id = s.id
GROUP BY pattern_name
HAVING session_count >= 3
ORDER BY session_count DESC;

-- Challenge-Solution pairs
SELECT c.problem, c.solution, COUNT(*) as frequency
FROM challenges c
GROUP BY c.problem, c.solution
ORDER BY frequency DESC;
```

### Prolog Queries (Logical Inference)

```prolog
% Pattern evolution (promote from surface → core)
?- pattern_observed(S1, Pattern, _),
   pattern_observed(S2, Pattern, _),
   pattern_observed(S3, Pattern, _),
   S1 \= S2, S2 \= S3, S1 \= S3.

% Security awareness (sessions addressing security)
?- session_focus(SessionId, security),
   session(SessionId, Date, Type, Branch, _, _),
   write(SessionId), write(' - '), write(Date), nl, fail.

% Technology stacks (what technologies are used together?)
?- tech_used(S, 'sqlite', _),
   tech_used(S, Prolog, _),
   sub_atom(Prolog, _, _, _, prolog).

% Decision patterns (philosophical vs pragmatic)
?- pragmatic_decision(S, Choice),
   decision(S, Choice, Rationale),
   write('Pragmatic: '), write(Choice), nl, fail.

% Workflow chains (sessions on same branch)
?- workflow_chain('20251008-061520', Related),
   session(Related, Date, Type, _, _, _),
   write(Related), write(' - '), write(Type), nl, fail.
```

## Integration with Existing Patina

### Existing Architecture

```
patina/
├── src/
│   └── commands/
│       ├── scrape/
│       │   └── code/
│       │       ├── database.rs    # ✅ Code facts (SQLite)
│       │       └── mod.rs
│       └── ask/
│           └── patterns.rs        # ✅ Pattern analysis (SQLite)
```

### New Architecture

```
patina/
├── src/
│   └── commands/
│       ├── scrape/
│       │   ├── code/
│       │   │   └── database.rs    # Code facts
│       │   └── session/           # 🆕 Session facts
│       │       ├── database.rs
│       │       ├── extractor.rs
│       │       └── prolog_export.rs
│       └── ask/
│           └── patterns.rs
└── layer/
    └── buckets/
        └── patina-dev/
            ├── facts.db           # SQLite database
            ├── facts.pl           # Exported Prolog facts
            ├── rules.pl           # Inference rules
            └── README.md
```

## Data Flow

### Extraction Pipeline

```
1. Session Markdown (227 files)
      ↓
2. LLM Extraction (structured JSON)
      ↓
3. facts.db (SQLite insert)
      ↓
4. facts.pl (Prolog export)
      ↓
5. Scryer Prolog (load + rules.pl)
      ↓
6. Queries & Insights
```

### Example: Adding New Session

```bash
# 1. Create session with /session-start
/session-start "implement prolog integration"

# 2. Work happens...
# 3. End session with /session-end

# 4. Extract facts from new session
patina session extract layer/sessions/20251010-123308.md

# 5. Sync to Prolog
patina session export-prolog

# 6. Query the new knowledge
scryer-prolog facts.pl rules.pl -g "session('20251010-123308', _, Type, _, _, _), write(Type), halt."
```

## Solving Cross-Domain Linking

### Current Problem

```prolog
% How do we link between domain buckets?
domain_link('patina-dev', 'rust-development', 'implements-patterns-from').
domain_link('patina-dev', 'security', 'applies-patterns-from').

% But how to query across domains?
?- rust_pattern(P), patina_uses_pattern(P).  % How to load rust-development facts?
```

### Proposed Solutions

#### Option 1: Shared SQLite Database

```
layer/buckets/
├── knowledge.db               # Single database for all domains
│   ├── sessions (patina-dev)
│   ├── patterns (patina-dev)
│   ├── rust_patterns
│   ├── security_patterns
│   └── domain_links
└── [domain]/
    ├── facts.pl → EXPORT FROM knowledge.db WHERE domain = '[domain]'
    └── rules.pl
```

**Pros**: Simple, efficient queries across domains
**Cons**: Tight coupling, domain buckets not self-contained

#### Option 2: Prolog Imports

```prolog
% patina-dev/rules.pl
:- consult('../rust-development/facts.pl').
:- consult('../security/facts.pl').

% Now can query across domains
cross_domain_pattern(Pattern) :-
    patina_pattern(Pattern),
    rust_pattern(Pattern).
```

**Pros**: Self-contained buckets, clear dependencies
**Cons**: Need to manage Prolog module namespacing

#### Option 3: REST API (Over-engineered?)

Each domain bucket exposes REST API for queries.

**Pros**: True microservices, language-agnostic
**Cons**: Complexity overkill for local knowledge base

#### **Recommended: Option 2 (Prolog Imports)**

Most aligned with Patina philosophy:
- Self-contained buckets ✅
- Clear dependencies ✅
- Simple implementation ✅
- Escape hatches (can still use SQL) ✅

## Next Steps

1. **Implement database.rs** - Session facts schema
2. **Build extractor** - LLM-based fact extraction
3. **Extract all 227 sessions** - Populate facts.db
4. **Prolog export** - facts.db → facts.pl automation
5. **Test cross-domain** - Create rust-development bucket, test imports
6. **Rust embedding** - Integrate Scryer Prolog via crate
7. **CLI commands** - `patina session extract/query/sync`

## Philosophy Alignment

✅ **Knowledge First**: Sessions → Facts → Insights
✅ **SQLite for Scale**: 227 sessions, fast queries
✅ **Prolog for Logic**: Pattern evolution, inference
✅ **Tool-based Design**: Clear input → output
✅ **Escape Hatches**: SQL AND Prolog queries
✅ **LLM Agnostic**: Extraction works with any LLM
