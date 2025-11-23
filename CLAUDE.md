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
- `layer/core/dependable-rust.md` - Black-box module pattern (small, stable interfaces)
- `layer/core/unix-philosophy.md` - Decomposition principle (systems → tools)
- `layer/core/adapter-pattern.md` - Trait-based external system integration

## Build Recipe
- `layer/core/build.md` - Persistent roadmap and task tracking across sessions. Start here when picking up development work. Contains phased tasks with links to detailed specs.

## Development Guidelines
- **Rust-first**: Pure Rust at runtime, no Python subprocess dependencies
  - Embeddings: ONNX Runtime via `ort` crate (not Python/CoreML)
  - Pre-converted models from HuggingFace (no export toolchain)
  - Cross-platform: Same vector space on Mac/Linux/Windows
  - Production-proven: Twitter scale (`ort`), Hugging Face (fastembed)
- Rust for CLI and core logic - let the compiler be your guard rail
- Docker for containerized builds and tests
- Patterns evolve from projects → topics → core
- Always provide escape hatches

## Testing Guidelines - IMPORTANT
**Always build release and test with live install:**
```bash
cargo build --release                    # Build release binary
cargo install --path .                   # Install to ~/.cargo/bin
patina <command>                         # Test with actual installed binary
```

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
```

The CI will fail if any of these checks don't pass! The pre-push script runs all checks for you.

## Key Commands
```bash
# Project lifecycle
patina init <name> --llm=claude --dev=docker  # Initialize new project
patina init .                                  # Re-init/update current project
patina init . --llm=gemini                    # Switch LLM adapter

# Development
patina build                # Docker containerized builds
patina test                 # Run tests in container
patina doctor               # Check project health
patina scrape                # Build semantic knowledge database

# Session Management (Claude adapter)
/session-git-start <name>       # Begin development session
/session-git-update             # Track progress
/session-git-note <insight>     # Capture insights
/session-git-end                # Distill learnings

```

## Build System
- Uses Docker for containerized builds
- Never requires specific tools beyond Docker
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
└── resources/             # Templates and scripts
    ├── claude/            # Claude adapter resources (session/git scripts)
    ├── gemini/            # Gemini adapter resources
    └── templates/         # Docker templates
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

### Session-Git Commands
- Integrated Git workflow into session tracking
- Automatic tagging at session boundaries
- Work classification based on Git metrics
- Failed experiments preserved as memory

### Modular Workspace Architecture
- Decomposed monolithic workspace into focused modules
- Each module is a tool with single responsibility
- Clear input → output transformations
- Apply dependable-rust pattern to isolate change and manage complexity
