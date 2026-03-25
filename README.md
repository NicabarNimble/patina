# Patina

> Context orchestration for AI-assisted development

Patina is a local-first Rust CLI that turns a repository into a reusable knowledge system for humans and AI tools. It scrapes code, git history, layer artifacts, and optional external sources into a local store, builds semantic indices, and exposes commands for retrieval, structure queries, workflow tracking, and cross-project knowledge.

## What Patina Does

- Builds project memory from code, commits, specs, sessions, and beliefs
- Provides semantic search with `patina scry` and structural queries with `patina assay`
- Captures project rules and decisions through `patina context`, specs, and epistemic beliefs
- Supports cross-project knowledge via external repos, persona data, and the Mother daemon
- Extends the system with adapters, schemas, and WASM children

Patina is designed around a simple idea: project knowledge should compound instead of being re-explained every session.

## Quick Start

```bash
# Install the CLI
cargo install --path .

# First-run setup
patina setup grammars

# Initialize a project
patina init .

# Allow an AI adapter in this repo
patina adapter add claude

# Build the local knowledge base
patina scrape
patina oxidize

# Query it
patina scry "error handling patterns"
patina assay search "child engine"
patina context --topic "testing"
```

Once configured, running bare `patina` launches the default adapter.

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
```

### Project workflow

```bash
patina session start "feature name"
patina session update
patina session note "important insight"
patina session end

patina spec create feature-name
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
patina mother search "belief query"
patina mother sources

patina model list
patina model add e5-base-v2
```

### Setup, extension, and safety

```bash
patina adapter list
patina adapter add opencode
patina adapter mcp

patina child list                  # Canonical (alias: `patina plugin list`)
patina child init my-child --world task
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
Sources                         Storage / indices                  Query surface
-------                         -----------------                  -------------
git history                 ->  .patina/local/data/patina.db   ->  scry
code + grammar children     ->  .patina/local/data/events.db   ->  assay
layer/ (specs, sessions,    ->  .patina/local/data/embeddings  ->  context
beliefs, patterns)                                              ->  belief / report / measure
external repos + sources
```

Core ideas:

- `scry` is the semantic retrieval layer
- `assay` is the factual and structural retrieval layer
- `context` surfaces project rules, conventions, and beliefs for AI assistants
- `rebuild` restores derived state from tracked project artifacts
- Mother provides long-lived cross-project services, caching, and routing

## Repository Layout

```text
patina/
├── src/                      # Rust CLI, retrieval, daemon, adapters, storage
├── grammars/                 # Grammar child sources used by scrape pipeline
├── plugins/                  # Transitional child crates (compatibility surface)
├── crates/                   # Shared internal crates
├── children/                 # First-party child components
├── wit/                      # WIT contracts (toys/) and compositions (worlds/)
├── sdk/                      # SDK tiers: core, data, agent, umbrella
├── layer/                    # Git-tracked project memory
│   ├── core/                 # Stable principles and patterns
│   ├── surface/              # Active specs, beliefs, reports, architecture docs
│   ├── sessions/             # Session archives
│   └── dust/                 # Archived material
└── .patina/
    ├── config.toml           # Project config
    ├── uid                   # Project identity
    ├── oxidize.yaml          # Embedding recipe
    ├── versions.json         # Version manifest
    └── local/
        ├── data/
        │   ├── patina.db     # Rebuildable project database
        │   ├── events.db     # Runtime event store
        │   └── embeddings/   # Vector indices and projections
        └── backups/

~/.patina/
├── config.toml               # Global config
├── registry.yaml             # Project and repo registry
├── adapters/                 # Adapter templates
├── cache/repos/              # Cloned reference repos
├── pipeline/                 # Installed grammar child artifacts
├── personas/                 # Cross-project persona events
└── run/                      # Mother runtime files
```

## Design Principles

- Pure Rust runtime; no Python subprocess dependency chain
- Local-first storage and rebuildable derived state
- Git as memory for durable project knowledge
- Adapter-based AI integration
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
