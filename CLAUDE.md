# Patina - Context Orchestration for AI Development

A tool that captures and evolves development patterns, making AI assistants smarter about your projects over time.

## Core Concept
Patina accumulates knowledge like the protective layer that forms on metal - your development wisdom builds up over time and transfers between projects.

## Architecture
- **Layer**: Hierarchical pattern storage (Core → Topics → Projects)
- **Adapters**: LLM-agnostic interfaces (Claude, Gemini)
- **Environments**: Container-first with escape hatches (Dagger → Docker)

## Design Document
See PROJECT_DESIGN.toml for detailed architecture and design decisions.

## Development Guidelines
- Write Rust exclusively - let the compiler be your guard rail
- Generate other languages through templates only
- Patterns evolve from projects → topics → core
- Always provide escape hatches

## Key Commands
```bash
# Project lifecycle
patina init <name> --llm=claude --dev=dagger
patina add <type> <name>     # Add pattern to session
patina commit                # Commit patterns to layer
patina push                  # Generate LLM context

# Development
patina build                 # Smart build (Dagger or Docker)
patina test                  # Run tests in container
patina update               # Update adapter components
```

## Build System
- Attempts Dagger pipeline if Go is available
- Falls back to Docker automatically
- Never requires specific tools
- Clear feedback about what's being used

## Project Structure
```
patina/
├── src/                    # Rust source (LLMs write here)
├── layer/                  # Pattern storage
│   ├── core/              # Universal patterns
│   ├── topics/            # Domain patterns
│   └── projects/          # Project-specific
├── resources/             # Templates
│   └── templates/         # Go, Docker, etc.
└── pipelines/             # Generated Dagger code
    └── main.go            # Never modified by LLMs
```

## Design Philosophy
1. **Knowledge First**: Patterns are the core value
2. **LLM Agnostic**: Work where the AI lives
3. **Container Native**: Reproducible everywhere
4. **Escape Hatches**: Never lock users in

## Current Focus
Check layer/projects/patina/decisions.md for architectural decisions and layer/topics/ for established patterns.