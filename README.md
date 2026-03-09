# Patina

> Context orchestration for AI-assisted development

Patina accumulates development wisdom like the protective layer that forms on metal—your patterns, decisions, and insights build up over time and transfer between projects.

## What is Patina?

Patina solves the fundamental problem of AI-assisted development: constantly re-teaching AI assistants about your project's context, patterns, and constraints.

**Core idea**: Your development knowledge compounds. Session insights become beliefs, beliefs ground retrieval, retrieval enables smarter AI assistance.

## Features (v0.17.0)

| Feature | Description |
|---------|-------------|
| **Semantic Search** | Vector search over code, commits, beliefs, and patterns via `scry` |
| **Structural Queries** | Module inventory, call graph, imports, co-change analysis via `assay` |
| **Epistemic Beliefs** | Capture project decisions with evidence, grounding, and attack/support relationships |
| **WASM Plugins** | Extend Patina with WebAssembly component model plugins |
| **Mother Daemon** | Cross-project knowledge, hot model caching, graph routing |
| **MCP Integration** | Model Context Protocol server for LLM tool integration |
| **Cross-Project Knowledge** | Query external repos and persona knowledge across projects |
| **Spec-Driven Development** | Lifecycle management for specs (draft → ready → active → complete) |
| **Session Tracking** | Git-integrated development sessions with tagging and classification |
| **Secrets Management** | Age encryption with Touch ID, multi-recipient vaults |
| **LLM Adapters** | Claude, Gemini, and OpenCode integration |
| **YOLO Devcontainers** | AI-ready development environments |

## Quick Start

```bash
# Install
cargo install --path .

# Initialize project
patina init .

# Add an LLM adapter
patina adapter add claude

# Build knowledge database (code + git + sessions)
patina scrape

# Build embedding projections
patina oxidize

# Search your codebase
patina scry "error handling patterns"        # Semantic search
patina scry --file src/main.rs               # Temporal: co-changing files
patina scry --belief measure-first           # Belief grounding: find related code

# Structural queries
patina assay                                 # Module inventory
patina assay search "plugin engine"          # FTS5 text search
patina assay cochange src/main.rs            # Co-change analysis

# Get project patterns and beliefs
patina context                               # Core + surface patterns
patina context --topic "error handling"      # Focused on a topic

# Query external repos
patina repo dojoengine/dojo                  # Clone + index
patina scry "spawn" --repo dojo              # Search it

# Check project health
patina doctor
```

## Commands

### Knowledge Pipeline
```bash
patina scrape                       # Run all scrapers (code + git + sessions)
patina scrape code                  # Extract AST, call graph, symbols
patina scrape git                   # Extract commits, file co-changes
patina scrape sessions              # Extract session observations

patina oxidize                      # Build vector embeddings from scraped data
patina rebuild                      # Rebuild .patina/ from git-tracked sources
```

Supported languages: Rust, TypeScript, JavaScript, Python, Go, C, C++, Solidity, Cairo

### Search & Retrieval

**Semantic search (scry)** — vector similarity over beliefs, patterns, commits, and code:
```bash
patina scry "error handling"                       # Semantic search
patina scry --file src/auth.rs                     # Temporal: what files co-change?
patina scry --belief eventlog-is-truth             # Belief grounding
patina scry --repo dojo "spawn"                    # Query external repo
patina scry --include-issues "bounty"              # Include GitHub issues
patina scry orient src/commands/                   # Structural orientation
patina scry recent --days 7                        # Recent changes
```

**Structural queries (assay)** — exact queries over code structure:
```bash
patina assay                                       # Module inventory
patina assay imports src/main.rs                   # What does main.rs import?
patina assay callers "find_project_root"            # Who calls this function?
patina assay cochange src/plugin/mod.rs            # Files that co-change
patina assay search "embedding model"              # FTS5 ranked text search
patina assay belief eventlog-is-truth              # Evidence for a belief
patina assay derive                                # Compute structural signals
```

**Context** — patterns and conventions for AI assistants:
```bash
patina context                                     # Core + surface patterns + beliefs
patina context --topic "testing"                   # Focused retrieval
```

### Epistemic Beliefs
```bash
patina belief audit                 # Show all beliefs with use/truth metrics
patina scry --belief <id>           # Ground a belief against code
patina assay belief <id>            # Find evidence for a belief
```

### Cross-Project Knowledge
```bash
patina repo dojoengine/dojo              # Clone + scrape to ~/.patina/repos/
patina repo list                         # Show registered repos
patina repo update dojo                  # Git pull + rescrape
patina repo remove dojo                  # Remove repo

patina persona note "prefers explicit error handling"  # Capture user knowledge
patina persona query "error handling"                  # Search persona knowledge
```

### Mother Daemon
```bash
patina mother start                      # Start the daemon
patina mother status                     # Show daemon status
patina mother graph                      # Graph operations
```

### WASM Plugins
```bash
patina plugin list                       # List installed plugins
```

Plugins use the WebAssembly Component Model (WIT interfaces). Patina ships with `patina-doctor` as a WASM plugin with a compiled fallback.

### Session Management
```bash
patina session start "feature name"      # Start session with git tagging
patina session update                    # Record progress with git metrics
patina session note "important insight"  # Add timestamped note
patina session end                       # Archive, classify, distill
```

Within Claude, use slash commands: `/session-start`, `/session-update`, `/session-note`, `/session-end`

### Spec Lifecycle
```bash
patina spec list                         # List all specs
patina spec ready                        # Show unblocked specs ready to work
patina spec blocked                      # Show blocked specs
patina spec status <name> active         # Update status (draft → ready → active → complete)
patina spec archive <name>               # Archive completed spec
```

