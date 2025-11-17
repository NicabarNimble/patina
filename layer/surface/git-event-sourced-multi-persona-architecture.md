---
id: git-event-sourced-multi-persona-architecture
version: 1
status: active
created_date: 2025-11-07
updated_date: 2025-11-07
oxidizer: nicabar
tags: [architecture, event-sourcing, multi-persona, git, collaboration, local-first]
related: [patina-llm-driven-neuro-symbolic-knowledge-system]
---

# Git-Based Event-Sourced Multi-Persona Architecture

## Overview

This document defines Patina's core data architecture, combining three powerful patterns:

1. **Event Sourcing**: Observations and beliefs are derived from immutable event logs
2. **Multi-Persona Collaboration**: Multiple contributors work on projects, each with their own personal knowledge system
3. **Git as Event Log**: Git serves as the distributed, auditable, reviewable event backbone

**Core Principle**: Projects are islands, personas are gods. Knowledge flows up (project → persona) through events. Knowledge flows down (persona → project) only through explicit requests.

---

## Motivation

### **Problems We're Solving**

1. **Collaborative Knowledge Formation**: How do multiple contributors build shared project knowledge without a central server?
2. **Personal vs Shared Knowledge**: How to maintain personal cross-project insights while contributing to team knowledge?
3. **Provenance and Auditability**: How to trace every belief to its evidence and know who contributed what?
4. **Local-First with Sync**: How to work offline while staying synchronized with the team?
5. **Schema Evolution**: How to change validation rules or data structures without breaking existing knowledge?

### **Why Event Sourcing?**

Observations are naturally events:
- "Added retry logic on Nov 3" is an event
- "Decided to use global state in blockchain context" is an event
- Events are immutable facts
- Current state (beliefs) can be derived from events

### **Why Git as Event Log?**

Traditional event sourcing uses central databases (Kafka, Event Store). We use Git because:
- ✅ **Distributed**: Every contributor has full history
- ✅ **Conflict Resolution**: Merge/rebase handles concurrent events
- ✅ **Auditable**: `git log`, `git blame` show full history
- ✅ **Reviewable**: PRs for all knowledge changes
- ✅ **Versioned**: Branches, tags, rollbacks
- ✅ **No Infrastructure**: Works offline, no server needed

### **Why Multi-Persona?**

Projects have multiple contributors, each with their own:
- Personal preferences
- Cross-project experiences
- Private observations
- Unique perspective

**Solution**: Separate personal knowledge (persona) from shared project knowledge.

---

## Core Concepts

### **1. Islands and Gods**

**Projects are Islands**:
- Self-contained repositories
- Own shared knowledge (committed to git)
- Own private workspaces (per contributor, not committed)
- Complete without personas (personas are optional enhancement)

**Personas are Gods**:
- Observe all project islands (read-only)
- Aggregate cross-project patterns
- Form personal beliefs
- Never impose on projects (no write access)

**Data Flow**:
```
Project Island → Events → Persona (god observes)
Persona → Knowledge → Project (only when requested)
```

### **2. Three Layers of Knowledge**

#### **Layer 1: Personal Persona** (`~/.patina/persona/`)

**Purpose**: Your cross-project knowledge accumulation

**Location**: User's home directory (private, never committed)

**Contents**:
- Personal beliefs derived from all projects
- Cross-project patterns
- Domain knowledge aggregated from multiple projects
- Personal preferences that differ from team norms

**Properties**:
- Reads from projects (one-way sync: project → persona)
- Never writes to projects
- Completely optional (projects work without it)

#### **Layer 2: Project Shared Knowledge** (`<project>/.patina/shared/`)

**Purpose**: Team's canonical knowledge for this project

**Location**: Inside git repository (committed, versioned)

**Contents**:
- Accepted decisions (team agreed)
- Identified patterns (validated by team)
- Shared observations (reviewed and merged)
- Domain declarations

**Properties**:
- Modified only through PRs (reviewed, auditable)
- Source of truth for project knowledge
- Shared across all contributors
- Materialized from event log

#### **Layer 3: Project Local Workspace** (`<project>/.patina/local/`)

**Purpose**: Contributor's private scratch space

**Location**: Inside git repository (`.gitignore`d, never committed)

**Contents**:
- Personal observations during work
- Session notes
- Failed experiments
- Draft ideas before proposing to team

**Properties**:
- Private to contributor
- Can query shared + local knowledge
- Can propose observations → shared (via PR)
- Syncs to personal persona

---

## Architecture Details

### **Directory Structure**

#### **Personal Persona** (`~/.patina/persona/`)

```
~/.patina/persona/
├── persona.db                       # Cross-project beliefs
├── event_log/                       # All events from all projects
│   ├── from-patina/
│   │   ├── local-events/            # Personal observations from patina
│   │   └── shared-events/           # Team knowledge from patina
│   ├── from-dust/
│   │   ├── local-events/
│   │   └── shared-events/
│   └── from-daydreams/
│       ├── local-events/
│       └── shared-events/
├── domains/                         # Domain knowledge packets
│   ├── rust.db                      # All Rust knowledge (from all projects)
│   │   ├── event_log                # Rust-specific events
│   │   └── patterns                 # Rust patterns across projects
│   ├── blockchain.db
│   └── agents.db
├── vectors/                         # Semantic search indices
│   ├── beliefs.usearch              # Persona-level beliefs
│   └── domains/
│       ├── rust.usearch
│       └── blockchain.usearch
└── projects.registry                # Track all known projects
```

