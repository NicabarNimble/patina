---
id: architecture-patina-system
status: needs-update
created: 2025-10-13
updated: 2025-12-09
oxidizer: nicabar
tags: [architecture, system-design, comprehensive, core-documentation]
references: [dependable-rust, reference-patina-metal]
---

# Patina System Architecture
**Definitive Documentation - October 2025**

> Context orchestration for AI-assisted development - a tool that captures and evolves development patterns, making AI assistants smarter about your projects over time.

---

## Executive Summary

**Patina** is a context management system that solves the fundamental challenge of AI-assisted development: constantly re-teaching AI assistants about project context, patterns, and constraints. Like the protective patina that forms on metal, development wisdom accumulates over time and becomes more valuable with each session.

### What Patina Is Today (October 2025)

Patina is a **production-ready Rust CLI** (v0.1.0) with the following core capabilities:

- **Layer System**: Organize knowledge as Core (eternal) → Surface (active) → Dust (historical) patterns
- **Multi-Language Code Analysis**: Extract semantic facts from 9+ programming languages via patina-metal
- **Session Management**: Git-integrated session tracking with automatic distillation to layer/sessions/
- **LLM Adapters**: Pluggable support for Claude (mature) and Gemini (experimental)
- **Semantic Database**: SQLite-based code indexing with Git-aware navigation
- **YOLO Command**: Automated devcontainer generation for instant development environments
- **Reference Repositories**: Track and scrape external codebases for pattern learning

### Core Value Proposition

1. **Accumulates Knowledge**: Every session, pattern, and decision is captured and made searchable
2. **Token-Efficient Context**: Compress large codebases into factual, queryable databases
3. **LLM-Agnostic**: Works with any AI assistant through adapter pattern
4. **Pattern Evolution**: Track which patterns survive over time via Git integration
5. **Tool-Based Decomposition**: Breaks complex systems into LLM-friendly tools

---

## System Architecture

### High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      Patina CLI (Rust)                       │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌────────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │  Commands  │  │   Adapters   │  │  Layer System      │  │
│  │            │  │              │  │                    │  │
│  │ • init     │  │ • Claude     │  │ • Core patterns    │  │
│  │ • scrape   │  │ • Gemini     │  │ • Surface docs     │  │
│  │ • doctor   │  │              │  │ • Dust archives    │  │
│  │ • ask      │  │              │  │ • Session logs     │  │
│  │ • yolo     │  │              │  │                    │  │
│  └────────────┘  └──────────────┘  └────────────────────┘  │
│         │                │                     │             │
│         └────────────────┴─────────────────────┘             │
│                          │                                   │
├──────────────────────────┼───────────────────────────────────┤
│                          ▼                                   │
│  ┌───────────────────────────────────────────────────────┐  │
│  │           Patina-Metal (Parser Subsystem)             │  │
│  │                                                       │  │
│  │  • Tree-sitter integration (9 languages)             │  │
│  │  • Cairo native parser                               │  │
│  │  • Semantic fact extraction                          │  │
│  │  • Git-aware indexing                                │  │
│  └───────────────────────────────────────────────────────┘  │
│                          │                                   │
└──────────────────────────┼───────────────────────────────────┘
                           ▼
         ┌─────────────────────────────────────┐
         │   Storage Layer                     │
         │                                     │
         │  • SQLite (knowledge.db)            │
         │  • File system (layer/)             │
         │  • Git (version control & memory)   │
         └─────────────────────────────────────┘
```

### Technology Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| **Core CLI** | Rust 2021 | Type safety, performance, zero-cost abstractions |
| **Database** | SQLite (rusqlite) | Embedded, fast, no external dependencies |
| **Parsing** | Tree-sitter 0.24 | Multi-language AST parsing, battle-tested |
| **Parallelism** | Rayon | CPU-bound parallelism without async complexity |
| **HTTP** | Reqwest (blocking) | Simple network calls, no async runtime needed |
| **Serialization** | Serde + TOML/JSON | Configuration and data interchange |
| **CLI Framework** | Clap 4.5 | Derive macros, excellent UX |
| **Error Handling** | Anyhow | Simple error propagation |
| **CRDT** | Automerge 0.5 | Future distributed coordination |

**Key Architectural Decision**: No async runtime (tokio). Patina's workload is synchronous (file I/O, SQLite queries, git commands, CPU-bound parsing). Using Rayon for parallelism is simpler and more idiomatic.

---

## Core Components

### 1. CLI Command System

**Location**: `src/main.rs`, `src/commands/`

**Purpose**: User-facing interface for all Patina operations

**Commands Available**:

| Command | Status | Purpose | Key Features |
|---------|--------|---------|--------------|
| `init` | Production | Initialize/re-initialize project | Git setup, adapter selection, pattern copying |
| `scrape` | Production | Build semantic database | Code/docs/pdf extraction, incremental indexing |
| `doctor` | Production | Health check & diagnostics | Environment validation, repo staleness checks |
| `ask` | Production | Query knowledge base | Natural language pattern search |
| `yolo` | Production | Generate devcontainers | Auto-detect tech stack, create .devcontainer/ |
| `build` | Production | Docker containerized builds | Isolated build environments |
| `test` | Production | Run tests in containers | Consistent test execution |
| `upgrade` | Production | Check for updates | Version checking, changelog display |
| `version` | Production | Version information | CLI and component versions |

**Implementation Pattern**: All commands follow the **dependable-rust** pattern:
- Public interface in `mod.rs` (≤150 lines)
- Internal implementation in `internal/` modules
- Clear separation of concerns

**Design Document**: `layer/core/dependable-rust.md`

---

### 2. LLM Adapters

**Location**: `src/adapters/`

**Purpose**: Provide LLM-specific integrations while maintaining a common interface

#### Claude Adapter (Mature - v1.0.0)

**Status**: Production-ready, most feature-complete

**Features**:
- `.claude/` directory structure with CLAUDE.md context files
- Session management commands: `/session-start`, `/session-update`, `/session-note`, `/session-end`
- Git-integrated session tracking with automatic tagging
- Experimental branch creation via `/launch [branch]`
- Auto-generated context from environment detection
- Version tracking with changelog display

**Session Scripts** (`resources/claude/.claude/bin/`):
- `session-git-start.sh` - Create branch, tag starting commit
- `session-git-update.sh` - Track progress with git metrics
- `session-git-note.sh` - Capture insights with git context
- `session-git-end.sh` - Distill session to layer/sessions/

**Session Classification**: Work types automatically detected:
- `pattern-work` - 5+ commits, patterns modified
- `exploration` - 0-4 commits, research/prototyping
- `maintenance` - Docs, tests, refactoring

**Key Innovation**: Git as memory system. Every session gets git tags marking start/end, enabling survival metrics for patterns.

**File**: `src/adapters/claude/mod.rs` (120 lines, within dependable-rust limit)

#### Gemini Adapter (Experimental)

**Status**: Basic structure in place, limited features

**Features**:
- `.gemini/` context file generation
- Compatible with Gemini API

**Future Work**: Session commands, advanced features to match Claude adapter

**Design Principle**: Adapter pattern allows adding new LLMs (GPT-4, local models) without changing core system.

---

### 3. Session Management System

**Location**: `src/session.rs`, `resources/claude/`

**Purpose**: Capture development sessions with Git integration for knowledge distillation

**Architecture**:

```
Session Lifecycle:
┌──────────────┐
│ /session-start│ → Creates git branch + tag
└──────┬────────┘
       ▼