### Versioning & Release
```bash
patina version                           # Show current version + ready specs
patina version --components              # Show component versions
patina version hotfix                    # Emergency patch bump
patina spec status <name> complete       # Completing a spec triggers release
```

### Secrets
```bash
patina secrets add API_KEY               # Add secret to vault
patina secrets run -- ./my-script.sh     # Inject secrets into environment
patina secrets check                     # Scan staged files for exposed secrets
patina secrets audit                     # Scan all tracked files
```

### Project Setup
```bash
patina init .                      # Initialize project
patina adapter add claude          # Add LLM adapter
patina doctor                      # Check project health
patina upgrade                     # Check for new CLI versions
patina model list                  # List available embedding models
```

### Development Environment
```bash
patina yolo                         # Generate AI-ready devcontainer
patina yolo --with foundry,cairo    # Add specific tools
```

### Measurement
```bash
patina eval                         # Evaluate retrieval quality
patina bench                        # Benchmark with ground truth
patina report                       # Generate project state report
```

## Architecture

```
patina/
├── src/                        # Rust source (~61k lines, 195 files)
│   ├── commands/               # CLI commands (27 total)
│   │   ├── scrape/             # Code, git, sessions, GitHub extraction
│   │   ├── scry/               # Semantic vector search
│   │   ├── assay/              # Structural queries (imports, callers, co-change)
│   │   ├── oxidize/            # Embedding projection training
│   │   ├── mother/             # Daemon (microserver, registry, graph)
│   │   ├── session/            # Git-integrated session management
│   │   ├── repo/               # Cross-project knowledge
│   │   ├── spec/               # Spec lifecycle management
│   │   ├── version/            # Versioning and release
│   │   ├── belief/             # Epistemic belief audit
│   │   ├── persona/            # Cross-project user knowledge
│   │   └── ...                 # init, doctor, secrets, yolo, eval, bench, etc.
│   ├── plugin/                 # WASM plugin engine (wasmtime)
│   ├── mother/                 # Daemon core (graph, children)
│   ├── retrieval/              # Oracle abstraction, RRF fusion
│   ├── mcp/                    # MCP protocol, JSON-RPC server
│   ├── embeddings/             # ONNX E5-base-v2, USearch HNSW
│   ├── release/                # Release strategy and version bumping
│   ├── secrets/                # Age encryption, Keychain integration
│   ├── adapters/               # LLM adapters (Claude, Gemini, OpenCode)
│   └── ...                     # db, forge, git, models, scanner, workspace
├── grammars/                   # Grammar WASM plugins (9 languages)
├── plugins/                    # Workspace plugin crates (sdk, doctor, models, repos)
├── layer/                      # Pattern storage (Git as memory)
│   ├── core/                   # Eternal principles + core beliefs
│   ├── surface/                # Active specs, architecture docs
│   │   ├── epistemic/beliefs/  # 103 epistemic beliefs
│   │   └── build/              # Specs and roadmap
│   ├── dust/                   # Archived patterns
│   └── sessions/               # 657 session archives
├── .patina/
│   ├── data/
│   │   ├── patina.db           # Unified eventlog + materialized views
│   │   └── embeddings/e5-base-v2/projections/
│   │       ├── semantic.*      # Trained MLP weights + HNSW index
│   │       ├── temporal.*
│   │       └── dependency.*
│   └── oxidize.yaml            # Projection training recipe
└── ~/.patina/
    ├── repos/                  # External repos (cross-project knowledge)
    ├── layer/                  # User-level beliefs and persona
    └── registry.yaml           # Repo registry
```

### Data Flow

```
Sources                    Scrape              Oxidize              Query
───────                    ──────              ───────              ─────
.git/commits          →    patina.db     →    Training pairs   →   scry (semantic)
src/**/* (AST)        →    ├── eventlog  →    E5 embedding     →   assay (structural)
layer/sessions/*.md   →    ├── call_graph →   MLP projection   →   context (patterns)
layer/**/beliefs/*.md →    ├── co_changes→    USearch HNSW     →   belief audit
GitHub issues         →    └── code_fts       (.usearch files)
```

### Embedding Model

Patina uses **E5-base-v2** (768-dim) with trained MLP projections per dimension.

| Dimension | Training Signal | Query Interface |
|-----------|-----------------|-----------------|
| Semantic | Same session = related | `scry "query"` (text → concepts) |
| Temporal | Same commit = related | `scry --file src/foo.rs` (file → co-changers) |
| Dependency | Caller/callee = related | `assay callers/callees` (call graph) |

**Architecture:** E5-base-v2 (768-dim) → trained MLP (768→1024→256) → USearch HNSW index per dimension.

## Design Principles

- **Pure Rust**: No Python subprocess dependencies (ONNX Runtime via `ort` crate)
- **Local-first**: SQLite + USearch, no cloud services required
- **LLM-agnostic**: Adapter pattern for Claude, Gemini, OpenCode
- **Git as memory**: Sessions and beliefs committed to repo, vectors rebuildable from source
- **Spec-driven**: Features start as specs, go through lifecycle, trigger releases on completion

## Requirements

- Rust (edition 2021, tested on 1.90+)
- Git
- Docker (optional, for `patina yolo` devcontainer generation)
- `gh` CLI (optional, for GitHub authentication)

## Development

```bash
# Build and install
cargo build --release
cargo install --path .

# Run tests
cargo test --workspace

# Pre-push checks (fmt + clippy + tests)
./resources/git/pre-push-checks.sh
```

## License

MIT License - see [LICENSE](LICENSE) file for details.