#### **Project Repository** (`<project>/.patina/`)

```
<project>/.patina/
├── shared/                          # ✅ COMMITTED TO GIT
│   ├── events/                      # Canonical event log (git-versioned)
│   │   ├── 2025-11-07-001-observation-captured.json
│   │   ├── 2025-11-07-002-decision-made.json
│   │   ├── 2025-11-07-003-pattern-identified.json
│   │   └── ...
│   ├── project.shared.db            # Materialized from events/
│   ├── domains.manifest             # Declared domains (rust, ecs, etc.)
│   └── schema.version               # Schema/tooling version
│
├── local/                           # ❌ .gitignore'd (NEVER COMMITTED)
│   ├── event_log.db                 # Personal event log
│   ├── local.observations.db        # Materialized local state
│   ├── vectors/
│   │   └── local.observations.usearch
│   └── sync.state                   # Last event synced to persona
│
└── code.db                          # Code structure (separate concern)
```

### **Event Log as Files (Git-Versioned)**

Instead of database tables for events, **events are JSON files in git**:

```json
// .patina/shared/events/2025-11-07-042-pattern-identified.json
{
  "event_id": "evt_042",
  "event_type": "pattern_identified",
  "timestamp": "2025-11-07T12:34:56Z",
  "author": "contributor-a",
  "sequence": 42,
  "payload": {
    "pattern_name": "error_boundaries_in_react",
    "content": "We use error boundaries to isolate component failures",
    "rationale": "Prevents cascading errors, improves UX",
    "domains": ["react", "architecture"],
    "evidence": [
      "components/UserProfile/index.tsx",
      "components/Dashboard/index.tsx",
      "components/Settings/index.tsx"
    ],
    "observation_count": 15
  }
}
```

**Why JSON files instead of database**:
- ✅ Git can diff, merge, blame them
- ✅ Human-readable in PRs
- ✅ Easy to review (see exactly what changed)
- ✅ Mergeable (git handles conflicts)
- ✅ Portable (no database migrations for event log)

**Naming Convention**: `YYYY-MM-DD-NNN-{event-type}.json`
- Lexicographic ordering
- Unique per day (NNN is sequence)
- Event type in filename for readability

---

## Database Schemas

### **persona.db** (Personal Cross-Project Knowledge)

```sql
-- Cross-project beliefs
CREATE TABLE beliefs (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    confidence REAL NOT NULL,              -- 0.30-0.95
    formed_at TIMESTAMP NOT NULL,
    evidence_count INTEGER NOT NULL,
    source_projects TEXT NOT NULL,         -- JSON: ["patina", "dust"]
    domains TEXT NOT NULL,                 -- JSON: ["rust", "architecture"]
    last_validated TIMESTAMP
);

-- Evidence for beliefs (links to project events)
CREATE TABLE belief_evidence (
    belief_id TEXT NOT NULL,
    project_name TEXT NOT NULL,
    event_id TEXT NOT NULL,                -- References event in project
    observation_id TEXT NOT NULL,          -- Project-local observation ID
    relevance_score REAL NOT NULL,
    FOREIGN KEY (belief_id) REFERENCES beliefs(id)
);

-- Domains discovered across projects
CREATE TABLE domains (
    name TEXT PRIMARY KEY,
    description TEXT,
    created_at TIMESTAMP NOT NULL,
    belief_count INTEGER DEFAULT 0,
    observation_count INTEGER DEFAULT 0,
    project_count INTEGER DEFAULT 0
);

-- Domain-belief associations
CREATE TABLE domain_beliefs (
    domain_name TEXT NOT NULL,
    belief_id TEXT NOT NULL,
    relevance_score REAL NOT NULL,
    PRIMARY KEY (domain_name, belief_id),
    FOREIGN KEY (domain_name) REFERENCES domains(name),
    FOREIGN KEY (belief_id) REFERENCES beliefs(id)
);

-- Track materialization state
CREATE TABLE materialization_state (
    project_name TEXT NOT NULL,
    last_event_id TEXT NOT NULL,
    last_materialized_at TIMESTAMP NOT NULL,
    PRIMARY KEY (project_name)
);
```

### **projects.registry** (Track All Projects)

```sql
-- All projects persona knows about
CREATE TABLE projects (
    name TEXT PRIMARY KEY,
    path TEXT NOT NULL,                    -- Absolute path
    first_seen TIMESTAMP NOT NULL,
    last_scanned TIMESTAMP,
    observation_count INTEGER DEFAULT 0,
    domains TEXT NOT NULL,                 -- JSON: ["rust", "ecs"]
    git_remote TEXT                        -- For identifying same project on different machines
);

-- Scan history
CREATE TABLE scan_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_name TEXT NOT NULL,
    scanned_at TIMESTAMP NOT NULL,
    new_events_count INTEGER NOT NULL,
    beliefs_updated INTEGER NOT NULL,
    FOREIGN KEY (project_name) REFERENCES projects(name)
);
```

