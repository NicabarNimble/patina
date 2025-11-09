---
id: patina-llm-driven-neuro-symbolic-knowledge-system
version: 3
status: active
created_date: 2025-11-07
updated_date: 2025-11-08
oxidizer: nicabar
tags: [architecture, neuro-symbolic, persona, event-sourcing, llm-integration, input-architecture, emergent-domains]
---

# Patina: LLM-Driven Neuro-Symbolic Knowledge System

## The Vision

**What We're Building**: A framework where AI observes work, records observations, reasons with the user to validate beliefs, and stores knowledge optimized for AI retrieval. The result: any LLM can load a persona and become indistinguishable from that identity.

**Core Principle**: The persona is permanent, the LLM is ephemeral. Claude today, Gemini tomorrow, GPT-5 next week - all become the persona by loading its beliefs, rules, and facts.

**Local-First**: Mac-centric small models (embeddings, neuro-symbolic validation), zero cloud dependency, full privacy.

---

## Document Structure & Review Guide

This document is organized into 10 focused topics for detailed review:

### Deep Dive Topics (Require Thorough Review)

**Topic 2: Event Sourcing Foundation** (~116 lines)
- Event structure, JSON schema, naming conventions
- Materialization algorithm and process
- Core architectural decision

**Topic 3: Domains as Emergent Tags** (~88 lines)
- Domain tagging strategy via LLM
- Schema design (domains, relationships, extraction tracking)
- Co-occurrence detection during oxidize

**Topic 7: Phase 1 Implementation** (~179 lines)
- 4-week breakdown (1A through 1D)
- Concrete tasks and success criteria
- Timeline and dependencies

**Topic 5: Persona & Project Architecture** (~56 lines)
- Storage locations and structure
- Shared/local split rationale
- Knowledge flow between persona and projects

### Medium Dive Topics (Important, Need Review)

**Topic 1: Vision & Core Architecture** (~50 lines)
- Framing: persona is permanent, LLM is ephemeral
- The Four Parts (Input, Storage, Validation, Loading)

**Topic 4: Neuro-Symbolic Reasoning** (~98 lines)
- Neural + Symbolic integration
- Prolog validation rules (already working)

**Topic 6: Current → Target State** (~76 lines)
- Migration path from current structure
- What's preserved vs rebuilt

### Light Review Topics (Check for Clarity)

**Topic 8: Success Metrics & Quality** (~36 lines)
- How we measure "Phase 1 complete"
- Data, command, structure, quality metrics

**Topic 10: Design Principles & Decisions** (~78 lines)
- Philosophical foundations
- Decisions made vs pending review

**Topic 9: Future Phases Summary** (~40 lines)
- Post-Phase 1 roadmap (summaries only)

### Review Status

- [ ] Topic 1: Vision & Core Architecture
- [ ] Topic 2: Event Sourcing Foundation
- [ ] Topic 3: Domains as Emergent Tags
- [ ] Topic 4: Neuro-Symbolic Reasoning
- [ ] Topic 5: Persona & Project Architecture
- [ ] Topic 6: Current → Target State
- [ ] Topic 7: Phase 1 Implementation
- [ ] Topic 8: Success Metrics & Quality
- [ ] Topic 9: Future Phases Summary
- [ ] Topic 10: Design Principles & Decisions

**Recommended Review Order**: Topics 2, 3, 7, 5, 1, 4, 6, 8, 10, 9

---

## Core Architecture

### The Four Parts

**1. Input** - Capture observations from explicit trigger points
- Session notes (`/session-note`, `/session-update`)
- Git commits (decisions, patterns, challenges)
- Manual observations (`patina observe`)
- **Not real-time watching** - git-like batch processing

**2. Storage** - Event-sourced with domains as emergent tags
- Observations are immutable events (JSON files in git)
- Databases are materialized views (rebuilt from events)
- Domains are tags, not databases (auto-tagged during scrape)
- Time travel: replay events to any point

