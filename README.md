# Patina

> Context orchestration for AI-assisted development

Patina accumulates development wisdom like the protective layer that forms on metal—your patterns, decisions, and insights build up over time and transfer between projects.

## What is Patina?

Patina solves the fundamental problem of AI-assisted development: constantly re-teaching AI assistants about your project's context, patterns, and constraints.

**Core idea**: Your development knowledge compounds. Session insights become observations, observations train embeddings, embeddings enable smarter retrieval.

## Features (v0.1.0)

| Feature | Description |
|---------|-------------|
| **Semantic Code Indexing** | Tree-sitter AST extraction for 9 languages |
| **Vector Search** | E5-base-v2 embeddings + USearch HNSW indices |
| **Neuro-symbolic Reasoning** | Embedded Prolog for belief validation |
| **Session Tracking** | Git-integrated session management |
| **YOLO Devcontainers** | AI-ready development environments |
| **LLM Adapters** | Claude and Gemini integration |

## Quick Start

```bash
# Install
cargo install --path .

# Initialize project with Claude adapter
patina init . --llm=claude

# Build knowledge database
patina scrape code

# Generate embeddings
patina embeddings generate

# Search semantically
patina query semantic "error handling patterns"

# Validate a belief against evidence
patina belief validate "this project prefers Result over panic"

# Check project health
patina doctor
```

## Commands

### Project Setup
```bash
patina init <name> --llm=claude    # Initialize with LLM adapter
patina init . --llm=gemini         # Reinitialize current project
patina doctor                       # Check project health
patina upgrade                      # Check for CLI updates
patina version                      # Show version info
```

### Knowledge Pipeline
```bash
patina scrape code                  # Extract AST facts → facts.db
patina scrape code --repo dojo      # Scrape reference repo
patina embeddings generate          # Build vector indices
patina embeddings generate --force  # Rebuild from scratch
patina embeddings status            # Show coverage
```

Supported languages: Rust, TypeScript, JavaScript, Python, Go, C, C++, Solidity, Cairo

### Semantic Search & Reasoning
```bash
patina query semantic "gas optimization"           # Vector search
patina query semantic "ECS" --type pattern         # Filter by type
patina belief validate "prefer composition"        # Prolog validation
patina ask "how does error handling work?"         # Ask about codebase
```

### Development Environment
```bash
patina yolo                         # Generate AI-ready devcontainer
patina yolo --with foundry,cairo    # Add specific tools
patina build                        # Docker containerized build
patina test                         # Run tests in container
```

### Session Management (Claude Adapter)

Within Claude, use these slash commands:
- `/session-start <name>` - Begin session with Git tracking
- `/session-update` - Capture progress
- `/session-note <insight>` - Record insight
- `/session-end` - Archive and distill learnings

## Architecture

```
patina/
├── src/
│   ├── adapters/          # LLM adapters (Claude, Gemini)
│   ├── commands/          # CLI commands
│   ├── embeddings/        # ONNX embeddings + model registry
│   ├── storage/           # SQLite + USearch hybrid storage
│   ├── reasoning/         # Embedded Prolog engine
│   └── query/             # Semantic search
├── layer/                 # Pattern storage
│   ├── core/              # Eternal principles
│   ├── surface/           # Active development
│   └── sessions/          # Session archives
├── .patina/               # Project data
│   ├── data/              # facts.db, code.db, observations/
│   └── config.toml        # Embedding model configuration
└── resources/
    └── models/            # ONNX embedding models
```

### Data Flow

```
Code → patina scrape → facts.db (AST facts, call graph)
                           ↓
Sessions → observations.db → patina embeddings → USearch indices
                           ↓
Query → E5 embedding → vector search → Prolog validation → results
```

### Embedding Models

| Model | Dimensions | Use Case |
|-------|------------|----------|
| all-MiniLM-L6-v2 | 384 | Fast, general-purpose (default) |
| bge-base-en-v1.5 | 768 | SOTA retrieval |
| e5-base-v2 | 768 | Question-answering |
| nomic-embed-text-v1.5 | 768 | Long-form (8K context) |

Configure in `.patina/config.toml`:
```toml
[embeddings]
model = "e5-base-v2"
```

## Design Principles

- **Rust-first**: Pure Rust at runtime, no Python dependencies
- **Local-first**: SQLite + USearch, no cloud services required
- **LLM-agnostic**: Adapter pattern for Claude, Gemini, etc.
- **Git as memory**: Sessions and events committed to repo
- **Vectors are ephemeral**: Rebuildable from source data

## Roadmap: Progressive Adapters (v0.2)

The vision: **One engine, variable patina thickness**.

### Multidimensional Embeddings

Instead of single 768-dim vectors, Patina will produce 2,304-dim multidimensional embeddings via 6 dimension adapters:

| Dimension | Training Data | What It Highlights |
|-----------|---------------|-------------------|
| Semantic | Session observations | Meaning, domain concepts |
| Temporal | Git co-change history | Files that change together |
| Dependency | Call graph | Functions that call each other |
| Syntactic | AST similarity | Similar code structure |
| Architectural | Directory structure | Position in system |
| Social | GitHub metadata | Contributor relationships |

### Patina Thickness Model

Same architecture, progressively richer training data:

- **Fresh patina**: Git + code only (structural understanding)
- **Working patina**: + sessions (patterns emerging)
- **Mature patina**: Deep session integration (contextual wisdom)

See `layer/surface/patina-embedding-architecture.md` for full design.

## Requirements

- Rust 1.70+
- Git
- Docker (for `patina yolo`, `patina build`, `patina test`)

## Development

```bash
# Build release
cargo build --release

# Run tests
cargo test --workspace

# Pre-push checks
./resources/git/pre-push-checks.sh
```

## License

MIT License - see [LICENSE](LICENSE) file for details.
