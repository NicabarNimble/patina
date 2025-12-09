# Project: Patina

Context orchestration for AI-assisted development.

## What This Is

Patina is a local-first RAG network that captures and evolves development patterns, making AI assistants smarter about your projects over time. Like the protective layer that forms on metal, your development wisdom builds up and transfers between projects.

## Architecture

```
patina/
├── src/                    # Rust source (CLI and core logic)
│   ├── adapters/          # LLM adapters + launcher functionality
│   ├── commands/          # CLI commands
│   ├── workspace/         # Global ~/.patina/ management
│   └── mothership/        # HTTP client for daemon communication
├── layer/                  # Pattern storage (Git as memory)
│   ├── core/              # Eternal patterns (dependable-rust, etc)
│   ├── surface/           # Active development & architecture docs
│   ├── rules/             # LLM instructions (this directory)
│   └── sessions/          # Distilled session knowledge
└── resources/             # Templates and scripts
```

## Key Modules

| Module | Purpose |
|--------|---------|
| `workspace` | First-run setup, ~/.patina/ structure, global config |
| `adapters/launch` | Frontend manifests, CLI detection, bootstrap generation |
| `commands/launch` | Open projects in AI frontends (like `code .`) |
| `scry` | Semantic search across project knowledge |
| `serve` | Mothership daemon (HTTP + future MCP) |
| `oxidize` | Embedding training and projection |

## Getting Started

```bash
# Development build
cargo build --release
cargo install --path .

# Test with actual binary
patina adapter list         # See available frontends
patina launch               # Open in default frontend
patina scry "query"         # Search knowledge base
```

## Key Entry Points

- `src/main.rs` - CLI entry point and command dispatch
- `src/lib.rs` - Library exports for all modules
- `src/commands/launch/` - Launcher implementation
- `src/workspace/` - Global config and first-run