**3. Validation** - Neuro-symbolic reasoning
- Neural: Semantic search finds patterns (USearch + Mac embeddings)
- Symbolic: Prolog validates evidence (Scryer embedded in Rust)
- User: Final validation (AI proposes, user decides)
- Result: Beliefs with full provenance

**4. Loading** - LLM adapters load persona
- `patina init --llm=claude` or `--llm=gemini`
- Adapter provides persona context to LLM
- LLM performs as that identity
- Framework is LLM-agnostic

**Current Focus**: #1 (Input) and #2 (Storage) = Phase 1

---

## Event Sourcing Foundation

### Why Events

**Observations are immutable facts**:
- "Added retry logic on Nov 3" is an event
- "Decided to use global state" is an event
- Events never change, beliefs derived from them evolve

**Benefits**:
- **Time travel**: Replay events to any point in time
- **Schema evolution**: Change validation rules, re-materialize beliefs
- **Auditability**: Git log shows full history of observations
- **Conflict resolution**: Git handles merge conflicts naturally
- **Replayability**: Rebuild entire persona from events

### Event Flow

```
┌─────────────────────────┐
│ Work Happens            │
│ (git commits, sessions) │
└────────────┬────────────┘
             ↓
┌─────────────────────────┐
│ patina scrape           │
│ sessions/git            │
└────────────┬────────────┘
             ↓
┌─────────────────────────┐
│ Event Files             │
│ .patina/shared/events/  │
│ *.json                  │
└────────────┬────────────┘
             ↓
┌─────────────────────────┐
│ patina materialize      │
│ (rebuild from events)   │
└────────────┬────────────┘
             ↓
┌─────────────────────────┐
│ observations.db         │
│ (queryable SQLite)      │
└────────────┬────────────┘
             ↓
┌─────────────────────────┐
│ patina oxidize          │
│ (generate vectors)      │
└────────────┬────────────┘
             ↓
┌─────────────────────────┐
│ vectors/                │
│ (usearch indices)       │
└────────────┬────────────┘
             ↓
┌─────────────────────────┐
│ patina query            │
│ (neural + symbolic)     │
└─────────────────────────┘
```

### Event File Format

**Location**: `.patina/shared/events/YYYY-MM-DD-NNN-type.json`

**Example**:
```json
{
  "event_id": "evt_042",
  "event_type": "observation_captured",
  "timestamp": "2025-11-08T12:34:56Z",
  "author": "nicabar",
  "sequence": 42,
  "payload": {
    "content": "Extracted environment detection to module when complexity grew",
    "observation_type": "pattern",
    "source_type": "session",
    "source_id": "20251107-124740",
    "domains": ["rust", "modularity", "architecture"],
    "reliability": 0.85
  }
}
```

**Properties**:
- Immutable (never edited)
- Committed to git (reviewable, auditable)
- Lexicographically ordered (YYYY-MM-DD-NNN)
- Human-readable (git diff shows what changed)

### Materialization

**What**: Converting event log into queryable database

**When**:
- After scrape (new events created)
- After git pull (new events from remote)
- On-demand: `patina materialize --force` (full rebuild)

**How**:
```rust
// Read events in order
for event in events_dir/*.json {
    match event.event_type {
        "observation_captured" => {
            db.execute("INSERT INTO observations (...) VALUES (...)")
        }
        "belief_formed" => {
            db.execute("INSERT INTO beliefs (...) VALUES (...)")
        }
        // ... handle other event types
    }
}
```

**Result**: observations.db is always derived from events, never canonical source

---

## Domains as Emergent Tags

### Core Principle

**Don't organize domains - let them organize themselves.**

Domains are infinite and overlapping: rust, typescript, modularity, bevy, ecs, narrative, blockchain, error-handling. They form graphs, not hierarchies. They emerge from actual work, not upfront design.

### Schema