┌──────────────┐
│ Active Session│ → layer/sessions/active-session.md (working file)
└──────┬────────┘
       ▼
┌──────────────┐
│ /session-update│ → Git metrics, progress tracking
└──────┬────────┘
       ▼
┌──────────────┐
│ /session-note │ → Capture insights mid-session
└──────┬────────┘
       ▼
┌──────────────┐
│ /session-end  │ → Distill to timestamped file + classification
└──────┬────────┘
       ▼
layer/sessions/YYYYMMDD-HHMMSS.md (permanent record)
```

**Git Integration**:
- Session start: Creates `session-{id}-start` tag
- Session end: Creates `session-{id}-end` tag
- Enables querying: "What patterns were added between these tags?"
- Survival metrics: "How long did this pattern last before being replaced?"

**Session File Structure**:
```markdown
# Session: [goal]
**ID**: YYYYMMDD-HHMMSS
**Started**: ISO timestamp
**LLM**: claude
**Git Branch**: feature/branch-name
**Session Tag**: session-{id}-start
**Starting Commit**: {hash}

## Previous Session Context
[Summary from last session's end]

## Goals
- [ ] Goal 1
- [ ] Goal 2

## Activity Log
### HH:MM - Update
[Work completed, decisions, challenges, patterns]

## Session Classification
- Work Type: pattern-work | exploration | maintenance
- Files Changed: N
- Commits: N
- Patterns Modified: N
- Session Tags: start..end
```

**Value**: 227+ sessions captured (as of Oct 2025), forming a searchable knowledge base of project evolution.

**Future Enhancement**: Neuro-symbolic extraction with Prolog for logical inference (experimental in `layer/buckets/`).

---

### 4. Indexer & Semantic Search (Patina-Metal)

**Location**: `patina-metal/` (workspace package)

**Purpose**: Multi-language code analysis for token-efficient LLM context

#### The Problem

LLMs need to understand codebases but can't read every file. Solution: Extract semantic facts into queryable database.

**Example Efficiency**:
```sql
-- Instead of 5000 tokens of source code:
SELECT name, parameters, return_type, complexity
FROM functions
WHERE name LIKE '%auth%';
-- Returns: 50 tokens of structured facts
```

#### Architecture

```
patina-metal/
├── src/
│   ├── lib.rs         # Unified Analyzer API
│   ├── metal.rs       # Language enum (9 languages)
│   ├── grammars.rs    # FFI bindings to C parsers
│   └── parser.rs      # Tree-sitter wrapper
├── grammars/          # Git submodules (exact versions)
│   ├── rust/
│   ├── go/
│   ├── solidity/
│   ├── python/
│   ├── javascript/
│   ├── typescript/
│   ├── c/
│   ├── cpp/
│   └── cairo/         # Native parser, not tree-sitter
└── build.rs           # Compiles C parsers
```

#### Language Support (All Production-Ready)

| Language | Files Tested | Symbols Extracted | Special Features |
|----------|--------------|-------------------|------------------|
| Rust | 151+ | 3,500+ | Trait impls, macros, unsafe blocks |
| Go | 832+ | 6,420+ | Interfaces, goroutines |
| Solidity | 209+ | 1,200+ | Contract inheritance, events |
| Python | 200+ | 1,328+ | Decorators, async functions |
| JavaScript | 100+ | 212+ | Classes, closures |
| TypeScript | 150+ | 513+ | Interfaces, type aliases |
| C | 50+ | 400+ | Structs, function pointers |
| C++ | 100+ | 800+ | Classes, templates, inheritance |
| Cairo | 50+ | 296+ | Native parser, trait impls, storage |

#### Extraction Categories

**1. Code Fingerprints** (Functions)
- Name, parameters, return type
- Complexity score (cyclomatic)
- Visibility (pub/private)
- Async/unsafe markers
- Location (file:line)

**2. Type Vocabulary**
- Structs, classes, enums, interfaces
- Type parameters (generics)
- Member fields
- Methods

**3. Import Facts**
- Module dependencies
- Re-exports
- Use statements

**4. Behavioral Hints**
- Panic/error sites
- TODO/FIXME comments
- Unsafe blocks
- Assert statements

**5. Git Metrics** (Integration layer)
- Change frequency per file
- Last modified timestamp
- Pattern survival time

#### Database Schema

**SQLite Tables** (`.patina/knowledge.db`):

```sql
CREATE TABLE code_fingerprints (
    id INTEGER PRIMARY KEY,
    file_path TEXT NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,  -- function, method, etc
    parameters TEXT,     -- JSON array
    return_type TEXT,
    complexity INTEGER,
    line_start INTEGER,
    line_end INTEGER,
    visibility TEXT,
    is_async BOOLEAN,
    is_unsafe BOOLEAN,
    git_last_modified TEXT,
    UNIQUE(file_path, name, line_start)
);

CREATE TABLE type_vocabulary (
    id INTEGER PRIMARY KEY,
    file_path TEXT NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,  -- struct, enum, class, interface
    type_params TEXT,    -- JSON array for generics
    UNIQUE(file_path, name)
);

CREATE TABLE import_facts (
    id INTEGER PRIMARY KEY,
    file_path TEXT NOT NULL,
    imported_module TEXT NOT NULL,
    imported_items TEXT, -- JSON array
    is_reexport BOOLEAN DEFAULT FALSE
);

CREATE TABLE behavioral_hints (
    id INTEGER PRIMARY KEY,
    file_path TEXT NOT NULL,
    kind TEXT NOT NULL,  -- panic, todo, unsafe, assert
    line_number INTEGER,
    context TEXT,
    severity TEXT
);
```

#### Usage

```bash
# Initialize database
patina scrape --init

# Index current project
patina scrape

# Index reference repository
patina scrape --repo dagger

# Batch index all repos
patina scrape --repo all

# Re-index stale repos (from doctor)
patina scrape --repo doctor

# Force full re-index
patina scrape --force

# Query database directly
patina scrape --query "SELECT COUNT(*) FROM code_fingerprints"
```

**Performance**: Handles 1000+ file repositories with incremental indexing. Respects `.gitignore` and `.ignore` patterns via `ignore` crate.

**Design Document**: `layer/surface/patina-metal-explained.md`

---

### 5. Pattern Storage (Layer System)

**Location**: `layer/`, `src/layer/`

**Purpose**: Organize knowledge hierarchically with Git-based evolution tracking

#### Layer Structure

```
layer/
├── core/              # Eternal patterns (stable, well-tested)
│   ├── dependable-rust.md
│   ├── adapter-pattern.md
│   ├── unix-philosophy.md
│   └── session-capture.md
│
├── surface/           # Active development (evolving patterns)
│   ├── pattern-selection-framework.md
│   ├── modular-architecture-plan.md
│   ├── patina-metal-explained.md
│   └── architecture/
│       └── layer-structure-evolution.md
│
├── dust/              # Historical/archived
│   ├── repos/         # Reference repositories for learning
│   │   ├── dagger/
│   │   ├── duckdb/
│   │   └── .patina-update.log
│   └── deprecated/
│
├── sessions/          # Session distillations (227+ files)
│   ├── 20251007-210232.md
│   ├── 20251010-123308.md
│   └── active-session.md  (working file)
│
└── buckets/           # Experimental: Domain-specific knowledge
    └── patina-dev/    # Prolog facts + rules
        ├── facts.pl
        ├── rules.pl
        └── queries.md
```

#### Pattern Evolution Flow

```
Dust (Historical) → Surface (Active) → Core (Eternal)
        ↓                  ↓                ↓
   Experimental        Proven in         Stable across
   ideas, failed       one project       multiple projects
   attempts
```

**Philosophy**: Not all patterns are equal. Applies different architectural approaches based on code characteristics (see Pattern Selection Framework).

#### Git as Memory System

**Key Innovation**: Use Git not just for version control, but as a memory system for AI development.

**Survival Metrics**:
```bash
# How long did this pattern survive?
git log --follow --oneline layer/surface/some-pattern.md

# What changed between these sessions?
git diff session-A-start..session-B-end layer/
```

**Pattern Confidence Scoring**:
- **Dust**: Experimental (confidence: 0.3)
- **Surface**: Active development (confidence: 0.6)
- **Core**: Proven, stable (confidence: 0.9)
- **Multiplied by Git state**: Committed > Staged > Untracked

#### Reference Repositories

**Location**: `layer/dust/repos/`

**Purpose**: Clone external codebases to learn patterns from successful projects

**Management**:
```bash
# Check repository health
patina doctor --repos

# Update stale repositories
patina doctor --repos --update

# Scrape all reference repos
patina scrape --repo all
```

**Tracked Repos** (12+ as of Oct 2025):
- dagger, duckdb, cuti, dust, humanlayer, kit, game-engine, death-mountain, etc.

**Update Log**: `.patina-update.log` tracks git pull operations and staleness.

**Design Document**: `layer/surface/architecture/layer-structure-evolution.md`

---

### 6. YOLO Command (Devcontainer Generator)

**Location**: `src/commands/yolo/`

**Status**: Production (merged to main October 2025)

**Purpose**: Auto-generate devcontainers from project analysis

#### What It Does

Scans project and generates a complete `.devcontainer/` setup:
- Detects programming languages and frameworks
- Installs required tools (rustc, go, node, python, etc.)
- Configures VSCode extensions
- Sets up docker-compose with volume mounts
- Integrates Claude Code CLI for AI assistance

#### Technology Detection

**Profile System**:
```rust
pub enum Profile {
    Rust,
    Go,
    Python,
    Node,
    Cairo,       // Blockchain language
    Dojo,        // Cairo framework
    Solidity,    // Smart contracts
    // Mix and match
}
```

**Scanner** (`scanner.rs`):
- Walks project tree respecting `.gitignore`
- Detects from config files: `Cargo.toml`, `go.mod`, `package.json`, `pyproject.toml`, `Scarb.toml`
- Framework detection: Dojo (Cairo game engine), Next.js, Django, etc.

#### Generated Structure

```
.devcontainer/
├── devcontainer.json       # VSCode config
├── docker-compose.yml      # Multi-container setup
├── Dockerfile              # Custom image
└── yolo-setup.sh          # Container initialization script
```

#### Security Features

**1Password Integration**:
- Credentials stored in 1Password vault (not on disk)
- Retrieved at container startup with Touch ID auth
- Mounted to tmpfs (RAM-only, disappears on exit)
- Never committed to git

**Path Security**:
- Uses `${HOME}` instead of hardcoded paths
- Validates project names (no injection attacks)
- `.gitignore` for sensitive files

#### Usage

```bash
# Generate devcontainer
patina yolo

# Custom name
patina yolo my-custom-container

# With specific profile (future)
patina yolo --profile rust-embedded
```

**Design Decisions**:
- Hybrid approach: Official VSCode `settings.json` API + docker-compose for flexibility
- bypassPermissions: true for root access (documented YOLO tradeoff)
- Claude Code CLI integrated for autonomous AI work in container

**Related Sessions**:
- `20251007-210232.md` - Security review and credential management
- `20251009-064522.md` - GitHub integration patterns

---

### 7. Environment Management

**Location**: `src/environment.rs`, `src/dev_env/`

**Purpose**: Detect available tools and configure development environments

#### Environment Detection

```rust
pub struct Environment {
    pub platform: Platform,      // macos, linux, windows
    pub available_tools: Vec<Tool>,
    pub languages: Vec<Language>,
}

pub enum Tool {
    Cargo,
    Git,
    Docker,
    Go,
    Python,
    Node,
    // ... extensible
}
```

**Detection Methods**:
- `which` command for tool availability
- Version checking for compatibility
- Feature flags for optional components

#### Dev Environment Providers

**Current**: Docker (production-ready)
```bash
patina build    # Docker containerized build
patina test     # Run tests in container
```

**Historical**: Dagger (removed Sept 2025)
- Reason: Complexity vs value tradeoff
- Go SDK requirement conflicted with Rust-first philosophy
- Docker provides sufficient isolation for current needs

**Design Decision**: Removed Dagger in favor of simpler Docker approach (commits `bd89545..91aed26`). Following escape hatch philosophy - don't force tools on users.

---

### 8. Git Integration

**Location**: `src/git/`, `resources/git/`

**Purpose**: Git operations for memory system, fork handling, and workflow automation

#### Features

**1. Fork Management** (`src/git/fork.rs`):
- Detects if repo is a fork
- Creates private forks for Pro users
- Handles upstream remote configuration
- Respects user choice (public vs private)

**2. Branch Strategy**:
- `patina` branch = primary stable branch (not main!)
- Feature branches for experiments
- Session tags for tracking evolution

**3. Pre-Push Checks** (`resources/git/pre-push-checks.sh`):
```bash
#!/bin/bash
cargo fmt --all           # Format code
cargo clippy --workspace  # Linting
cargo test --workspace    # Tests
```

**CI Integration**: GitHub Actions enforce same checks.

#### Git as Memory System

**Philosophy**: Git isn't just version control, it's the memory layer for AI development.

**Session Tagging**:
```bash
git tag session-20251013-120000-start
# ... work happens ...
git tag session-20251013-150000-end
```

**Queries Enabled**:
- "Show me all changes during session X"
- "When was this pattern introduced?"
- "How many times has this file been modified?"
- "Which patterns survived longest?"

**Design Document**: `layer/surface/commands/git-integration-migration.md`

---

### 9. Ask Command (Pattern Search)

**Location**: `src/commands/ask/`

**Status**: Production

**Purpose**: Natural language search across patterns and code knowledge base

#### Features

- Query pattern files (layer/core, layer/surface)
- Search code fingerprints from SQLite
- Confidence scoring based on layer + git state
- Results ranked by relevance

#### Usage

```bash
# Search patterns
patina ask "How do I structure Rust modules?"

# Search code
patina ask "Where are async functions defined?"

# Combine both
patina ask "Show me error handling patterns used in this codebase"
```

**Implementation**: Uses SQLite full-text search + pattern file scanning.

---

### 10. Doctor Command (Health Check)

**Location**: `src/commands/doctor.rs`

**Purpose**: Comprehensive health check and diagnostics

#### Checks Performed

**Environment**:
- Required tools present (git, cargo)
- Optional tools available (docker, go)
- Version compatibility

**Project Structure**:
- `.patina/` directory exists
- `layer/` structure valid
- Database integrity

**Reference Repositories** (`--repos`):
- Git repo health (clean working tree, fetch successful)
- Update status (behind upstream by N commits)
- Database staleness (git changes since last scrape)

**Update Mode** (`--repos --update`):
- Safe git pull (only if working tree clean)
- Logs all operations to `.patina-update.log`
- Marks databases as STALE if git changes detected

#### Output Modes

**Human-readable** (default):
```
✅ Cargo: 1.75.0
✅ Git: 2.40.0
⚠️  Docker: Not found (optional)

Reference Repositories:
✅ dagger: Up to date
⚠️  duckdb: Behind by 150 commits (database STALE)
```

**JSON** (`--json`):
```json
{
  "environment": {
    "cargo": { "found": true, "version": "1.75.0" }
  },
  "repos": [
    {
      "name": "duckdb",
      "status": "stale",
      "commits_behind": 150
    }
  ]
}
```

**Integration**: Output feeds into `patina scrape --repo doctor` for targeted re-indexing.

---

## Design Decisions & Rationale

### Why Rust?

**Decision**: Build core CLI in Rust, avoid async runtime

**Rationale**:
1. **Type safety**: Compiler catches errors at compile time (LLMs + types = safer code)
2. **Performance**: Zero-cost abstractions, perfect for parsing/indexing
3. **Single binary**: Distribute via `cargo install`, no dependencies
4. **Ecosystem**: Excellent CLI tooling (clap, anyhow, rusqlite)
5. **Synchronous by default**: Patina's workload is file I/O and CPU-bound, not network I/O

**From Session 20251002-055812**:
> "Rust-first philosophy validated. The 15,280 lines of Rust core vs 1,104 lines of Go modules confirms our architectural choice. Go modules were Dagger-specific, now deprecated."

**Design Document**: `layer/core/oxidized-knowledge.md`

---

### Why Git as Memory?

**Decision**: Use Git not just for version control, but as the memory system

**Rationale**:
1. **Already required**: Every project has git
2. **Built-in time travel**: Git log shows pattern evolution
3. **Confidence signals**: Committed > Staged > Untracked maps to pattern maturity
4. **Survival metrics**: Track how long patterns last before replacement
5. **Session boundaries**: Git tags mark session start/end for queries

**From Session 20251007-210232**:
> "Git memory commands for work tracking and survival metrics enable pattern success measurement. Session tagging creates queryable timeline of development."

**Design Document**: `layer/surface/commands/session-git-start-flow.md` (and related)

---

### Why SQLite Over DuckDB?

**Decision**: Migrate from DuckDB to SQLite (Sept 2025, PR #32)

**Rationale**:
1. **Simpler dependency**: No bundled C++ compilation needed
2. **Faster CI**: Build time reduced from 10+ min to <5 min
3. **Better Rust integration**: rusqlite is mature and idiomatic
4. **Sufficient performance**: No need for DuckDB's analytical features
5. **Wider compatibility**: SQLite works everywhere, including embedded

**Commits**: `01ac249..a9a7d0d` (9 commits total)

**Migration Details**:
- Replaced `Appender` with transactions
- Changed `VARCHAR` to `TEXT` (SQLite's native type)
- Updated all query patterns
- Zero behavioral changes (drop-in replacement)

**From Commit a9a7d0d**:
> "docs: update layer/surface docs for SQLite migration. DuckDB was overkill for our use case. SQLite is simpler, faster to build, and perfectly adequate."

---

### Why LLM Adapters vs Single Provider?

**Decision**: Adapter pattern for LLM integration

**Rationale**:
1. **Vendor independence**: Don't lock users into one AI provider
2. **Evolution-friendly**: New LLMs released constantly
3. **User choice**: Some prefer Claude, others Gemini, some want local models
4. **API isolation**: Changes in one adapter don't affect others
5. **Future-proof**: When GPT-5/Claude-5/Gemini-3 arrive, just add adapter

**Trait Definition** (`src/adapters/mod.rs`):
```rust
pub trait LLMAdapter {
    fn name(&self) -> &'static str;
    fn init_project(&self, ...) -> Result<()>;
    fn get_custom_commands(&self) -> Vec<(&'static str, &'static str)>;
    fn check_for_updates(&self, ...) -> Result<Option<(String, String)>>;
    // ... extensible
}
```

**Design Document**: `layer/core/adapter-pattern.md`

---

### Why Session-Based Capture?

**Decision**: Structured session files with Git integration

**Rationale**:
1. **Context continuity**: Each session builds on previous context
2. **Decision history**: Why was this choice made? Check the session.
3. **Pattern emergence**: Recurring themes across sessions become patterns
4. **Learning from failures**: Failed experiments documented, not lost
5. **Token-efficient recall**: Summarized sessions vs re-reading all files

**From Session 20251010-123308**:
> "Session structure enables extraction: Consistent sections (Patterns Observed, Key Decisions, Challenges Faced) map to database tables. LLM extracts facts from unstructured prose, Prolog infers conclusions from structured facts."

**Design Document**: `layer/core/session-capture.md`

---

### Why Modular Workspace Architecture?

**Decision**: Decompose systems into tool-sized pieces

**Rationale** (from Pattern Selection Framework):
1. **LLMs excel at tools**: Clear input → output transformations
2. **LLMs struggle with systems**: Stateful orchestration across contexts
3. **Tool-based decomposition**: Break complex systems into LLM-friendly units
4. **Single responsibility**: Each module does one thing well
5. **Testability**: Tools are easier to test than systems

**Three Categories of Code**:
- **Eternal Tools**: Stable domain, clear I/O (e.g., parser, hasher)
- **Stable Adapters**: Bridge to external systems (e.g., LLM adapters)
- **Evolution Points**: Rapidly changing, planned obsolescence (e.g., container orchestration)

**From Pattern Selection Framework**:
> "Patina's value isn't in forcing one pattern everywhere, but in knowing which pattern fits where. It maintains the system understanding that LLMs lack, enabling effective decomposition of complex systems into tool-sized pieces."

**Design Document**: `layer/surface/pattern-selection-framework.md`

---

### Why Dependable Rust Pattern?

**Decision**: Strict module structure (public interface ≤150 lines, internal/ for implementation)

**Rationale**:
1. **Reviewability**: Public API fits on one screen
2. **Stability**: Small external interface = fewer breaking changes
3. **Documentation**: Forces good docs (must fit in 150 lines)
4. **LLM-friendly**: Clear boundaries for AI to understand
5. **Refactoring safety**: Can change internals without breaking public API

**Pattern Definition**:
```
module/
├── mod.rs          # Public API (≤150 lines)
└── internal.rs     # Implementation (unlimited)
```

**Example**: `src/adapters/claude/mod.rs` is exactly 120 lines (within limit).

**CI Enforcement**: Pre-push checks validate line counts.

**Design Document**: `layer/core/dependable-rust.md`

---

### Why Tool-Based Decomposition?

**Decision**: Break systems into Unix-philosophy tools

**Rationale**:
1. **Do one thing well**: Each tool has single responsibility
2. **Composability**: Tools combine for complex workflows
3. **Testability**: Isolated tools are easier to test
4. **Replacement**: Tools can be swapped without affecting others
5. **LLM comprehension**: "Do X" is easier than "Manage Y system"

**Example Decomposition** (planned for modules/):
```
Monolithic workspace service
      ↓
environment-provider    (create containers)
environment-registry    (track containers)
code-executor          (run commands)
git-manager            (git operations)
api-gateway            (HTTP coordination)
```

**Design Document**: `layer/surface/modular-architecture-plan.md`

---

### Why Docker Over Native?

**Decision**: Containerized builds and tests as default

**Rationale**:
1. **Reproducibility**: Same environment everywhere
2. **Isolation**: Don't pollute host system
3. **CI/CD alignment**: What works locally works in CI
4. **Tool version control**: Lock exact versions in container
5. **Escape hatch**: Native builds still possible

**From PROJECT_DESIGN.toml**:
> "Uses Docker for containerized builds. Never requires specific tools beyond Docker. Clear feedback about what's being used."

---

## Evolution Timeline

### July 2025: Foundation
- Initial concept and CLI structure
- Brain → Layer terminology shift (session_summary_july27.md)
- Pattern storage system established

### August 2025: Core Patterns
- Dependable Rust pattern formalized (layer/core/dependable-rust.md)
- Pattern Selection Framework created
- Multi-language parsing research begins
- SQLite + Automerge CRDT design (git-aware-navigation-design.md)

### September 2025: Major Refactorings
- **DuckDB → SQLite migration** (PR #32, 9 commits)
  - Rationale: Simpler, faster CI builds
  - Impact: Zero behavioral change, 50% CI speedup
- **Dagger removal** (commits bd89545..91aed26)
  - Rationale: Complexity vs value, Go dependency
  - Replacement: Simple Docker approach
- Language extraction: 9/9 languages complete (Rust, Go, Python, JS, TS, C, C++, Solidity, Cairo)
- Reference repository management (doctor --repos --update)

### October 2025: Production Features
- **YOLO Command** (feature/yolo-command branch)
  - Devcontainer generation from project analysis
  - 1Password integration for secure credentials
  - Scanner optimization (ignore crate for gitignore respect)
- **GitHub Actions integration** (automation repo)
  - Repo cleanup workflows
  - Branch protection automation
- **Neuro-symbolic exploration** (layer/buckets/)
  - Prolog integration for logical inference
  - LLM extracts facts, Prolog infers knowledge

### Key Milestones

| Date | Milestone | Commits | Sessions |
|------|-----------|---------|----------|
| Jul 27 | Brain → Layer rename | 15 | 1 |
| Aug 13 | Pattern Selection Framework | - | Multiple |
| Sept 1-10 | Language extractions complete | 20+ | 10+ |
| Sept 28 | DuckDB → SQLite migration | 9 | 1 |
| Oct 1 | Batch scraping (all repos) | 8 | 1 |
| Oct 7-9 | YOLO command development | 15+ | 3 |
| Oct 10 | Neuro-symbolic Prolog POC | 7 | 1 |

**Total Sessions Captured**: 227+ (layer/sessions/)

---

## Current Capabilities

### Production-Ready Features

✅ **Project Initialization**
```bash
patina init my-project --llm claude --dev docker
patina init .  # Re-initialize/update current project
```

✅ **Semantic Code Indexing**
```bash
patina scrape --init             # Create database
patina scrape                    # Index codebase
patina scrape --repo dagger      # Index reference repo
patina scrape --repo all         # Batch index all repos
patina scrape --repo doctor      # Re-index stale repos
```

✅ **Pattern Search**
```bash
patina ask "error handling patterns"
patina ask "async function definitions"
```

✅ **Health Diagnostics**
```bash
patina doctor                    # Full health check
patina doctor --repos            # Check reference repos
patina doctor --repos --update   # Update stale repos
```

✅ **Session Management** (Claude adapter)
```bash
/session-start "Feature implementation"
/session-update
/session-note "Key insight about architecture"
/session-end
/launch experiment-branch  # Create experimental branch
```

✅ **Container Builds**
```bash
patina build    # Docker containerized build
patina test     # Run tests in container
```

✅ **Version Management**
```bash
patina version            # Show version
patina version --components  # Show all component versions
patina upgrade            # Check for updates
```

✅ **YOLO Devcontainer Generation**
```bash
patina yolo               # Auto-generate .devcontainer/
patina yolo --defaults    # Use all defaults without prompting
patina yolo --with cairo,solidity  # Include additional tools
```

### Experimental Features

🚧 **Neuro-Symbolic Inference** (layer/buckets/)
- Prolog rules for pattern classification
- LLM fact extraction from sessions
- Logical inference over extracted facts

### Language Support (Patina-Metal)

All production-ready, comprehensive extraction:

| Language | Status | Extraction Features |
|----------|--------|---------------------|
| **Rust** | ✅ Full | Functions, structs, enums, traits, impls, macros, unsafe blocks |
| **Go** | ✅ Full | Functions, interfaces, structs, methods, goroutines |
| **Solidity** | ✅ Full | Contracts, functions, events, modifiers, inheritance |
| **Python** | ✅ Full | Functions, classes, decorators, async/await |
| **JavaScript** | ✅ Full | Functions, classes, arrow functions, async |
| **TypeScript** | ✅ Full | All JS + interfaces, type aliases, generics |
| **C** | ✅ Full | Functions, structs, typedefs, macros |
| **C++** | ✅ Full | Classes, templates, inheritance, namespaces |
| **Cairo** | ✅ Full | Functions, traits, impls, struct fields (native parser) |

**Future**: More languages as needed (Zig, Swift, Kotlin, etc.)

---

## Key Patterns & Philosophy

### 1. Tool-Based Decomposition

**Principle**: LLMs build tools, not systems. Decompose complex systems into tool-sized pieces.

**From Pattern Selection Framework**:
- **Tools**: Single operation, stateless, clear I/O
- **Systems**: Multiple operations, stateful, context-dependent

**Application in Patina**:
- `patina scrape` is a tool (index codebase)
- `patina doctor` is a tool (health check)
- Together they form a system (knowledge management)

### 2. Pattern Evolution (Dust → Surface → Core)

**Principle**: Not all patterns are equal. They evolve from experimental to eternal.

**Layer Flow**:
```
Dust (Failed experiments, historical)
  ↓ Prove value in one project
Surface (Active patterns, being tested)
  ↓ Prove value across multiple projects
Core (Eternal patterns, universally applicable)
```

**Example**:
- Dagger integration started in Surface
- Moved to Dust when removed (experiment failed)
- Docker approach stayed in Surface (proven but evolving)

### 3. Escape Hatches Everywhere

**Principle**: Never lock users in. Always provide manual alternatives.

**Examples**:
- Can query SQLite directly with `--query`
- Can edit layer/ files manually
- Can bypass patina commands and use git directly
- Can run native builds without Docker

**From PROJECT_DESIGN.toml**:
> "Always provide escape hatches. Never lock users in."

### 4. Git as Memory, Not Workflow Enforcement

**Principle**: Git stores history, but doesn't dictate process.

**What Git Tracks**:
- Pattern evolution over time
- Session boundaries (tags)
- Code survival metrics
- Decision history

**What Git Doesn't Enforce**:
- Specific branching model
- Commit message format
- PR requirements

**From CLAUDE.md**:
> "Git Discipline: Commit often, use a scalpel not a shotgun. One commit = one purpose."

### 5. Information Over Automation

**Principle**: Provide information, let users decide actions.

**Examples**:
- `doctor --repos` shows staleness, doesn't auto-update
- `upgrade` shows new version, doesn't auto-install
- `scrape` shows what will be indexed, requires confirmation for destructive ops

**Design Philosophy**: Users maintain control, patina provides insight.

### 6. LLM-Agnostic Design

**Principle**: Work with any AI assistant through adapters.

**Current**: Claude (mature), Gemini (basic)
**Future**: GPT-4, local models, custom agents

**Adapter Trait** defines common interface, implementations handle specifics.

### 7. Token Efficiency

**Principle**: Compress information for LLM context windows.

**Techniques**:
- Semantic extraction (facts vs full source)
- Pattern summaries (core concepts, not full docs)
- Session distillation (key decisions, not full transcript)
- Incremental indexing (only changed files)

**Impact**: 100:1 compression ratio typical (5000 tokens → 50 tokens of facts).

### 8. Dependable Module Boundaries

**Principle**: Small external interfaces, unlimited internal complexity.

**Black Box Pattern**:
- Public API: ≤150 lines, well-documented, stable
- Internal implementation: Any size, free to evolve

**Benefit**: Refactor internals without breaking users.

---

## Known Limitations & Future Directions

### Current Limitations

**Performance**:
- Large repos (5000+ files) can take 3-5 minutes to scrape
- No progress indicators during long operations (UX issue identified, attempts reverted)

**Language Coverage**:
- Cairo language has incomplete feature extraction (attributes, storage)
- No support yet for: Zig, Swift, Kotlin, Ruby, PHP

**Session Management**:
- Claude adapter is mature, Gemini adapter is basic
- Session classification is rule-based (could use ML)
- Cross-session pattern detection is manual

**YOLO Command**:
- Limited to detected profiles (can be extended with --with flag)
- 1Password integration for secure credentials (fallback to bind mount if not available)

**Workspace Agent**:
- Removed Dagger-based workspace agent (Sept 2025)
- Command execution crashes (bug unfixed)
- HTTP interface is awkward for Max subscription users (needs MCP wrapper)

**Neuro-Symbolic**:
- Prolog integration is proof-of-concept only
- Manual fact extraction (no automation yet)
- Limited to patina-dev domain bucket

### Near-Term Future (3-6 months)

**YOLO Command Maturity**:
- Merge to main branch
- Support more frameworks (Next.js, Django, FastAPI)
- Alternative credential providers (Bitwarden, encrypted volumes)

**Pattern Success Metrics**:
- Automatic calculation of pattern survival time
- Confidence scoring based on git metrics
- Pattern recommendation based on project characteristics

**Enhanced Search**:
- Vector embeddings for semantic search
- Cross-file relationship queries (call graphs)
- Pattern similarity detection

**Session Intelligence**:
- Automatic fact extraction from session files
- Prolog-based logical inference
- Cross-session pattern mining

**Additional Languages**:
- Zig (systems programming)
- Swift (iOS/macOS)
- Kotlin (Android/JVM)

### Long-Term Vision (6-12 months)

**Cross-Project Pattern Discovery**:
- Identify patterns used across multiple projects
- Automatic promotion: Project → Topic → Core
- Pattern effectiveness metrics

**Distributed Knowledge**:
- Sync patterns across machines (Automerge CRDT)
- Team-wide pattern sharing
- Cross-organization pattern libraries

**AI Agent Integration**:
- Agent-driven testing (create workspace, write test, fix failures, iterate)
- Autonomous pattern extraction from successful code
- Self-improving documentation

**Pattern Marketplace**:
- Share patterns with community
- Discover patterns from others
- Rate and review pattern effectiveness

**Advanced Analysis**:
- API compatibility checking
- Doc drift detection (docs vs implementation)
- Security pattern enforcement

### Open Questions

**Architecture**:
- Should workspace agent be rebuilt? With what technology?
- MCP vs HTTP for LLM integration?
- Native support for Claude Agent SDK?

**Features**:
- Video/audio session capture (pair programming sessions)?
- Pattern A/B testing (which approach works better)?
- Automatic pattern deprecation (detect when patterns stop being used)?

**Scaling**:
- Multi-project workspaces?
- Organization-level pattern repositories?
- Pattern versioning and compatibility?

---

## Project Structure

```
patina/
├── src/                          # Rust source (15,280 lines as of Oct 2025)
│   ├── main.rs                   # CLI entry point
│   ├── lib.rs                    # Public API
│   ├── adapters/                 # LLM adapters
│   │   ├── mod.rs                # Adapter trait
│   │   ├── claude/               # Claude adapter (mature)
│   │   │   ├── mod.rs            # Public interface (120 lines)
│   │   │   └── internal/         # Implementation
│   │   └── gemini/               # Gemini adapter (basic)
│   ├── commands/                 # CLI commands (40 files)
│   │   ├── init/                 # Project initialization
│   │   ├── scrape/               # Semantic indexing
│   │   │   ├── code.rs           # Code extraction
│   │   │   ├── docs.rs           # Documentation extraction
│   │   │   └── pdf.rs            # PDF extraction
│   │   ├── ask/                  # Pattern search
│   │   ├── doctor.rs             # Health checks
│   │   ├── build.rs              # Docker builds
│   │   ├── test.rs               # Container tests
│   │   ├── version.rs            # Version info
│   │   └── upgrade.rs            # Update checks
│   ├── layer/                    # Pattern storage
│   │   └── mod.rs                # Layer management
│   ├── session.rs                # Session tracking
│   ├── environment.rs            # Tool detection
│   ├── dev_env/                  # Development environments
│   │   ├── docker.rs             # Docker integration
│   │   └── mod.rs
│   ├── git/                      # Git operations
│   │   ├── fork.rs               # Fork management
│   │   └── mod.rs                # Git commands
│   └── version.rs                # Version constants
│
├── patina-metal/                 # Parser subsystem (workspace package)
│   ├── src/
│   │   ├── lib.rs                # Unified Analyzer API
│   │   ├── metal.rs              # Language enum
│   │   ├── grammars.rs           # Tree-sitter FFI
│   │   ├── parser.rs             # Parser wrapper
│   │   └── extractors/           # Language-specific extractors
│   │       ├── rust.rs           # Rust extraction
│   │       ├── go.rs             # Go extraction
│   │       ├── solidity.rs       # Solidity extraction
│   │       ├── python.rs         # Python extraction
│   │       ├── javascript.rs     # JavaScript extraction
│   │       ├── typescript.rs     # TypeScript extraction
│   │       ├── c.rs              # C extraction
│   │       ├── cpp.rs            # C++ extraction
│   │       └── cairo.rs          # Cairo extraction (native parser)
│   ├── grammars/                 # Git submodules (tree-sitter grammars)
│   │   ├── rust/
│   │   ├── go/
│   │   ├── solidity/
│   │   ├── python/
│   │   ├── javascript/
│   │   ├── typescript/
│   │   ├── c/
│   │   └── cpp/
│   ├── build.rs                  # Compiles C parsers
│   └── Cargo.toml                # Dependencies (tree-sitter, cairo-lang-parser)
│
├── layer/                        # Knowledge storage (file-based)
│   ├── core/                     # Eternal patterns (6 patterns)
│   │   ├── dependable-rust.md
│   │   ├── adapter-pattern.md
│   │   ├── unix-philosophy.md
│   │   ├── oxidized-knowledge.md
│   │   ├── safety-boundaries.md
│   │   └── session-capture.md
│   ├── surface/                  # Active patterns (53+ docs)
│   │   ├── pattern-selection-framework.md
│   │   ├── modular-architecture-plan.md
│   │   ├── patina-metal-explained.md
│   │   ├── git-aware-navigation-design.md
│   │   ├── architecture/
│   │   ├── commands/
│   │   └── ...
│   ├── dust/                     # Historical & reference repos
│   │   ├── repos/                # External repos for learning (12+)
│   │   │   ├── dagger/
│   │   │   ├── duckdb/
│   │   │   ├── cuti/
│   │   │   └── .patina-update.log
│   │   └── deprecated/           # Removed features
│   ├── sessions/                 # Session logs (227+ files)
│   │   ├── 20251007-210232.md
│   │   ├── 20251010-123308.md
│   │   ├── active-session.md     # Working file
│   │   └── ...
│   └── buckets/                  # Experimental: Domain knowledge
│       └── patina-dev/           # Neuro-symbolic POC
│           ├── facts.pl          # Prolog facts
│           ├── rules.pl          # Inference rules
│           └── queries.md        # Example queries
│
├── resources/                    # Templates and scripts
│   ├── claude/                   # Claude adapter resources
│   │   ├── .claude/
│   │   │   └── bin/              # Session scripts
│   │   │       ├── session-git-start.sh
│   │   │       ├── session-git-update.sh
│   │   │       ├── session-git-note.sh
│   │   │       ├── session-git-end.sh
│   │   │       └── launch.sh
│   │   └── launch.sh
│   ├── gemini/                   # Gemini adapter resources (minimal)
│   ├── git/                      # Git workflow scripts
│   │   └── pre-push-checks.sh    # CI checks (fmt, clippy, test)
│   ├── scripts/                  # Utility scripts
│   └── templates/                # Project templates
│       └── docker/               # Dockerfile templates
│
├── .github/                      # GitHub workflows
│   └── workflows/
│       ├── ci.yml                # CI pipeline (fmt, clippy, test)
│       └── release.yml           # Release automation
│
├── Cargo.toml                    # Workspace config
├── PROJECT_DESIGN.toml           # Architecture documentation
├── CLAUDE.md                     # Project instructions (for Claude)
├── README.md                     # User documentation
└── .gitignore
```

**Key Metrics** (October 2025):
- Rust source: ~15,280 lines (core)
- Commands: 40 files
- Layer patterns: 59+ documents (6 core, 53+ surface)
- Sessions captured: 227+ files
- Languages supported: 9 (all production-ready)
- Reference repos: 12+ tracked

---

## Testing & CI

### Testing Strategy

**Unit Tests**: Colocated with code
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Test internal implementation
    }
}
```

**Integration Tests**: `tests/` directory
- Test public API only
- Black-box testing of commands
- Use tempfile for isolation

**Patina-Metal Tests**:
- Validated on real repositories
- 12+ reference repos scraped successfully
- Language extractors tested against 1000+ files each

### CI Pipeline

**GitHub Actions** (`.github/workflows/ci.yml`):
```yaml
- cargo fmt --all --check      # Formatting
- cargo clippy --workspace     # Linting (zero warnings)
- cargo test --workspace       # All tests must pass
- cargo build --release        # Release build
```

**Pre-Push Hook** (`resources/git/pre-push-checks.sh`):
- Runs same checks locally
- Prevents broken commits reaching CI
- Can be bypassed with `git push --no-verify` (escape hatch)

**Branch Protection**:
- All CI checks must pass
- Reviews required for main/patina branches
- Force push disabled

### Test Coverage

**Well-Tested**:
- Core layer system
- SQLite database operations
- Pattern file parsing
- Git integration
- Command parsing (clap)

**Needs More Tests**:
- LLM adapter implementations (mocking required)
- Session distillation logic
- YOLO devcontainer generator
- Reference repo management

---

## Dependencies & Rationale

### Core Dependencies (from Cargo.toml)

| Dependency | Version | Purpose | Why This One? |
|------------|---------|---------|---------------|
| **clap** | 4.5 | CLI parsing | Derive macros, excellent UX, industry standard |
| **anyhow** | 1.0 | Error handling | Simple, context-friendly errors |
| **serde** | 1.0 | Serialization | De facto standard, derive macros |
| **toml** | 0.8 | Config parsing | Human-friendly, comments supported |
| **rusqlite** | 0.32 | SQLite database | Mature, bundled option, synchronous |
| **rayon** | 1.10 | Parallelism | Data parallelism without async |
| **reqwest** | 0.12 | HTTP client | Blocking mode, no async needed |
| **chrono** | 0.4 | Time handling | Full-featured, timezone support |
| **uuid** | 1.10 | Unique IDs | Standard, fast |
| **dirs** | 5.0 | System paths | Cross-platform home dirs |
| **which** | 7.0 | Tool detection | Cross-platform `which` command |
| **notify** | 6.1 | File watching | Cross-platform file events |
| **colored** | 2 | Terminal colors | Simple, works everywhere |
| **automerge** | 0.5 | CRDT | Future distributed features |
| **tree-sitter** | 0.24 | Parsing | Multi-language AST parser |
| **walkdir** | 2 | Directory traversal | Respects symlinks |
| **ignore** | 0.4 | Gitignore support | Same crate as ripgrep |

### Patina-Metal Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| **tree-sitter** | 0.24 | Core parsing library |
| **streaming-iterator** | 0.1 | Efficient AST traversal |
| **cairo-lang-parser** | 2.12 | Native Cairo parser |
| **cairo-lang-syntax** | 2.12 | Cairo AST |
| **cairo-lang-filesystem** | 2.12 | Cairo file handling |
| **cc** | 1.0 | Compile C parsers (build.rs) |

### Dependency Philosophy

**Prefer**:
- Mature crates (1.0+)
- Small, focused crates
- Synchronous APIs
- Zero or minimal dependencies themselves
- Permissive licenses (MIT/Apache-2.0)

**Avoid**:
- Async when not needed
- Heavy frameworks
- Unstable APIs (0.x without good reason)
- Crates with many dependencies
- GPL-licensed crates (distribution issues)

**From CLAUDE.md**:
> "Rust for CLI and core logic - let the compiler be your guard rail. No tokio/async - use rayon for parallelism."

---

## Deployment & Installation

### Installation

**From crates.io** (when published):
```bash
cargo install patina
```

**From source**:
```bash
git clone --recursive https://github.com/NicabarNimble/patina
cd patina
cargo build --release
cargo install --path .
```

**Note**: `--recursive` required for patina-metal grammar submodules.

### Binary Distribution

**Target**: Single binary, no runtime dependencies

**Platforms**:
- macOS (aarch64, x86_64)
- Linux (x86_64, aarch64)
- Windows (x86_64) - untested but should work

**Size**: ~10-15 MB (with bundled SQLite and tree-sitter grammars)

### Configuration

**System-wide**: `~/.patina/`
```
~/.patina/
├── config.toml           # Global config (future)
└── claude-linux/         # Claude credentials (if YOLO used)
    └── .credentials.json
```

**Per-project**: Project root
```
project/
├── .patina/
│   ├── config.toml       # Project config
│   └── knowledge.db      # Semantic database
├── layer/                # Patterns (git-tracked)
├── .claude/              # Claude adapter (gitignored)
└── PROJECT_DESIGN.toml   # Design doc (git-tracked)
```

### Upgrade Process

**Check for updates**:
```bash
patina upgrade --check
```

**Manual upgrade** (from source):
```bash
cd patina
git pull
cargo build --release
cargo install --path .
```

**Re-initialize project** (update adapter files):
```bash
patina init .
```

---

## Design Principles (Summary)

1. **One Tool, One Job**: Unix philosophy applied to Rust modules
2. **Context is King**: Every feature serves context management
3. **User Patterns First**: Respect existing workflows, provide escape hatches
4. **Knowledge Compounds**: Every project makes future projects smarter
5. **Tool-Based Decomposition**: Build tools, not systems (LLMs excel at tools)
6. **Git as Memory**: Use version control for pattern evolution tracking
7. **Information Over Automation**: Show users information, let them decide
8. **Dependable Boundaries**: Small public interfaces, unlimited internal complexity
9. **Token Efficiency**: Compress information for LLM context windows
10. **Escape Hatches Everywhere**: Never lock users into specific workflows

**From PROJECT_DESIGN.toml**:
> "Help build systems with LLMs by maintaining context and decomposing into tools. Pattern-selection-framework guides architecture: Tools not Systems."

---

## Conclusion

Patina is a **production-ready context orchestration system** that solves the fundamental challenge of AI-assisted development. Through systematic session capture, semantic code analysis, pattern evolution tracking, and LLM-agnostic adapters, it enables developers to build and maintain complex projects with AI assistance that gets smarter over time.

### What Makes Patina Different

**Not a Code Generator**: Patina doesn't generate code. It provides context.

**Not an IDE**: Patina works with your tools (VSCode, vim, whatever).

**Not a Workflow Manager**: Patina doesn't enforce processes. It captures decisions.

**Not LLM-Specific**: Patina works with Claude, Gemini, future models via adapters.

### The Patina Metaphor

Like the patina that forms on copper over time:
- **Protective**: Guards against forgetting context
- **Accumulative**: Grows richer with each session
- **Revealing**: Shows the history beneath
- **Beautiful**: Makes projects more valuable over time

### Current State (October 2025)

- **Version**: 0.1.0 (experimental but stable)
- **Codebase**: 15,280+ lines of Rust
- **Languages Supported**: 9 (all production-ready)
- **Sessions Captured**: 227+
- **Patterns Documented**: 59+
- **Reference Repos**: 12+

### What's Next

- YOLO command maturation (merge from feature branch)
- Enhanced pattern success metrics
- Neuro-symbolic inference automation
- Additional language support
- Cross-project pattern discovery

---

## References

### Core Design Documents

- `PROJECT_DESIGN.toml` - System architecture and philosophy
- `layer/core/dependable-rust.md` - Module structure pattern
- `layer/core/adapter-pattern.md` - LLM adapter design
- `layer/surface/pattern-selection-framework.md` - When to use which pattern
- `layer/surface/modular-architecture-plan.md` - Tool-based decomposition
- `layer/surface/patina-metal-explained.md` - Parser subsystem

### Session History

Key sessions documenting major decisions:
- `session_summary_july27_full.md` - Brain → Layer rename
- `20251002-055812.md` - Agent-driven testing, workspace review
- `20251007-210232.md` - Security review, credential management
- `20251009-064522.md` - GitHub integration, branching strategy
- `20251010-123308.md` - Neuro-symbolic exploration

### External Influences

- **Container Use Pattern**: Isolated workspaces via containers
- **Eskil Steenberg**: "Black box" module design philosophy
- **No Boilerplate**: Synchronous Rust over async complexity
- **Unix Philosophy**: Do one thing well, compose tools

---

**Document Metadata**:
- **Created**: 2025-10-13
- **Sources**: 227+ session files, 62 Rust source files, 59+ pattern documents, git history Aug-Oct 2025
- **Scope**: Comprehensive system architecture for Patina v0.1.0
- **Audience**: Maintainers, contributors, users wanting deep understanding
- **Status**: Living document, update as system evolves

**Last Updated**: 2025-10-13 by Claude Code (via comprehensive codebase analysis)