### **domains/{domain}.db** (Domain Knowledge Packet)

```sql
-- Observations about this domain from all projects
CREATE TABLE observations (
    id TEXT PRIMARY KEY,
    project_name TEXT NOT NULL,
    event_id TEXT NOT NULL,                -- References event in project
    content TEXT NOT NULL,
    observation_type TEXT NOT NULL,
    relevance_to_domain REAL NOT NULL,     -- 0.0-1.0
    source_type TEXT NOT NULL,             -- session, commit, code
    reliability REAL NOT NULL,
    created_at TIMESTAMP NOT NULL
);

-- Patterns that span multiple projects in this domain
CREATE TABLE patterns (
    id TEXT PRIMARY KEY,
    pattern_name TEXT NOT NULL,
    description TEXT,
    observation_count INTEGER NOT NULL,
    project_count INTEGER NOT NULL,        -- Seen in N projects
    confidence REAL NOT NULL,
    created_at TIMESTAMP NOT NULL
);

-- Evidence for patterns
CREATE TABLE pattern_evidence (
    pattern_id TEXT NOT NULL,
    observation_id TEXT NOT NULL,
    relevance_score REAL NOT NULL,
    PRIMARY KEY (pattern_id, observation_id),
    FOREIGN KEY (pattern_id) REFERENCES patterns(id),
    FOREIGN KEY (observation_id) REFERENCES observations(id)
);
```

### **project.shared.db** (Materialized Team Knowledge)

```sql
-- Materialized observations (rebuilt from events/)
CREATE TABLE observations (
    id TEXT PRIMARY KEY,                   -- Matches event_id
    content TEXT NOT NULL,
    observation_type TEXT NOT NULL,        -- pattern, decision, challenge
    domains TEXT NOT NULL,                 -- JSON: ["rust", "ecs"]
    source_type TEXT NOT NULL,             -- session, commit, manual
    reliability REAL NOT NULL,
    author TEXT NOT NULL,                  -- Git author
    created_at TIMESTAMP NOT NULL,
    event_file TEXT NOT NULL               -- Source event filename
);

-- Materialized patterns
CREATE TABLE patterns (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    rationale TEXT,
    domains TEXT NOT NULL,
    evidence TEXT,                         -- JSON: observation IDs
    author TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    event_file TEXT NOT NULL
);

-- Materialized decisions
CREATE TABLE decisions (
    id TEXT PRIMARY KEY,
    decision TEXT NOT NULL,
    rationale TEXT NOT NULL,
    alternatives TEXT,                     -- JSON: alternative approaches
    domains TEXT NOT NULL,
    author TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    event_file TEXT NOT NULL
);

-- Materialization metadata
CREATE TABLE materialization_state (
    last_event_file TEXT NOT NULL,
    last_materialized_at TIMESTAMP NOT NULL,
    event_count INTEGER NOT NULL
);
```

### **local.observations.db** (Personal Scratch Space)

```sql
-- Local event log (per-contributor)
CREATE TABLE event_log (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT UNIQUE NOT NULL,
    event_type TEXT NOT NULL,              -- observation_captured, session_started, etc.
    payload TEXT NOT NULL,                 -- JSON
    timestamp TIMESTAMP NOT NULL,
    proposed BOOLEAN DEFAULT FALSE,        -- Tracked if proposed to shared
    proposed_event_id TEXT                 -- Links to shared event if accepted
);

-- Materialized local observations
CREATE TABLE observations (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    observation_type TEXT NOT NULL,
    source_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    content_hash TEXT NOT NULL             -- For deduplication
);

-- Sync state (which shared events synced to persona)
CREATE TABLE sync_state (
    last_shared_event_file TEXT NOT NULL,
    last_synced_at TIMESTAMP NOT NULL
);
```

---

## Event Types and Lifecycle

### **Event Type Taxonomy**

#### **Observation Events** (Facts Captured)

```json
{
  "event_type": "observation_captured",
  "payload": {
    "content": "Added retry logic to RPC calls",
    "observation_type": "pattern",
    "source_type": "commit",
    "source_id": "abc123",
    "domains": ["rust", "blockchain"]
  }
}
```

#### **Decision Events** (Choices Made)

```json
{
  "event_type": "decision_made",
  "payload": {
    "decision": "Use global connection pool for blockchain RPC",
    "rationale": "Performance critical, acceptable tradeoff",
    "alternatives": ["Per-request connections", "Connection pooling per component"],
    "domains": ["blockchain", "architecture"]
  }
}
```

#### **Pattern Events** (Recurring Themes)

```json
{
  "event_type": "pattern_identified",
  "payload": {
    "pattern_name": "error_boundaries_in_react",
    "content": "We use error boundaries to isolate component failures",
    "rationale": "Prevents cascading errors",
    "evidence": ["obs_123", "obs_456", "obs_789"],
    "domains": ["react", "architecture"]
  }
}
```