```sql
-- Observations have domain tags (JSON array)
CREATE TABLE observations (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    observation_type TEXT NOT NULL,  -- pattern, decision, challenge
    domains TEXT,                    -- JSON: ["rust", "modularity", "architecture"]
    source_type TEXT NOT NULL,       -- session, commit, manual
    source_id TEXT NOT NULL,
    reliability REAL NOT NULL,       -- 0.0-1.0
    created_at TIMESTAMP NOT NULL,
    event_file TEXT NOT NULL         -- Source event filename
);

-- Domain catalog (auto-populated)
CREATE TABLE domains (
    name TEXT PRIMARY KEY,
    first_seen TIMESTAMP NOT NULL,
    observation_count INTEGER DEFAULT 0,
    project_count INTEGER DEFAULT 0
);

-- Domain relationships (discovered during oxidize)
CREATE TABLE domain_relationships (
    domain_a TEXT NOT NULL,
    domain_b TEXT NOT NULL,
    relationship_type TEXT NOT NULL,  -- 'co_occurs_with', 'subset_of'
    strength REAL NOT NULL,           -- 0.0-1.0
    discovered_at TIMESTAMP NOT NULL,
    PRIMARY KEY (domain_a, domain_b, relationship_type)
);

-- Extraction tracking (avoid re-scraping)
CREATE TABLE extraction_state (
    source_type TEXT NOT NULL,        -- 'session' or 'commit'
    source_id TEXT NOT NULL,          -- file path or commit hash
    source_mtime BIGINT,              -- for files (sessions)
    extracted_at TIMESTAMP NOT NULL,
    observation_count INTEGER NOT NULL,
    PRIMARY KEY (source_type, source_id)
);
```

### Auto-Tagging During Scrape

**When**: `patina scrape sessions` or `patina scrape git`

**Process**:
1. Extract observation: "Extracted environment detection to module"
2. LLM analyzes content + project context (using driving LLM: Claude/Gemini adapter)
3. Auto-tags domains: `["rust", "modularity", "architecture"]`
4. Creates event file with domain tags
5. Updates domain catalog (increment observation_count)

**No user action needed** - domains emerge automatically.

### Relationship Discovery During Oxidize

**When**: `patina oxidize`

**Process**:
1. Generate embeddings for all observations
2. Semantic clustering finds related observations
3. Analyze clusters for domain co-occurrence:
   - "bevy" + "rust" + "ecs" appear together 99% → co_occurs_with relationship
   - "modularity" spans software/writing/games → universal pattern
4. Calculate co-occurrence strength (0.0-1.0)
5. Store in domain_relationships table

**Examples**:
```sql
-- Bevy almost always uses Rust
INSERT INTO domain_relationships (domain_a, domain_b, relationship_type, strength)
VALUES ('bevy', 'rust', 'co_occurs_with', 0.99);

-- Modularity is universal (appears across domains)
INSERT INTO domain_relationships (domain_a, domain_b, relationship_type, strength)
VALUES ('modularity', 'universal_pattern', 'is_type', 1.0);
```

---

## Neuro-Symbolic Reasoning

### Why Both Neural AND Symbolic?

**Neural Alone**:
- ✅ Discovers patterns through fuzzy similarity
- ❌ Can hallucinate, no explainability

**Symbolic Alone**:
- ✅ Rigorous validation, full provenance
- ❌ Brittle, requires exact matches

**Together**:
- Neural proposes ("these 15 observations look related")
- Symbolic validates ("do they meet evidence threshold?")
- User decides ("yes, form belief" or "no, not yet")
- Result: Beliefs you can trust

### Neural Layer: USearch + Mac Embeddings

**Technology**:
- USearch for fast HNSW vector similarity
- all-MiniLM-L6-v2 via ONNX Runtime (INT8 quantized, 23MB)
- Metal GPU acceleration (on-device, no cloud)

**Use Cases**:
- Semantic search: "Find observations about retry logic" (fuzzy matching)
- Pattern discovery: Cluster similar observations
- Cross-domain similarity: "modularity in Rust" similar to "modularity in writing"
- Graceful uncertainty: Cosine similarity scores (0.0-1.0)

