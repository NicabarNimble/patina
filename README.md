# Patina

> Context orchestration for AI-assisted development

Patina accumulates development wisdom like the protective layer that forms on metal—your patterns, decisions, and insights build up over time and transfer between projects.

## What is Patina?

Patina solves the fundamental problem of AI-assisted development: constantly re-teaching AI assistants about your project's context, patterns, and constraints.

**Core idea**: Your development knowledge compounds. Session insights become observations, observations train embeddings, embeddings enable smarter retrieval.

## Features (v0.1.0)

| Feature | Description |
|---------|-------------|
| **Unified Eventlog** | Code AST, git history, sessions → single patina.db |
| **Multi-Dimension Search** | Semantic, temporal, dependency projections |
| **Cross-Project Knowledge** | Query external repos via `~/.patina/repos/` |
| **GitHub Integration** | Index issues with bounty detection |
| **Mothership Daemon** | `patina serve` for container queries |
| **YOLO Devcontainers** | AI-ready development environments |
| **LLM Adapters** | Claude and Gemini integration |

## Quick Start

```bash
# Install
cargo install --path .

# Initialize project with Claude adapter
patina init . --llm=claude

# Build knowledge database (code + git + sessions)
patina scrape

# Train embedding projections
patina oxidize

# Search your codebase
patina scry "error handling patterns"    # Semantic search
patina scry "find spawn_entity"          # Exact match (FTS5)
patina scry --file src/main.rs           # Temporal: co-changing files

# Query external repos
patina repo dojoengine/dojo              # Clone + index
patina scry "spawn" --repo dojo          # Search it

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

patina oxidize                      # Train projections from .patina/oxidize.yaml
```

Supported languages: Rust, TypeScript, JavaScript, Python, Go, C, C++, Solidity, Cairo

### Search (scry)
```bash
patina scry "error handling"                       # Semantic search
patina scry "find MyClass::new"                    # FTS5 exact match (auto-detected)
patina scry --file src/auth.rs                     # Temporal: what files co-change?
patina scry --dimension dependency "execute"       # Call graph relationships
patina scry --repo dojo "spawn"                    # Query external repo
patina scry --include-issues "bounty"              # Include GitHub issues
```

### Cross-Project Knowledge
```bash
patina repo dojoengine/dojo              # Clone + scrape to ~/.patina/repos/
patina repo add <url> --with-issues      # Also fetch GitHub issues
patina repo list                         # Show registered repos
patina repo update dojo                  # Git pull + rescrape
patina repo rm dojo                      # Remove repo
```

### Mothership Daemon
```bash
patina serve                             # Start on localhost:50051
patina serve --host 0.0.0.0              # Bind all interfaces (for containers)
curl http://localhost:50051/health       # Health check
```

### Project Setup
```bash
patina init . --llm=claude         # Initialize with Claude adapter
patina doctor                      # Check project health
patina version                     # Show version info
```

### Development Environment
```bash
patina yolo                         # Generate AI-ready devcontainer
patina yolo --with foundry,cairo    # Add specific tools
patina build                        # Docker containerized build
patina test                         # Run tests in container
```

### Session Management (Claude Adapter)

Within Claude, use these slash commands:
- `/session-start <name>` - Begin session with Git tracking
- `/session-update` - Capture progress
- `/session-note <insight>` - Record insight
- `/session-end` - Archive and distill learnings

## Command Reference

Patina has 23 commands totaling ~42k lines of Rust. Here's the full inventory:

### Active Commands

| Command | Lines | Module | Description |
|---------|------:|--------|-------------|
| `secrets` | 2,100 | `secrets/` | Age encryption, Touch ID, multi-recipient vaults |
| `adapter` | 1,900 | `adapters/` | LLM frontend management (Claude, Gemini) |
| `scrape` | 1,900 | `storage/`, `git/` | Extract code, git history, sessions to SQLite |
| `serve` | 1,600 | `mcp/` | MCP server for LLM tool integration |
| `scry` | 1,100 | `retrieval/` | Hybrid search (semantic + lexical + temporal) |
| `oxidize` | 900 | `embeddings/` | Build vector embeddings from scraped data |
| `init` | 800 | `project/` | Initialize project with LLM adapter |
| `model` | 800 | `models/` | Manage embedding models in mothership cache |
| `doctor` | 600 | `commands/` | Health check and diagnostics |
| `persona` | 300 | `retrieval/` | Cross-project user knowledge |
| `assay` | 200 | `retrieval/` | Structural queries (imports, callers, inventory) |
| `repo` | 200 | `mothership/` | Register external repos for cross-project search |
| `upgrade` | 160 | `commands/` | Check for new CLI versions |
| `rebuild` | 100 | `main.rs` | Rebuild .patina/ from git-tracked sources |

### Measurement Tools

| Command | Lines | Purpose |
|---------|------:|---------|
| `eval` | 200 | Retrieval quality evaluation |
| `bench` | 200 | Benchmarking with ground truth |