#### **Belief Events** (Validated Knowledge)

```json
{
  "event_type": "belief_formed",
  "payload": {
    "belief": "I modularize when complexity grows",
    "confidence": 0.87,
    "evidence_count": 15,
    "evidence_observations": ["obs_1", "obs_5", "obs_12"],
    "domains": ["rust", "architecture"],
    "scope": "persona"
  }
}
```

### **Event Lifecycle: Local → Shared → Persona**

#### **Phase 1: Local Capture** (Private)

```bash
# Contributor working in project-x
cd project-x

# Observe pattern during work
patina observe "We always validate user input before DB writes"

# Event stored: .patina/local/event_log.db
{
  "event_type": "observation_captured",
  "payload": {
    "content": "We always validate user input before DB writes",
    "observation_type": "pattern",
    "source_type": "manual"
  },
  "scope": "local"
}
```

**State**: Only contributor knows about this observation.

#### **Phase 2: Propose to Shared** (PR Created)

```bash
# Contributor proposes to team
patina propose observation "We always validate user input before DB writes" \
  --type pattern \
  --domains security,validation \
  --rationale "Seen in 12 endpoints, prevents SQL injection"

# Generates file: .patina/shared/events/2025-11-07-043-pattern-identified.json
# Creates PR: "pattern: document input validation practice"
```

**State**: Team can review in PR, suggest changes to event payload.

#### **Phase 3: Merge to Shared** (Team Knowledge)

```bash
# PR approved and merged
# Event file now in main branch: .patina/shared/events/2025-11-07-043-pattern-identified.json

# All contributors pull
git pull

# Materialize new event
patina materialize

# Updates: .patina/shared/project.shared.db
INSERT INTO patterns (id, name, content, domains, author, created_at, event_file)
VALUES ('evt_043', 'input_validation', '...', '["security"]', 'contributor-a', NOW(), '2025-11-07-043-pattern-identified.json');
```

**State**: All contributors have this knowledge in `project.shared.db`.

#### **Phase 4: Sync to Persona** (Personal Knowledge)

```bash
# Contributor syncs to personal persona
patina persona sync

# Reads: project-x/.patina/shared/events/ (new events since last sync)
# Copies to: ~/.patina/persona/event_log/from-project-x/shared-events/
# Materializes: Updates persona cross-project beliefs

# If contributor also has input validation pattern in project-y
# Persona can now form cross-project belief:
# "I validate user input before DB writes" (evidence from 2 projects)
```

**State**: Personal persona has cross-project insight.

---

## Data Flow Diagrams

### **Flow 1: Observation Capture and Sharing**

```
┌─────────────────────────────────────────────────────────────┐
│ Contributor A Working in project-x                          │
└─────────────────────────────────────────────────────────────┘
                            ↓
                    [Work Happens]
                            ↓
                    Git Commit: "add retry logic"
                            ↓
                    Scraper Detects New Commit
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ .patina/local/event_log.db                                  │
│ INSERT: observation_captured(content="add retry logic")     │
└─────────────────────────────────────────────────────────────┘
                            ↓
                    Materialize Local
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ .patina/local/local.observations.db                         │
│ INSERT INTO observations (id, content, type, ...)           │
└─────────────────────────────────────────────────────────────┘
                            ↓
            [Contributor decides to share with team]
                            ↓
                    patina propose observation "add retry logic"
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Creates PR with:                                            │
│ .patina/shared/events/2025-11-07-042-observation.json      │
└─────────────────────────────────────────────────────────────┘
                            ↓
                    [Team Reviews PR]
                            ↓
                    [PR Merged to main]
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Git: .patina/shared/events/2025-11-07-042-observation.json │
│ (Now in repository, versioned)                              │
└─────────────────────────────────────────────────────────────┘
                            ↓
            All Contributors: git pull
                            ↓
                    patina materialize
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ .patina/shared/project.shared.db                            │
│ INSERT INTO observations (from event file)                  │
└─────────────────────────────────────────────────────────────┘
                            ↓
            Contributors: patina persona sync
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ ~/.patina/persona/event_log/from-project-x/shared-events/  │
│ Copy shared events for personal aggregation                 │
└─────────────────────────────────────────────────────────────┘
```

### **Flow 2: Cross-Project Belief Formation**

```
┌─────────────────────────────────────────────────────────────┐
│ Contributor A's Persona                                      │
│ ~/.patina/persona/                                           │
└─────────────────────────────────────────────────────────────┘
                            ↓
        Syncs events from multiple projects
                            ↓
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│ project-x    │  │ project-y    │  │ project-z    │
│ 15 obs about │  │ 8 obs about  │  │ 12 obs about │
│ input        │  │ input        │  │ validation   │
│ validation   │  │ validation   │  │              │
└──────────────┘  └──────────────┘  └──────────────┘
                            ↓
            Semantic Clustering (Neural Layer)
                            ↓
        "input validation" cluster (35 observations)
                            ↓
            Prolog Validation (Symbolic Layer)
                            ↓
        evidence_count(35) >= threshold(5) ✓
        project_count(3) >= min_projects(2) ✓
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Belief Formed (persona.db)                                  │
│                                                              │
│ belief: "I validate user input before DB writes"            │
│ confidence: 0.91                                             │
│ evidence_count: 35                                           │
│ source_projects: ["project-x", "project-y", "project-z"]    │
│ domains: ["security", "validation"]                         │
└─────────────────────────────────────────────────────────────┘
                            ↓
        Stored in ~/.patina/persona/persona.db
                            ↓
        Available when Contributor A works in ANY project
```