**Example**:
```rust
// Query: "error handling patterns"
let query_embedding = embeddings.encode("error handling patterns")?;
let results = storage.search_with_scores(&query_embedding, 20)?;

// Results:
// [
//   ("Use Result<> for recoverable errors", similarity=0.89),
//   ("Panic for invariant violations", similarity=0.82),
//   ("Log errors with context", similarity=0.78),
//   ...
// ]
```

### Symbolic Layer: Scryer Prolog

**Technology**:
- Scryer Prolog embedded in Rust (zero shell overhead)
- Dynamic fact injection (observations from SQLite → Prolog facts)
- Validation rules in `.patina/validation-rules.pl`

**Responsibilities**:

**A. Evidence Validation**
```prolog
% Does observation have enough support to become belief?
valid_belief(Observation) :-
    evidence_count(Observation, Count),
    Count >= 3,
    cross_project_evidence(Observation, Projects),
    length(Projects, PCount),
    PCount >= 2.
```

**B. Confidence Calculation**
```prolog
% Weighted evidence scoring
weighted_evidence_score(Score) :-
    findall(Weight, (
        observation(_, _, _, Similarity, Reliability, _),
        Weight is Similarity * Reliability
    ), Weights),
    sum_list(Weights, Score).

% Adequate evidence check
adequate_evidence(Score, StrongCount) :-
    Score >= 3.0,
    StrongCount >= 2.
```

**C. Contradiction Detection**
```prolog
% Are beliefs contradictory or contextually valid?
not_contradiction(B1, B2) :-
    belief(persona, B1),
    belief(Project, B2),
    opposite(B1, B2),
    valid_in_context(B2, Project).

% Example: global state OK in blockchain contexts
valid_in_context(use_global_state, Project) :-
    blockchain_project(Project),
    performance_critical(Project).
```

**Why Prolog**: Provides explainable, traceable reasoning that LLMs cannot hallucinate away. Every belief can answer "why do I believe this?"

---

## Persona and Project Architecture

### Persona (Cross-Project Identity)

**Location**: `~/.patina/persona/`

**Purpose**: Accumulated beliefs across all projects. This is the identity an LLM loads to perform as that persona.

**Structure**:
```
~/.patina/persona/
├── persona.db           # Cross-project beliefs
├── rules.pl             # Validation rules (epistemology)
├── vectors/             # Semantic search indices
└── projects.registry    # Track all known projects
```

**Properties**:
- Observes all projects (read-only)
- Forms beliefs when patterns appear across multiple projects
- Never imposes on projects (projects are king)
- Any persona: user identity, project philosophy, Einstein, team coding style

### Project (Local Knowledge)

**Location**: `<project>/.patina/`

**Purpose**: Project-specific knowledge. Project is king in its domain.

**Structure** (Event-Sourced):
```
<project>/.patina/
├── shared/              # Team knowledge (committed to git)
│   ├── events/          # Immutable event log (JSON files)
│   └── project.db       # Materialized from events
├── local/               # Personal workspace (gitignored)
│   ├── observations.db  # Scratch space, draft observations
│   └── vectors/         # Local usearch indices
├── code.db              # Code structure (separate concern)
└── persona.link         # Reference to ~/.patina/persona/
```

**Properties**:
- Can have beliefs that differ from persona (contextual, not contradictory)
- Example: Persona avoids global state BUT project uses it for blockchain performance
- Contributes evidence back to persona via sync
- Solo developer: shared/ is your official knowledge, local/ is scratch
- Team: shared/ reviewed via PRs, local/ is private

### Knowledge Flow

```
Project Work → Events → Project DB → Persona Sync → Persona Beliefs
```

**Persona Awareness**: Queries across projects to discover cross-project patterns without imposing beliefs.

---

## Current State vs Target State

### What We Have Now (Working)

✅ **Phase 2.7 Complete**: Embedded Scryer Prolog integration
- ReasoningEngine with dynamic fact injection
- `patina belief validate` CLI command
- 94 tests passing

✅ **USearch + Mac Embeddings**: INT8 quantized (23MB)
- Semantic search working
- Metal GPU acceleration

