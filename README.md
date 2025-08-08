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

# Navigate your knowledge base
patina navigate "authentication patterns"
patina navigate "testing" --layer core
patina navigate "docker" --json

# Add patterns to your session
patina add pattern "jwt-refresh-tokens"

# Commit patterns to your brain
patina commit -m "Add JWT refresh token pattern"

# Update context for AI
patina update

# Work with your AI assistant using consistent context
```

### Git-Aware Navigation

Patina's navigation system understands your git workflow:
- **Untracked** (?) - Experimental patterns
- **Modified** (M) - Work in progress
- **Committed** - Locally validated
- **Pushed** (↑) - Shared with team
- **Merged** - Production ready

```bash
# Start rqlite for persistence (currently manual)
docker compose up -d

# Navigate with git awareness
patina navigate "pattern name"

# Future: This will auto-start rqlite when needed
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
# Clone and build
git clone <repo>
cd patina
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