### **Flow 3: Project Requesting Domain Knowledge**

```
┌─────────────────────────────────────────────────────────────┐
│ Contributor B Working in new-project                        │
│ (New React project)                                          │
└─────────────────────────────────────────────────────────────┘
                            ↓
        Declares domain: patina domain add react
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ new-project/.patina/shared/domains.manifest                 │
│ { "domains": ["react"] }                                     │
└─────────────────────────────────────────────────────────────┘
                            ↓
        Query domain knowledge from persona
                            ↓
        patina query domain react --type pattern
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Persona reads: ~/.patina/domains/react.db                   │
│                                                              │
│ Finds patterns from all React projects:                     │
│ - error_boundaries_in_react (from project-x)                │
│ - hook_composition_pattern (from project-y)                 │
│ - context_for_global_state (from project-x, project-y)      │
└─────────────────────────────────────────────────────────────┘
                            ↓
        Returns to Contributor B
                            ↓
        Contributor B can:
        - View as reference (informational)
        - Adopt locally (copy to new-project beliefs)
        - Ignore (project is king!)
```

---

## Commands and Workflows

### **Project-Level Commands**

#### **Initialize Patina in Project**

```bash
patina init

# Creates:
# .patina/shared/
#   ├── events/
#   ├── project.shared.db (empty, will be materialized)
#   ├── domains.manifest
#   └── schema.version
# .patina/local/
#   ├── event_log.db
#   ├── local.observations.db
#   └── sync.state
# .gitignore: .patina/local/
```

#### **Capture Local Observation**

```bash
patina observe "We use error boundaries in React components"

# Stores in: .patina/local/event_log.db
# Event: observation_captured (scope: local)
# Materializes to: .patina/local/local.observations.db
```

#### **Query Knowledge** (Local + Shared)

```bash
patina query "error handling patterns"

# Searches:
# 1. .patina/local/local.observations.db (personal)
# 2. .patina/shared/project.shared.db (team)
# Returns: Combined results with source tags
```

#### **Propose Observation to Team**

```bash
patina propose observation "We use error boundaries in React components" \
  --type pattern \
  --domains react,architecture \
  --rationale "Prevents cascading failures, seen in 15 components"

# Generates: .patina/shared/events/2025-11-07-NNN-pattern-identified.json
# Creates git branch: add-error-boundary-pattern
# Opens PR with event file
```

#### **Materialize Shared Events**

```bash
patina materialize

# Reads: .patina/shared/events/*.json (in order)
# Rebuilds: .patina/shared/project.shared.db from scratch
# Use after: git pull (to incorporate new team knowledge)
```

#### **Sync to Personal Persona**

```bash
patina persona sync

# Reads: .patina/shared/events/ (new events since last sync)
# Copies to: ~/.patina/persona/event_log/from-{project}/
# Updates: sync.state (track last synced event)
# Materializes: Updates persona cross-project beliefs
```

### **Persona-Level Commands** (Outside Projects)

#### **Query Cross-Project Knowledge**

```bash
patina persona query "error handling patterns"

# Searches: ~/.patina/persona/persona.db
# Returns: Cross-project beliefs with evidence from multiple projects
```

#### **List Domains**

```bash
patina persona domains

# Output:
# rust (3 projects, 247 observations)
# react (2 projects, 89 observations)
# blockchain (1 project, 45 observations)
```

#### **Explain Belief**

```bash
patina persona explain "I use error boundaries in React"

# Output:
# Belief: I use error boundaries in React components
# Confidence: 0.87
# Evidence: 23 observations
# Projects: project-x (15), project-y (8)
# Domains: react, architecture
#
# Supporting Evidence:
# - project-x: "error boundaries in UserProfile" (session, 2025-10-15)
# - project-x: "error boundaries in Dashboard" (commit, 2025-10-20)
# - project-y: "added ErrorBoundary wrapper" (commit, 2025-11-01)
# [...]
```

#### **Query Domain Knowledge**

```bash
# In a project that uses "rust" domain
patina query domain rust --type pattern

# Queries: ~/.patina/domains/rust.db
# Returns: Patterns from ALL projects using Rust
# Shows: Which project each pattern came from
```

---

## Deduplication Strategy

### **Problem**

Multiple sources can generate similar observations:
- Git commit: "add retry logic"
- Session note: "added retry logic to RPC calls"
- Manual observation: "use retry for network calls"

Need to avoid storing duplicates while preserving provenance.

