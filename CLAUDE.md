# Patina - Context Orchestration for AI Development

A tool that captures and evolves development patterns, making AI assistants smarter about your projects over time.

## Core Concept
Patina accumulates knowledge like the protective layer that forms on metal - your development wisdom builds up over time and transfers between projects.

## Architecture
- **Layer**: Pattern evolution system (Core → Surface → Dust)
- **Adapters**: LLM-agnostic interfaces (Claude, Gemini) 
- **Environments**: Modular workspace system with container orchestration
- **Philosophy**: Decompose systems into tools that LLMs can build

## Design Documents
- `PROJECT_DESIGN.toml` - Core architecture and design decisions
- `layer/surface/pattern-selection-framework.md` - Pattern selection strategy
- `layer/surface/modular-architecture-plan.md` - Workspace decomposition

## Development Guidelines
- Rust for CLI and core logic - let the compiler be your guard rail
- Go for Dagger integration - embrace Go idioms with solid testing
- Patterns evolve from projects → topics → core
- Always provide escape hatches

## Git Commit Guidelines
- NEVER add "🤖 Generated with Claude Code" or "Co-Authored-By: Claude" to commit messages
- Keep commit messages clean and professional without AI attribution
- Focus on what changed and why, not who/what wrote it

## CI Requirements - IMPORTANT
Before pushing, ALWAYS run these checks locally:
```bash
# Quick way - run all checks at once:
./resources/git/pre-push-checks.sh

# Or run individually:
cargo fmt --all           # Fix Rust formatting
cargo clippy --workspace  # Check for warnings
cargo test --workspace    # Run tests

# Go checks (if workspace/ exists)
cd workspace && go fmt ./... && go test -v ./... && cd ..
```

The CI will fail if any of these checks don't pass! The pre-push script runs all checks for you.

## Key Commands
```bash
# Project lifecycle
patina init <name> --llm=claude --dev=dagger  # Initialize new project
patina init .                                  # Re-init/update current project
patina init . --llm=gemini                    # Switch LLM adapter
patina init . --dev=docker                    # Switch dev environment
patina init . --llm=claude --dev=dagger       # Update or switch both adapters

# Development
patina build                # Smart build (Dagger or Docker)
patina test                 # Run tests in container
patina doctor               # Check project health
patina scrape                # Build semantic knowledge database
patina agent <command>      # Manage modular workspace environments

# Session Management (Claude adapter)
/session-git-start <name>       # Begin development session
/session-git-update             # Track progress
/session-git-note <insight>     # Capture insights
/session-git-end                # Distill learnings

```

## Build System
- Attempts Dagger pipeline if Go is available
- Falls back to Docker automatically
- Never requires specific tools
- Clear feedback about what's being used

## Project Structure
```
patina/
├── src/                    # Rust source (CLI and core logic)
│   ├── adapters/          # LLM adapters (Claude, Gemini)
│   ├── commands/          # CLI commands
│   └── indexer/           # Pattern indexing with Git awareness
├── layer/                  # Pattern storage (Git as memory)
│   ├── core/              # Eternal patterns (dependable-rust, etc)
│   ├── surface/           # Active development & architecture docs
│   ├── dust/              # Historical/archived patterns
│   └── sessions/          # Distilled session knowledge
├── resources/             # Templates and scripts
│   ├── claude/            # Claude adapter resources (session/git scripts)
│   ├── gemini/            # Gemini adapter resources
│   └── templates/         # Go, Docker, Dagger templates
├── modules/               # Modular workspace system (Go)
│   ├── environment-registry/  # Track active environments
│   ├── environment-provider/  # Create containers
│   ├── code-executor/         # Execute commands
│   ├── git-manager/           # Git operations
│   └── api-gateway/           # HTTP coordination
└── pipelines/             # Generated Dagger code
    └── main.go            # Container orchestration
```

## Design Philosophy
1. **Knowledge First**: Patterns are the core value
2. **LLM Agnostic**: Work where the AI lives
3. **Container Native**: Reproducible everywhere
4. **Escape Hatches**: Never lock users in

## Git Discipline

**Commit often, and use a scalpel not a shotgun.**

- Commit after completing each logical change
- One commit = one purpose (fix one bug, add one feature, refactor one function)
- Run `/session-git-update` frequently to monitor uncommitted changes
- If warned about old uncommitted changes, commit immediately
- Prefer `git add -p` for surgical staging when files have multiple changes

## Recent Developments

### Session-Git Commands
- Integrated Git workflow into session tracking
- Automatic tagging at session boundaries
- Work classification based on Git metrics
- Failed experiments preserved as memory

### Modular Workspace Architecture
- Decomposed monolithic workspace into focused modules
- Each module is a tool with single responsibility
- Clear input → output transformations
- environment-registry module follows Eternal Tool pattern

### Pattern Selection Framework
- Three categories: Eternal Tools, Stable Adapters, Evolution Points
- Apply different patterns based on code characteristics
- Tool-based decomposition for LLM-friendly development
- See `layer/surface/pattern-selection-framework.md`