✅ **Session Tracking**: `.claude/` adapter
- `/session-start`, `/session-update`, `/session-end` commands
- Session markdown format (Obsidian-compatible)

✅ **Existing Data**:
- 463 observations in observations.db
- 7 of 266 sessions manually extracted
- code.db for code structure

### Current Structure

```
.patina/
├── db/
│   ├── observations.db     # 463 observations (flat, no events)
│   └── facts.db            # Manual observations (legacy)
├── storage/
│   └── observations/       # USearch indices
└── validation-rules.pl     # Prolog rules
```

### Target State (Phase 1 Complete)

```
.patina/
├── shared/                 # Git-committed
│   ├── events/             # ~266 event files (sessions + git)
│   ├── project.db          # Materialized from events
│   └── vectors/            # USearch indices
├── local/                  # Gitignored
│   ├── observations.db     # Scratch space
│   └── vectors/
├── code.db                 # Code structure (unchanged)
└── persona.link

~/.patina/persona/          # Future Phase 2
├── persona.db
├── rules.pl
└── projects.registry
```

### What Needs to Change

**1. Event Sourcing**:
- ❌ Now: Direct writes to observations.db
- ✅ Target: Scrape creates event files → materialize builds DB

**2. Domains as Tags**:
- ❌ Now: No domains field
- ✅ Target: Auto-tagged during scrape, relationships discovered during oxidize

**3. Scrape Architecture**:
- ❌ Now: `patina embeddings generate` does extraction + vectorization together
- ✅ Target: `patina scrape` (extraction) → `patina oxidize` (vectorization) separate

**4. Session Extraction**:
- ❌ Now: 7 of 266 sessions manually extracted
- ✅ Target: Parser reads session markdown → creates event files (all 266 sessions)

**5. Shared/Local Split**:
- ❌ Now: Single `.patina/db/observations.db`
- ✅ Target: `.patina/shared/` (committed) + `.patina/local/` (gitignored)

---

## Phase 1 Implementation

### Overview

**Goal**: Build input architecture with event sourcing and domains as tags from the start.

**Timeline**: 4 weeks (4 sub-phases)

**Outcome**: All sessions + git commits extracted as events, materialized into observations.db with domain tags, ready for neuro-symbolic querying.

### Phase 1A: Event Foundation (Week 1)

**Tasks**:
- [ ] Design event file JSON schema
  - Event types: observation_captured, pattern_identified, decision_made, belief_formed
  - Payload structure with domains, source_type, reliability
  - Naming convention: YYYY-MM-DD-NNN-type.json
- [ ] Implement `patina materialize` command
  - Read events from `.patina/shared/events/` in order
  - Build observations.db, domains, domain_relationships tables
  - Track last materialized event
  - Support `--force` for full rebuild
- [ ] Backup existing data
  - Export 463 observations from current observations.db
  - Save as `.patina/db/observations.db.backup`
  - Save facts.db as backup
- [ ] Start fresh with event-sourced structure
  - Create `.patina/shared/events/` directory
  - Create `.patina/local/` directory
  - Initialize new schema with domains support
- [ ] Test materialize with sample events
  - Create 10 test event files
  - Run materialize, verify observations.db
  - Run materialize --force, verify full rebuild

**Success Criteria**:
- ✅ Event file format documented
- ✅ Materialize command working (incremental + full rebuild)
- ✅ Existing data backed up
- ✅ Fresh structure with domains schema

**Code References**:
- `src/commands/materialize/mod.rs` (new command)
- `src/storage/events.rs` (event file handling)
- `src/storage/observations.rs` (update schema for domains)

### Phase 1B: Session Scraping (Week 2)

**Tasks**:
- [ ] Design session markdown parser
  - Read Obsidian-compatible markdown format
  - Extract observations from session activity logs
  - Support current session format (Activity Log sections)
  - Adapt format as needed for system requirements