### **Recommended Approach: Hybrid**

#### **1. Content Hash (Semantic Deduplication)**

```sql
-- observations table includes content_hash
CREATE TABLE observations (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    source_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    UNIQUE(content_hash, source_id)         -- Same content from different sources OK
);
```

**Implementation**:
```rust
fn content_hash(content: &str) -> String {
    // Normalize: lowercase, trim, remove punctuation
    let normalized = content
        .to_lowercase()
        .trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>();

    // Hash
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

**Behavior**:
- "add retry logic" vs "Add retry logic." → Same hash
- "add retry logic" vs "added retry logic to RPC" → Different hash (good! more specific)
- Same observation from same source → Rejected (UNIQUE constraint)
- Same observation from different source → Allowed (different provenance)

#### **2. Source Tracking (Provenance)**

```sql
CREATE TABLE extraction_state (
    source_type TEXT NOT NULL,             -- 'session' or 'commit'
    source_id TEXT NOT NULL,               -- file path or commit hash
    source_mtime BIGINT,                   -- for files (sessions)
    extracted_at TIMESTAMP NOT NULL,
    observation_count INTEGER NOT NULL,
    PRIMARY KEY (source_type, source_id)
);
```

**Behavior**:
- **Git commits**: Tracked by commit hash (immutable)
  - If hash in extraction_state → skip
- **Session files**: Tracked by path + mtime
  - If path in extraction_state AND mtime unchanged → skip
  - If mtime changed → re-extract, UPDATE observations

#### **3. Update vs Insert Logic**

```sql
-- When re-extracting from updated source
INSERT INTO observations (id, content, content_hash, source_type, source_id, ...)
VALUES (?, ?, ?, ?, ?, ...)
ON CONFLICT(content_hash, source_id) DO UPDATE SET
    content = excluded.content,
    reliability = excluded.reliability,
    updated_at = CURRENT_TIMESTAMP;
```

**Decision Tree**:
- **Same content_hash + source_id**: UPDATE (refined but same observation)
- **Different content_hash**: INSERT (new observation)
- **Source deleted**: Mark `deleted = TRUE` (preserve history)

---

## Git Workflow Integration

### **Event Files as First-Class Citizens**

Events are JSON files committed to git, so standard git workflows apply:

#### **Creating Events via PR**

```bash
# Contributor creates event file
patina propose observation "..." \
  --type pattern \
  --domains rust

# Behind the scenes:
# 1. Generate event file: .patina/shared/events/2025-11-07-042-pattern.json
# 2. Create branch: add-pattern-042
# 3. Commit event file
# 4. Open PR

# Team reviews PR:
# - Can see event JSON in diff
# - Can request changes to payload
# - Can reject if inaccurate
```

#### **Merging Events**

```bash
# PR approved and merged
# Event file now in main branch

# All contributors pull
git pull

# Materialize new event
patina materialize

# Reads: .patina/shared/events/2025-11-07-042-pattern.json
# Inserts: project.shared.db patterns table
```

#### **Handling Conflicts**

**Scenario**: Two contributors propose events simultaneously

```bash
# Contributor A: Creates 2025-11-07-042-pattern.json
# Contributor B: Creates 2025-11-07-042-decision.json (same sequence!)

# First PR merged → sequence 042 taken
# Second PR gets conflict on filename

# Resolution:
# Rename to next sequence: 2025-11-07-043-decision.json
# Rebase and merge
```

**Automatic sequence resolution**:
```bash
patina propose --auto-sequence

# Checks .patina/shared/events/ for highest sequence number
# Generates next available sequence
```

#### **Reverting Events**

```bash
# Event was merged but team decides it's inaccurate
git revert <commit-hash>

# Removes event file from git
# Re-materialize
patina materialize

# Observation removed from project.shared.db
```

### **Git as Audit Trail**

Every event has full git history:

```bash
# Who created this pattern?
git log --follow .patina/shared/events/2025-11-07-042-pattern.json

# What changed in this event?
git diff abc123 .patina/shared/events/2025-11-07-042-pattern.json

