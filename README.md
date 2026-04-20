# Patina

> Context management for AI-assisted development

Patina is a local-first Rust CLI that turns a repository into a reusable knowledge system for humans and AI tools. It scrapes code, git history, layer artifacts, and optional external sources into a local store, builds semantic indices, and exposes commands for retrieval, structure queries, workflow tracking, and cross-project knowledge.

## What Patina Does

- Builds project memory from code, commits, specs, sessions, and beliefs
- Provides semantic search with `patina scry` and structural queries with `patina assay`
- Captures project rules and decisions through `patina context`, specs, and epistemic beliefs
- Supports cross-project knowledge via external repos, persona data, and the Mother daemon
- Extends the system with WASM children built on the WASI component model and Patina SDK

Patina is designed around a simple idea: project knowledge should compound instead of being re-explained every session.

## Quick Start

### Install options

Supported targets: macOS 14+ (Apple Silicon) and Linux x86_64.

```bash
# Homebrew (macOS 14+ Apple Silicon)
brew tap NicabarNimble/tap
brew install patina

# curl installer (stable channel default)
curl -fsSL https://raw.githubusercontent.com/NicabarNimble/patina/main/install.sh | bash

# Source install (developer path)
cargo install --path .
```

### Release channels (curl installer)

```bash
# Stable (default)
curl -fsSL https://raw.githubusercontent.com/NicabarNimble/patina/main/install.sh | bash

# Beta (moving tag `beta`)
curl -fsSL https://raw.githubusercontent.com/NicabarNimble/patina/main/install.sh | bash -s -- --channel beta

# Nightly (moving tag `nightly`)
curl -fsSL https://raw.githubusercontent.com/NicabarNimble/patina/main/install.sh | bash -s -- --channel nightly

# Pin explicit version
curl -fsSL https://raw.githubusercontent.com/NicabarNimble/patina/main/install.sh | bash -s -- --version 0.62.1
```

### First run

```bash
# First-run setup
patina setup grammars

# Initialize a project
patina init .

# Allow an AI interface in this repo
patina interface add claude

# Start the Mother daemon
patina mother start

# Build the local knowledge base
patina scrape
patina oxidize

# Query it
patina scry "error handling patterns"
patina assay search "child engine"
patina context --topic "testing"
```

### Managed daemon (Homebrew)

```bash
brew services start patina
brew services status patina
brew services restart patina
```

Homebrew users should prefer `brew services` over manual `nohup` process management.

Once configured, running bare `patina` launches the default interface.

## Core Workflow

```bash
# Refresh knowledge after changes
patina scrape
patina oxidize

# Semantic retrieval
patina scry "release automation"
patina scry --file src/main.rs
patina scry --belief git-as-memory
patina scry recent --days 7

# Structural and factual retrieval
patina assay
patina assay imports src/main.rs
patina assay callers find_project_root
patina assay cochange src/commands/spec/mod.rs

# Project guidance for AI work
patina context
patina belief audit
```

## Command Guide

### Retrieval and analysis

```bash
patina scry "query"                 # Semantic search over code, beliefs, patterns, commits
patina scry --all-repos "query"     # Search current project + registered repos
patina scry --detail <query-id>      # Fetch full content for a previous result
patina assay                         # Module inventory
patina assay search "term"          # Ranked factual search via FTS5
patina assay belief <belief-id>      # Ground a belief against code and facts
patina context --topic "architecture"
patina report                        # Generate project state report
patina atlas --json                  # Spec + MCT visibility snapshot (Mother-backed when available)
patina atlas --html --output .tmp/atlas/spec-atlas.html
patina atlas --serve --port 7417     # Local fallback read-only dashboard server
patina measure --full                # Health view from measurement data
patina eval --combined               # Evaluate retrieval pipeline quality
patina bench retrieval               # Benchmark retrieval quality
```

### Knowledge pipeline and storage

```bash
patina scrape                        # Delta-driven scrape of git, code, layer, beliefs
patina scrape code                   # Re-extract code facts
patina scrape git                    # Re-extract commit and co-change data
patina scrape layer                  # Re-extract patterns, sessions, and specs
patina oxidize                       # Build embeddings and projections
patina rebuild                       # Rebuild local derived state from tracked sources
patina events export                 # Export runtime events to JSONL replica
patina events import layer/events.jsonl

# Operator retrieval path for Mother grant audit events
rg '"event_type":"mother.grant"' layer/events.jsonl
# (or) sqlite3 .patina/local/data/events.db "select seq,timestamp,data from eventlog where event_type='mother.grant' order by seq desc limit 20;"
```

### Project workflow

```bash
patina ai session new "feature boundary"
patina ai session update
patina ai session note "important insight"
patina ai session end

patina spec create feat feature-name
patina spec ready
patina spec promote feature-name
patina spec complete feature-name
patina spec next

patina version                       # Show current version and ready specs
patina version hotfix                # Patch bump without spec ceremony
patina upgrade --check               # Check for newer CLI release
```