- [ ] Implement `patina scrape sessions` command
  - Parse layer/sessions/*.md files
  - Extract patterns, decisions, challenges from activity logs
  - LLM auto-tags domains (using Claude/Gemini adapter)
  - Create event files in .patina/shared/events/
  - Track in extraction_state table (avoid re-scraping)
- [ ] Extract all 266 sessions
  - Run scrape on layer/sessions/
  - Generate ~266 event files (some sessions may have multiple observations)
  - Verify event file quality (spot check 10 sessions)
- [ ] Test materialize with session events
  - Run materialize after scrape
  - Verify observations.db populated correctly
  - Check domain tags applied
- [ ] Handle session updates
  - Detect changed sessions (mtime tracking)
  - Re-extract if modified (UPDATE existing events)
  - Support `--force` to re-scrape all

**Success Criteria**:
- ✅ Session parser handles current markdown format
- ✅ All 266 sessions extracted as events
- ✅ Domain tags auto-applied during scrape
- ✅ Extraction tracking prevents duplicate scraping
- ✅ Modified sessions can be re-scraped

**Code References**:
- `src/commands/scrape/sessions.rs` (new)
- `src/parsers/session_markdown.rs` (new)
- `src/adapters/claude/domain_tagger.rs` (use existing LLM adapter)

### Phase 1C: Git Scraping (Week 3)

**Tasks**:
- [ ] Implement `patina scrape git` command
  - Extract commit messages (all history, not just 90 days)
  - Parse conventional commit format (feat:, fix:, refactor:, etc.)
  - Map to observation types (feat:/fix: → decision, refactor: → pattern, etc.)
  - LLM auto-tags domains
  - Create event files
  - Track in extraction_state (commit hash)
- [ ] Deduplication strategy
  - Content hash for semantic deduplication
  - Source tracking (same content from different sources OK)
  - UNIQUE constraint on (content_hash, source_id)
- [ ] Extract git history
  - Run scrape on all git commits
  - Generate event files
  - Verify no duplicates with session events
- [ ] Replace `embeddings generate` git extraction
  - Move git extraction logic to scrape command
  - Keep embeddings for vectorization only (Phase 1D)
- [ ] Test materialize with git + session events
  - Run materialize after git scrape
  - Verify observations.db has both session and git observations
  - Check domain tags applied to git observations

**Success Criteria**:
- ✅ Git scraper extracts all commit history
- ✅ Deduplication working (no duplicate observations)
- ✅ Domain tags auto-applied to git observations
- ✅ Extraction tracking by commit hash
- ✅ Git and session observations coexist in observations.db

**Code References**:
- `src/commands/scrape/git.rs` (new)
- `src/commands/embeddings/mod.rs` (remove git extraction, keep vectorization)
- `src/storage/observations.rs` (deduplication logic)

### Phase 1D: Oxidize & Integration (Week 4)

**Tasks**:
- [ ] Rename `embeddings` command to `oxidize`
  - `patina oxidize` generates vectors from observations.db
  - Reads materialized observations
  - Generates embeddings (all-MiniLM-L6-v2)
  - Builds USearch indices
  - Discovers domain relationships via semantic clustering
- [ ] Domain relationship discovery
  - Semantic clustering finds related observations
  - Analyze clusters for domain co-occurrence
  - Calculate co-occurrence strength (0.0-1.0)
  - Insert into domain_relationships table
- [ ] Shared/Local split implementation
  - Create `.patina/shared/` and `.patina/local/` directories
  - Move events to shared/events/
  - Move project.db to shared/
  - Create local/observations.db for scratch space
  - Update .gitignore:
    ```
    .patina/local/
    .patina/shared/project.db
    .patina/shared/vectors/
    ```
  - Git tracks: `.patina/shared/events/`
- [ ] Update all commands for new structure
  - `patina query` searches shared/project.db + local/observations.db
  - `patina belief validate` uses shared/project.db
  - `patina scrape` writes to shared/events/
  - `patina materialize` builds shared/project.db
  - `patina oxidize` builds shared/vectors/
- [ ] End-to-end test
  - Scrape sessions → events
  - Scrape git → events
  - Materialize → project.db
  - Oxidize → vectors + domain relationships
  - Query → semantic search working
  - Belief validate → Prolog validation working
- [ ] Migration guide for existing installations
  - Document backup process
  - Document fresh start process
  - Provide migration script (optional)

**Success Criteria**:
- ✅ `patina oxidize` separate from scrape
- ✅ Domain relationships discovered automatically
- ✅ Shared/local split complete
- ✅ All commands work with new structure
- ✅ End-to-end flow working (scrape → materialize → oxidize → query)
- ✅ Migration guide documented

**Code References**:
- `src/commands/oxidize/mod.rs` (renamed from embeddings)
- `src/commands/query/mod.rs` (update for new structure)
- `src/commands/belief/validate.rs` (update for new structure)
- `docs/migration-phase1.md` (new)

---

## Success Metrics

### Phase 1 Complete When:

**Data**:
- ✅ All 266 sessions extracted as events
- ✅ All git commits extracted as events (deduplicated)
- ✅ ~500-700 observations in observations.db (from events)
- ✅ Observations have domain tags (auto-applied)
- ✅ Domain catalog populated (50-100 domains)
- ✅ Domain relationships discovered

**Commands**:
- ✅ `patina scrape sessions` extracts sessions → events
- ✅ `patina scrape git` extracts commits → events
- ✅ `patina materialize` rebuilds observations.db from events
- ✅ `patina oxidize` generates vectors + domain relationships
- ✅ `patina query` semantic search working
- ✅ `patina belief validate` Prolog validation working

**Structure**:
- ✅ `.patina/shared/events/` contains ~266+ event files
- ✅ `.patina/shared/project.db` materialized from events
- ✅ `.patina/shared/vectors/` contains usearch indices
- ✅ `.patina/local/` for scratch space
- ✅ Event files committed to git
- ✅ Materialized DBs gitignored

**Quality**:
- ✅ Extraction tracking prevents re-scraping
- ✅ Modified sessions re-scraped on change
- ✅ No duplicate observations (content hash deduplication)
- ✅ Domain tags accurate (spot check 20 observations)
- ✅ Domain relationships meaningful (spot check 10 relationships)
- ✅ Full provenance: every observation → event file → git history

---

## Future Phases (Post-Phase 1)

### Phase 2: Cross-Project Persona (4 weeks)

**Goal**: Accumulate beliefs across projects in `~/.patina/persona/`

**Key Features**:
- Persona database aggregates observations from multiple projects
- Cross-project pattern detection (semantic clustering)
- Bubble-up mechanism (project belief → persona belief when validated)
- Context-dependent beliefs (project can differ from persona with justification)

### Phase 3: LLM Integration & Retrieval (2 weeks)

**Goal**: Real-time persona loading for AI during work

**Key Features**:
- LLM adapter loads persona context at conversation start
- Semantic search during AI reasoning (AI queries persona while working)
- Belief-guided code generation (AI respects persona constraints)
- Pattern suggestions (AI: "In 3 other projects, you used X pattern here")

### Phase 4: Temporal Evolution (2 weeks)

**Goal**: Track how beliefs evolve over time

**Key Features**:
- Belief timeline (formed, strengthened, weakened, contextualized)
- Meta-learning insights (analyze how your thinking changed)
- Belief history queries (`patina belief history <belief>`)

### Phase 5: Failed Experiments as Knowledge (2 weeks)

**Goal**: Capture failed experiments as anti-patterns

**Key Features**:
- Tag branches/commits as experiments
- Capture why experiment failed
- Store as anti-pattern observation
- Warning system when AI suggests known failures

---

## Design Principles

### 1. Local-First Privacy

**All data stays on your Mac**:
- No cloud RAG services
- No telemetry
- No training data extraction
- No model API calls for embeddings

**On-device processing**:
- Mac sentence embeddings (ONNX Runtime, Metal GPU)
- Local SQLite databases
- Local Prolog reasoning
- Local vector indices

### 2. LLM Interchangeability

**Persona is permanent, LLM is ephemeral**:
- Beliefs stored outside any specific LLM
- Rules that any LLM must follow
- Claude today, Gemini tomorrow, GPT-5 next week
- All become the persona by loading beliefs + rules + facts

### 3. Event Sourcing for Time Travel

**Events are source of truth**:
- Observations are immutable events
- Databases are derived state
- Change validation rules → replay events → new beliefs
- Full audit trail via git

### 4. Organic Growth

**Don't design beliefs upfront**:
- Capture observations from actual work
- Discover patterns through semantic search
- Validate through Prolog rules + user approval
- Beliefs emerge naturally

### 5. Explainable Reasoning

**Every belief can answer "why?"**:
- Which observations support it
- Similarity scores for each observation
- Prolog rules that validated it
- Projects it came from
- When it was formed
- Full provenance via event files

### 6. Project Autonomy

**Project is king in its domain**:
- Can have beliefs that differ from persona
- Can override persona with justification
- Contributes evidence back to persona
- Maintains local autonomy

**Persona provides context, not control.**

---

## Related Documents

**Architecture**:
- `git-event-sourced-multi-persona-architecture.md` - Event sourcing deep dive, multi-persona collaboration
- `pattern-selection-framework.md` - Pattern selection strategy (Eternal Tools, Stable Adapters, Evolution Points)
- `modular-architecture-plan.md` - Workspace decomposition

**Sessions**:
- `layer/sessions/20251107-061130.md` - Observation extraction architecture exploration
- `layer/sessions/20251107-124740.md` - Neuro-symbolic design session

**Code**:
- `src/reasoning/engine.rs` - ReasoningEngine (embedded Scryer Prolog)
- `src/storage/observations.rs` - ObservationStorage (SQLite + USearch)
- `src/commands/embeddings/mod.rs` - Current extraction (to be refactored)
- `.patina/validation-rules.pl` - Prolog belief validation rules

---

## Next Steps

### Immediate Actions (This Week)

1. **Finalize event file JSON schema**
   - Document structure with examples
   - Define all event types
   - Create schema validation

2. **Implement materialize command**
   - Basic event reading
   - SQLite materialization
   - Test with sample events

3. **Backup existing data**
   - Export current observations.db
   - Document what exists now

4. **Start Phase 1A**
   - Create fresh `.patina/shared/events/` structure
   - Initialize new schema with domains
   - Verify materialize working

### Week-by-Week Plan

**Week 1**: Phase 1A (Event Foundation)
- Event schema + materialize command + fresh structure

**Week 2**: Phase 1B (Session Scraping)
- Session parser + scrape sessions command + extract all 266 sessions

**Week 3**: Phase 1C (Git Scraping)
- Git scraper + deduplication + extract commit history

**Week 4**: Phase 1D (Oxidize & Integration)
- Rename embeddings → oxidize + domain relationships + shared/local split

**Week 5**: Phase 1 polish + documentation

**Week 6+**: Start Phase 2 (Cross-Project Persona)

---

## Critical Design Decisions

### Decisions Made

✅ **Event sourcing from start** - Observations are immutable events, databases are materialized
✅ **Domains as tags** - Not separate databases, auto-tagged during scrape
✅ **Scrape separate from oxidize** - Extraction and vectorization are separate concerns
✅ **Shared/local split** - Git-committed team knowledge vs gitignored scratch space
✅ **LLM for domain tagging** - Use driving LLM (Claude/Gemini) via adapter
✅ **Obsidian-compatible markdown** - Session format works with Obsidian, adapt as needed
✅ **Backup and start fresh** - Existing 463 observations backed up, start with events
✅ **Git-like batch processing** - Explicit capture points, not real-time watching

### Decisions Pending Review

⏳ **Domain tagging LLM** - Use driving LLM (Claude/Gemini), review after doc
⏳ **Event file git storage** - Events committed to git, review if concerns arise
⏳ **Session markdown changes** - Adapt format as needed, review during parser implementation

---

**Status**: Phase 1 design complete, ready for implementation
**Next**: Start Phase 1A (Event Foundation) this week