# When was this decision made?
git log --format="%h %ai %s" -- .patina/shared/events/*decision*.json
```

---

## Multi-Persona Collaboration Scenarios

### **Scenario 1: Two Contributors, One Project**

**Setup**:
- Contributor A: Works on feature-x
- Contributor B: Works on feature-y
- Both work in project-x

**Workflow**:

```bash
# Contributor A observes pattern while working
cd project-x
patina observe "We validate input before DB writes"
# Stored locally in A's .patina/local/

# Contributor B independently observes same pattern
patina observe "Always validate user input"
# Stored locally in B's .patina/local/

# Contributor A proposes to team
patina propose observation "..." --type pattern
# Creates PR #123

# Contributor B sees PR, validates it matches their observation
# PR #123 merged

# Both contributors pull and materialize
git pull
patina materialize

# Now both have shared knowledge in project.shared.db
# Both sync to personal personas
patina persona sync

# Both personas have evidence from project-x
```

### **Scenario 2: One Contributor, Multiple Projects**

**Setup**:
- Contributor A works on: project-x (React), project-y (React), project-z (Vue)
- Notices error handling pattern in React projects

**Workflow**:

```bash
# In project-x
cd project-x
patina observe "Use error boundaries for component isolation"
patina propose observation "..." --type pattern
# Merged to project-x

# In project-y
cd project-y
patina observe "Error boundaries in critical paths"
patina propose observation "..." --type pattern
# Merged to project-y

# Persona syncs from both projects
patina persona sync

# Persona detects cross-project pattern
# Semantic clustering finds both observations
# Prolog validates: evidence from 2 projects
# Forms belief: "I use error boundaries in React"

# Later, in project-z (Vue, no error boundaries)
cd project-z
patina query "error handling patterns"

# Query returns:
# - Local project-z observations (Vue patterns)
# - Persona can suggest (but not impose): "In React I use error boundaries"
```

### **Scenario 3: Multiple Contributors, Conflicting Observations**

**Setup**:
- Contributor A: "We always use TypeScript"
- Contributor B: "We use JavaScript for rapid prototyping"

**Workflow**:

```bash
# Contributor A proposes
patina propose observation "We always use TypeScript" \
  --type decision

# Creates PR #101

# Contributor B sees PR, disagrees
# Comments: "We use JS for prototypes, TS for production"

# Resolution: Refine event payload
{
  "event_type": "decision_made",
  "payload": {
    "decision": "We use TypeScript for production code",
    "rationale": "Type safety reduces bugs",
    "context": "Production code only",
    "alternatives": ["JavaScript for rapid prototyping"]
  }
}

# Contributor B proposes complementary decision
patina propose observation "We use JavaScript for prototyping" \
  --type decision \
  --context "Rapid prototyping phase only"

# Creates PR #102

# Both PRs merged
# Project now has TWO decisions, each with context
# No conflict: both are true in their contexts
```

---

## Materialization Process

### **What is Materialization?**

**Definition**: Converting immutable event log into queryable relational state.

**Analogy**: Redux reducer turning actions into state tree.

**In Patina**: Event files (JSON) → Database tables (SQLite)

### **Materialization Triggers**

**When to materialize**:
1. After `git pull` (new shared events)
2. After local observations captured (local events)
3. After persona sync (cross-project events)
4. On-demand: `patina materialize`

### **Materialization Algorithm**

```rust
fn materialize_shared_events(db_path: &Path, events_dir: &Path) -> Result<()> {
    let conn = Connection::open(db_path)?;

    // Get last materialized event
    let last_event = conn.query_row(
        "SELECT last_event_file FROM materialization_state",
        [],
        |row| row.get::<_, String>(0)
    ).ok();

    // Read all event files in order
    let mut event_files: Vec<_> = fs::read_dir(events_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension() == Some("json"))
        .collect();

    event_files.sort(); // Lexicographic order

    // Process events after last materialized
    let start_index = last_event.as_ref()
        .and_then(|last| event_files.iter().position(|f| f.file_name() == Some(last)))
        .map(|i| i + 1)
        .unwrap_or(0);

    for event_file in &event_files[start_index..] {
        // Parse event
        let event: Event = serde_json::from_str(&fs::read_to_string(event_file)?)?;

        // Materialize based on event type
        match event.event_type.as_str() {
            "observation_captured" => {
                conn.execute(
                    "INSERT INTO observations (id, content, type, domains, author, created_at, event_file)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        event.event_id,
                        event.payload["content"],
                        event.payload["observation_type"],
                        event.payload["domains"],
                        event.author,
                        event.timestamp,
                        event_file.file_name().unwrap().to_str()
                    ]
                )?;
            }
            "pattern_identified" => {
                conn.execute(
                    "INSERT INTO patterns (id, name, content, rationale, domains, evidence, author, created_at, event_file)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        event.event_id,
                        event.payload["pattern_name"],
                        event.payload["content"],
                        event.payload["rationale"],
                        event.payload["domains"],
                        event.payload["evidence"],
                        event.author,
                        event.timestamp,
                        event_file.file_name().unwrap().to_str()
                    ]
                )?;
            }
            _ => {
                // Unknown event type, skip or log warning
            }
        }

        // Update materialization state
        conn.execute(
            "UPDATE materialization_state SET last_event_file = ?1, last_materialized_at = ?2",
            params![event_file.file_name().unwrap().to_str(), chrono::Utc::now()]
        )?;
    }

    Ok(())
}
```

### **Incremental vs Full Materialization**

**Incremental** (Default):
- Read events after last materialized
- Fast (only new events)
- Preserves materialization_state

**Full** (Force Rebuild):
```bash
patina materialize --force

# Algorithm:
# 1. DROP all tables except materialization_state
# 2. Recreate tables
# 3. Process ALL events from beginning
# 4. Useful after schema changes
```

---

## Integration with Neuro-Symbolic System

### **How Event Sourcing Enhances Neuro-Symbolic Architecture**

**Prolog as Materializer**:
- Events (observations) → Prolog validation → Materialized beliefs
- Prolog rules determine which events form valid beliefs
- Can change rules and re-materialize (replay events)

**Semantic Search on Materialized State**:
- Vectors generated from materialized observations (not events)
- Event log provides provenance for every vector
- Can rebuild indices from events if needed

**Belief Formation Pipeline**:
```
Event Log (immutable observations)
    ↓
Materialize → observations.db
    ↓
Generate Embeddings → observations.usearch
    ↓
Semantic Clustering (neural layer)
    ↓
Prolog Validation (symbolic layer)
    ↓
Belief Events (belief_formed)
    ↓
Materialize → beliefs.db
```

### **Time Travel and Debugging**

**Query historical state**:
```sql
-- Rebuild state as of Nov 1, 2025
SELECT * FROM events WHERE timestamp <= '2025-11-01'
-- Materialize only these events
-- See what beliefs existed on that date
```

**Trace belief evolution**:
```sql
-- How did "I modularize when complex" belief form?
SELECT e.*
FROM events e
JOIN belief_evidence be ON e.event_id = be.event_id
WHERE be.belief_id = 'belief_123'
ORDER BY e.timestamp;
```

**A/B test validation rules**:
```rust
// Try different Prolog rules
prolog_engine.load_rules("confidence-rules-v1.pl");
let beliefs_v1 = materialize_beliefs(events)?;

prolog_engine.load_rules("confidence-rules-v2.pl");
let beliefs_v2 = materialize_beliefs(events)?;

// Compare: which rules produce better beliefs?
```

---

## Benefits Summary

| Benefit | How Achieved |
|---------|--------------|
| **Provenance** | Every observation traces to git commit + author |
| **Auditability** | `git log` shows full event history |
| **Reviewability** | PRs for all knowledge changes (team validation) |
| **Time Travel** | Replay events to any point in time |
| **Schema Flexibility** | Change validation rules → replay events → new beliefs |
| **Collaboration** | Multiple contributors, clear shared vs private boundary |
| **Distributed** | Git handles sync, no central server |
| **Offline-First** | Work locally, sync later via git push/pull |
| **Conflict Resolution** | Git merge handles concurrent events |
| **Replayability** | Rebuild materialized state from scratch |

---

## Migration Path

### **For Existing Patina Installations**

**Phase 1: Add Event Log** (Non-Breaking)
- Keep existing observations.db
- Add event_log.db
- Write observations to BOTH (dual-write)
- Backfill events from existing observations

**Phase 2: Split Shared/Local** (Breaking)
- Create .patina/shared/ and .patina/local/
- Move team knowledge → shared/
- Move personal notes → local/
- Update .gitignore

**Phase 3: Events as Files** (Breaking)
- Convert event_log.db → JSON files
- Commit to git
- Remove event_log.db
- Materialize from JSON files

**Phase 4: Multi-Persona** (Additive)
- Add persona.db structure
- Add sync command
- Personas are optional (projects work without them)

---

## Open Questions for Team

1. **Event Versioning**: How to handle event schema changes?
   - Add `event_schema_version` field?
   - Separate directories per version?

2. **Event Retention**: Keep all events forever or prune?
   - Compress old events (gzip)?
   - Archive after N days?
   - Keep recent + "important" events only?

3. **Materialization Performance**: When projects grow large?
   - Incremental materialization should be fast
   - Full rebuild on schema change (acceptable?)
   - Index event files for faster lookup?

4. **Persona Discovery**: How does persona find projects?
   - Manual registration: `patina persona register <project-path>`
   - Auto-discover: Scan filesystem for .patina/ directories?
   - Config file: ~/.patina/projects.toml?

5. **Domain Packet Sync**: Should domains auto-sync or manual?
   - Auto: Every persona sync updates domain packets
   - Manual: `patina persona sync-domains`
   - Hybrid: Auto for declared domains, manual for discovery?

6. **Conflict Resolution Policy**: What if two contributors propose conflicting patterns?
   - PR review catches conflicts (human judgment)
   - Both patterns can be true in different contexts
   - Prolog rules detect contradictions?

---

## Next Steps

1. **Review this architecture** with team
2. **Answer open questions** above
3. **Create implementation plan** (break into phases)
4. **Design database migrations** (existing → new schema)
5. **Implement Phase 1**: Event log + materialization
6. **Build out multi-persona** support
7. **Integrate with neuro-symbolic** system (Prolog + embeddings)

---

## Related Documents

- [patina-llm-driven-neuro-symbolic-knowledge-system.md](./patina-llm-driven-neuro-symbolic-knowledge-system.md) - Overall vision and goals
- [neuro-symbolic-knowledge-system.md](./neuro-symbolic-knowledge-system.md) - Current implementation status
- [persona-belief-architecture.md](./persona-belief-architecture.md) - Belief formation system

---

## References

- **LiveStore**: Johannes Schickling's event-sourced SQLite for local-first apps
- **Event Sourcing**: Martin Fowler, Greg Young (CQRS/ES patterns)
- **Git as Database**: Fossil SCM, Git-based CMS systems
- **Multi-Persona Collaboration**: Distributed knowledge management, federated learning