### Niche / Legacy

| Command | Lines | Status | Notes |
|---------|------:|--------|-------|
| `yolo` | 220 | Niche | Devcontainer generation |
| `query` | 460 | Superseded | Use `scry` |
| `ask` | 100 | Superseded | Use `scry` |
| `embeddings` | - | Superseded | Use `oxidize` |
| `belief` | 450 | Experimental | Neuro-symbolic, unused |
| `build` | 30 | Stub | Docker wrapper |
| `test` | 30 | Stub | Docker wrapper |

### Codebase Summary

| Category | Lines |
|----------|------:|
| Active commands | ~10,700 |
| Measurement tools | ~400 |
| Niche/legacy | ~2,200 |
| Shared infrastructure | ~28,500 |
| **Total** | **~41,800** |

## Architecture

```
patina/
├── src/
│   ├── commands/
│   │   ├── scrape/        # Code, git, sessions, GitHub extraction
│   │   ├── oxidize/       # MLP training (semantic, temporal, dependency)
│   │   ├── scry/          # Unified query interface
│   │   ├── repo/          # Cross-project knowledge
│   │   └── serve/         # Mothership HTTP daemon
│   ├── embeddings/        # ONNX E5-base-v2 embeddings
│   └── reasoning/         # Embedded Prolog for belief validation
├── layer/
│   ├── core/              # Eternal principles, build.md roadmap
│   ├── surface/           # Specs, design docs
│   └── sessions/          # Session archives (Git-tracked)
├── .patina/
│   ├── data/
│   │   ├── patina.db      # Unified eventlog + materialized views
│   │   └── embeddings/e5-base-v2/projections/
│   │       ├── semantic.safetensors   # Trained MLP weights
│   │       ├── semantic.usearch       # HNSW vector index
│   │       ├── temporal.*
│   │       └── dependency.*
│   └── oxidize.yaml       # Projection training recipe
└── ~/.patina/
    ├── repos/             # External repos (cross-project knowledge)
    └── registry.yaml      # Repo registry
```

### Data Flow

```
Sources                    Scrape              Oxidize              Query
───────                    ──────              ───────              ─────
.git/commits          →    patina.db     →    Training pairs   →   scry
src/**/* (AST)        →    ├── eventlog  →    E5 embedding     →   ├── semantic
layer/sessions/*.md   →    ├── call_graph →   MLP projection   →   ├── temporal
GitHub issues         →    ├── co_changes→    USearch HNSW     →   └── dependency
                           └── code_fts       (.usearch files)
```

### Embedding Model

Patina uses **E5-base-v2** (768-dim) with trained MLP projections per dimension.

Configure in `.patina/config.toml`:
```toml
[embeddings]
model = "e5-base-v2"
```

## Design Principles

- **Pure Rust**: No Python subprocess dependencies (ONNX Runtime via `ort` crate)
- **No async**: Blocking I/O with rayon for parallelism, rouille for HTTP server
- **Local-first**: SQLite + USearch, no cloud services required
- **LLM-agnostic**: Adapter pattern for Claude, Gemini, etc.
- **Git as memory**: Sessions committed to repo, vectors rebuildable from source

## Multidimensional Embeddings

Patina uses **separate dimension projections** rather than a single embedding space. Each dimension is an independent 256-dim projection trained on different relationship signals:

| Dimension | Training Signal | Status | Query Interface |
|-----------|-----------------|--------|-----------------|
| Semantic | Same session = related | ✅ Done | `scry "query"` (text→concepts) |
| Temporal | Same commit = related | ✅ Done | `scry --file src/foo.rs` (file→co-changers) |
| Dependency | Caller/callee = related | ✅ Done | `scry --dimension dependency` (func→call graph) |
| Syntactic | AST similarity | Future | Similar code structure |
| Architectural | Same module = related | Future | Position in system |
| Social | Same author = related | Skipped | Single-user, not valuable |

**Architecture:** E5-base-v2 (768-dim) → trained MLP (768→1024→256) → USearch HNSW index per dimension.

### Roadmap: Progressive Adapters

The vision: **One engine, variable patina thickness**.

### Patina Thickness Model

Same architecture, progressively richer training data:

- **Fresh patina**: Git + code only (structural understanding)
- **Working patina**: + sessions (patterns emerging)
- **Mature patina**: Deep session integration (contextual wisdom)

See `layer/surface/patina-embedding-architecture.md` for full design.

## Requirements

- Rust 1.70+
- Git
- Docker (optional, for `patina yolo`, `patina build`, `patina test`)
- `gh` CLI (optional, for `--with-issues` GitHub integration)

## Development

```bash
# Build release
cargo build --release

# Run tests
cargo test --workspace

# Pre-push checks
./resources/git/pre-push-checks.sh
```

## License

MIT License - see [LICENSE](LICENSE) file for details.
