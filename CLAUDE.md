# Patina - Context Orchestration for AI Development

A tool that captures and evolves development patterns, making AI assistants smarter about your projects over time.

## Core Concept
Patina accumulates knowledge like the protective layer that forms on metal - your development wisdom builds up over time and transfers between projects.

## Architecture
- **Layer**: Pattern evolution system (Core → Surface → Dust) with epistemic beliefs
- **Adapters**: LLM-agnostic interfaces (Claude, Gemini, OpenCode)
- **Mother**: Cross-project daemon with graph routing, WASM plugin children
- **Plugin System**: WebAssembly Component Model (WIT) for extensibility
- **Philosophy**: Decompose systems into tools that LLMs can build

## Design Documents
- `layer/core/dependable-rust.md` - Black-box module pattern (small, stable interfaces)
- `layer/core/unix-philosophy.md` - Decomposition principle (systems → tools)
- `layer/core/adapter-pattern.md` - Trait-based external system integration

## Build Recipe
- `layer/core/build.md` - Persistent roadmap and task tracking across sessions. Start here when picking up development work. Contains phased tasks with links to detailed specs.

## AI Workflow Rules
- **NEVER use Claude Code's memory system** (MEMORY.md, ~/.claude/projects/*/memory/) — Patina IS the memory system. Use `layer/sessions/`, `layer/core/`, and `patina scry`/`patina context` instead.
- **NEVER use plan mode** — just read code and do the work directly
- Read code before write code. Spec changes before code changes.
- Before coding: check `cargo tree` for existing deps, read existing patterns in use. Use what's already in the tree — don't introduce new dependencies when the solution is already compiled into the binary. Evolve existing architecture, don't invent parallel ones.

## Development Guidelines
- **Rust-first**: Pure Rust at runtime, no Python subprocess dependencies
  - Embeddings: ONNX Runtime via `ort` crate (not Python/CoreML)
  - Pre-converted models from HuggingFace (no export toolchain)
  - Cross-platform: Same vector space on Mac/Linux/Windows
  - Production-proven: Twitter scale (`ort`), Hugging Face (fastembed)
- Rust for CLI and core logic - let the compiler be your guard rail
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
patina init <name>              # Initialize new project skeleton
patina init .                   # Re-init/update current project
patina adapter add claude       # Add LLM adapter support
patina doctor                   # Check project health

# Knowledge pipeline
patina scrape                   # Build semantic knowledge database
patina oxidize                  # Build embedding projections
patina rebuild                  # Rebuild .patina/ from git-tracked sources

# Search & retrieval
patina scry "query"             # Semantic vector search
patina assay                    # Structural queries (imports, callers, etc.)
patina context                  # Get project patterns and beliefs

# Spec-driven development
patina spec ready               # Show specs ready to work
patina spec status <name> active  # Update spec status
patina version                  # Show version + ready specs

# Session Management (Claude adapter)
/session-start <name>           # Begin development session
/session-update                 # Track progress
/session-note <insight>         # Capture insights
/session-end                    # Distill learnings
```

## Project Structure
```
patina/
├── src/                        # Rust source (~61k lines, 195 files)
│   ├── commands/               # CLI commands (27 total)
│   ├── plugin/                 # WASM plugin engine (wasmtime)
│   ├── mother/                 # Daemon core (graph, children)
│   ├── retrieval/              # Oracle abstraction, RRF fusion
│   ├── mcp/                    # MCP protocol, JSON-RPC server
│   ├── embeddings/             # ONNX E5-base-v2, USearch HNSW
│   ├── release/                # Release strategy and version bumping
│   ├── secrets/                # Age encryption, Keychain integration
│   ├── adapters/               # LLM adapters (Claude, Gemini, OpenCode)
│   └── ...                     # db, forge, git, models, scanner, workspace
├── grammars/                   # Grammar plugins (WASM, outside workspace)
├── plugins/                    # Workspace plugin crates (sdk, doctor, models, repos)
├── layer/                      # Pattern storage (Git as memory)
│   ├── core/                   # Eternal patterns + core beliefs
│   ├── surface/                # Specs, architecture, epistemic beliefs
│   ├── dust/                   # Archived patterns
│   └── sessions/               # Session archives
└── resources/                  # Templates and adapter scripts
```

## Design Philosophy
1. **Knowledge First**: Patterns and beliefs are the core value
2. **LLM Agnostic**: Adapter pattern — work where the AI lives
3. **Pure Rust, Local-first**: SQLite + ONNX + USearch, no cloud required
4. **Spec-driven**: Features start as specs, go through lifecycle, trigger releases
5. **Escape Hatches**: Never lock users in

## Git Discipline

**Commit often, and use a scalpel not a shotgun.**

- Commit after completing each logical change
- One commit = one purpose (fix one bug, add one feature, refactor one function)
- Run `/session-update` frequently to monitor uncommitted changes
- If warned about old uncommitted changes, commit immediately
- Prefer `git add -p` for surgical staging when files have multiple changes

### Session Commands
- Integrated Git workflow into session tracking
- Automatic tagging at session boundaries
- Work classification based on Git metrics
- Failed experiments preserved as memory

### Modular Workspace Architecture
- Decomposed monolithic workspace into focused modules
- Each module is a tool with single responsibility
- Clear input → output transformations
- Apply dependable-rust pattern to isolate change and manage complexity

<!-- PATINA:START -->
## Patina

MCP tools: `scry` (search), `context` (patterns), `assay` (structural queries)

*Generated by Patina | Adapter: Claude Code*
<!-- PATINA:END -->