### Cross-project knowledge and services

```bash
patina repo add owner/repo
patina repo list
patina repo update owner-repo

patina persona note "prefers explicit errors"
patina persona query "error handling"

patina connect github                # Connect GitHub via OAuth device flow
patina connect status

patina mother start
patina mother stop
patina mother status
patina mother install                # Install launchd supervisor (macOS, always-on)
patina mother search "belief query"
patina mother sources
patina mother graph                  # Cross-project relationship graph

patina pando list --json             # Includes first-party pandos (atlas, folder-text-to-parquet)
curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/atlas
curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/atlas/atlas.json

patina model list
patina model add e5-base-v2
```

### Setup, extension, and safety

```bash
patina interface list
patina interface add opencode

patina child list                    # Canonical (alias: `patina plugin list`)
patina child init my-child
patina child init my-grammar --world pipeline --legacy
patina schema new my-schema
patina schema build

patina secrets add API_KEY
patina secrets run -- ./script.sh
patina secrets audit

patina doctor
patina setup grammars
patina yolo
```

## Architecture

```text
Sources                         Storage / indices                        Query surface
-------                         -----------------                        -------------
git history                 ->  ~/.patina/mother/projects/{uid}/     ->  scry
code + grammar children     ->    patina.db (projections)            ->  assay
layer/ (specs, sessions,    ->    events.db (runtime events)         ->  context
beliefs, patterns)          ->    runtime.db (child state)           ->  belief / report / measure
external repos + sources    ->  .patina/local/cache/embeddings/      ->  eval / bench
```

Mother owns per-project databases in `~/.patina/mother/projects/{uid}/`. The CLI opens these local files directly for core operations — no daemon required for scrape, scry, or assay. Mother adds child orchestration, cross-project queries, secrets management, and session coordination.

Core ideas:

- `scry` is the semantic retrieval layer
- `assay` is the factual and structural retrieval layer
- `context` surfaces project rules, conventions, and beliefs for AI assistants
- `rebuild` restores derived state from tracked project artifacts
- Mother is the daemon — children are WASM components, pandos are composed groups of children that appear as user-facing products
- Children use the WASI component model with Patina toy interfaces for sandboxed capability access

## Repository Layout

```text
patina/
├── src/                      # Rust CLI, retrieval, storage, command surfaces
├── mother/                   # Mother daemon crate (state, registry, children)
├── grammars/                 # Grammar child sources used by scrape pipeline
├── crates/                   # Shared internal crates
├── children/                 # First-party child components (WASM)
├── wit/                      # WIT contracts
│   ├── toys/                 # Toy interfaces (per-capability packages)
│   ├── child/                # Child world composition
│   └── pipeline/             # Pipeline world composition
├── sdk/                      # Patina SDK (umbrella crate with inline toy types)
├── layer/                    # Git-tracked project memory (THE PRODUCT)
│   ├── core/                 # Stable values, beliefs, and principles
│   ├── surface/              # Active specs, beliefs, reports, architecture docs
│   ├── sessions/             # Session archives
│   └── dust/                 # Archived material
└── .patina/
    ├── config.toml           # Project config
    ├── uid                   # Stable 8-hex project identity
    ├── oxidize.yaml          # Embedding recipe
    └── local/
        └── cache/
            └── embeddings/   # Vector indices (working-copy-local, branch-specific)

~/.patina/
├── config.toml               # Global config
├── registry.yaml             # Project and repo registry
├── interfaces/               # Interface templates (claude, opencode, gemini)
├── cache/repos/              # Cloned reference repos
├── pipeline/                 # Installed grammar child artifacts
├── mother/
│   ├── state.db              # Mother lifecycle (sessions, project registry, graph)
│   ├── graph.db              # Cross-project belief graph
│   └── projects/{uid}/       # Per-project databases
│       ├── events.db         # Runtime events (irreplaceable, machine-local)
│       ├── patina.db         # Scrape projections (rebuildable)
│       └── runtime.db        # Child state for this project
├── persona/                  # Cross-project persona (identity, beliefs)
└── run/                      # Mother runtime files (pid, socket)
```

## Design Principles

- Pure Rust runtime; no Python subprocess dependency chain
- Local-first storage and rebuildable derived state
- Git as memory for durable project knowledge (`layer/` is the product)
- Mother daemon manages databases, children, and cross-project coordination
- WASI component model for sandboxed extensibility (children, toys, pandos)
- Interface-based AI integration (Claude, OpenCode, Gemini)
- Spec-driven workflow for larger changes
- Extensible via schemas, children, and grammar pipelines

## Requirements

- Rust toolchain
- Git
- Docker optional for `patina yolo`
- `gh` optional for GitHub-related flows

## Development

Use the live-install workflow when testing changes:

```bash
cargo build --release && cargo install --path . && patina --help
```

Before pushing:

```bash
./resources/git/pre-push-checks.sh
```

## License

MIT License - see `LICENSE`.
