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
- **Brain Storage**: File-based (now), SQLite (future)

## Quick Start

```bash
# Initialize a new project
patina init my-project

# Add topic knowledge
patina topic add blockchain starknet

# Generate context for AI
patina context generate

# Work with your AI assistant using consistent context
```

## Architecture

```
patina-core/        # Context orchestration
patina-brain/       # Knowledge storage
patina-env/         # Environment traits
patina-llm/         # LLM adapter traits
```

Each component does one thing well, following Unix philosophy.

## Development Status

Currently building the initial file-based brain system. SQLite storage coming next.

## Philosophy

- **One tool, one job**: Each component has a single, clear purpose
- **Context is king**: Everything serves consistent context
- **User patterns first**: Respect existing workflows
- **Knowledge compounds**: Every project makes future projects smarter