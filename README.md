# Patina (Codename)

> Context orchestration for AI-assisted development

Patina accumulates a protective layer of wisdom over time, making AI interactions more effective with each use.

## What is Patina?

Patina is a context orchestration system that solves the fundamental problem of AI-assisted development: constantly re-teaching AI assistants about your project's context, patterns, and constraints.

Like the patina that forms on metal, your development wisdom accumulates into a protective layer that:
- Preserves your architectural decisions
- Maintains your coding patterns
- Enforces your constraints
- Grows richer over time

## Core Concepts

### Hierarchical Context
```
Core (Universal principles)
 └─ Topics (Domain knowledge)
     └─ Projects (Specific implementations)
```

### Knowledge Evolution
Project patterns that prove successful can be promoted to topics, making them available for future projects. Your personal database project might become a "personal-data" topic for other projects to build upon.

### Pluggable Architecture
- **Environment Providers**: Docker, Dagger, Nix
- **LLM Adapters**: Claude, OpenAI, Local models
- **Layer Storage**: File-based (now), SQLite (future)

## Quick Start

```bash
# Initialize a new project
patina init my-project

# Build semantic knowledge database
patina scrape --init              # Initialize database
patina scrape                     # Index codebase

# Check project health
patina doctor

# Manage development environments
patina agent start                # Start container orchestration
patina build                      # Build in container
patina test                       # Test in container

# Work with your AI assistant using consistent context
```

### Semantic Knowledge Database

Patina's scrape command builds a searchable database from your codebase:
- Indexes code structure and documentation
- Tracks incremental changes for efficiency  
- Uses DuckDB for fast semantic queries
- Integrates with Git for freshness checks

```bash
# Initialize and build knowledge base
patina scrape --init
patina scrape

# Query the database directly
patina scrape --query "SELECT * FROM symbols WHERE kind = 'function'"
```

## Architecture

```
patina-core/        # Context orchestration
patina-layer/       # Knowledge storage
patina-env/         # Environment traits
patina-llm/         # LLM adapter traits
```

Each component does one thing well, following Unix philosophy.

## Current Setup

### Requirements
- Rust toolchain
- Git (for state detection)
- Docker (optional, for rqlite persistence)

### Quick Setup
```bash
# Clone with submodules (required for parser support)
git clone --recursive <repo>
cd patina

# Or if you already cloned without --recursive:
# git submodule update --init --recursive

cargo build --release

# Copy docker-compose.yml.example for rqlite
cp docker-compose.yml.example docker-compose.yml
docker compose up -d  # Optional: for persistence
```

## Development Status

✅ **Completed:**
- Core layer system with pattern management
- Git-aware navigation with confidence scoring
- Real-time git state detection
- rqlite persistence for decentralized sharing
- Claude adapter with session commands

🚧 **In Progress:**
- Auto-management of rqlite process
- Workspace integration for pattern exploration
- Pattern promotion workflows
- Additional LLM adapters

## Philosophy

- **One tool, one job**: Each component has a single, clear purpose
- **Context is king**: Everything serves consistent context
- **User patterns first**: Respect existing workflows
- **Knowledge compounds**: Every project makes future projects smarter