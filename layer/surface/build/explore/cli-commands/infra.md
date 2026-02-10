# Infra Commands

> Infrastructure setup and management.

## Overview

| Command | Status | One-Line Description |
|---------|--------|---------------------|
| `init` | EXISTS | Initialize a new project |
| `adapter` | EXISTS | Manage LLM adapters |
| `model` | EXISTS | Manage embedding models |
| `mother` | EXISTS | Daemon management |
| `repo` | EXISTS | External repository management |
| `secrets` | EXISTS | Secret management |
| `rebuild` | EXISTS | Rebuild from sources |
| `upgrade` | EXISTS | Check CLI version |
| `version` | EXISTS | Project versioning (semver) |
| `spec` | EXISTS | Spec lifecycle management |
| `yolo` | EXISTS | Devcontainer generation |

---

## `init` — Initialize Project

### What does it do?
Creates a new Patina project with skeleton structure.

### Current Interface
```
patina init <name>                # New project
patina init .                     # Initialize current directory
```

### What does it read?
- Templates (bundled or from `~/.patina/templates/`)

### What does it write?
- `.patina/` directory structure
- `layer/` directory structure
- `.gitignore` updates

### Who uses it?
- User: Starting a new project

### When is it used?
Once per project.

### Gaps
- No interactive mode for configuration
- Doesn't add adapters (separate step)

### Overlaps
- None

---

## `adapter` — Manage LLM Adapters

### What does it do?
Add, configure, and manage LLM adapters (Claude, Gemini, etc.).

### Current Interface
```
patina adapter list               # List available/allowed adapters
patina adapter add <name>         # Add adapter to project
patina adapter remove <name>      # Remove adapter from project
patina adapter default <name>     # Set default adapter
patina adapter check              # Check installation status
patina adapter doctor             # Health check adapters
patina adapter refresh            # Update adapter files
patina adapter mcp                # Configure MCP server
```

### What does it read?
- `~/.patina/adapters/` (global adapter configs)
- `.patina/adapters/` (project adapter configs)
- Adapter templates

### What does it write?
- `.patina/adapters/<name>/` (adapter config, CLAUDE.md, etc.)
- MCP configuration files

### Who uses it?
- User: Setting up LLM integration

### When is it used?
Setup — when adding/configuring adapters.

### Gaps
- No way to see which adapter is currently active (runtime)
- No connection to `science config`

### Overlaps
- `adapter doctor` overlaps with `dev doctor`

---

## `model` — Manage Embedding Models

### What does it do?
Manage embedding models in the mother cache.

### Current Interface
```
patina model list                 # List available models
patina model add <name>           # Download model to cache
patina model remove <name>        # Remove model from cache
patina model status               # Show model status for project
```

### What does it read?
- `~/.patina/cache/models/` (model cache)
- Model registry (remote?)

### What does it write?
- `~/.patina/cache/models/<name>/` (downloaded models)

### Who uses it?
- User: Switching embedding models
- Dev: Testing different models

### When is it used?
Rare — when adding/changing models.

### Gaps
- No way to set active model per project
- No connection to `science config`
- Unclear which models are available

### Overlaps
- Model selection should integrate with `science config`

---

## `mother` — Daemon Management

### What does it do?
Manage the Patina daemon for cross-project knowledge, caching, and routing.

### Current Interface
```
patina mother start               # Start daemon
patina mother stop                # Stop daemon
patina mother status              # Check daemon status
patina mother query               # Query across projects
```

### What does it read?
- `~/.patina/mother/` (daemon state)
- Project registrations

### What does it write?
- `~/.patina/mother/graph.db` (project graph)
- Daemon PID file

### Who uses it?
- User: Cross-project queries
- System: Background operations

### When is it used?
Rare — daemon usually auto-starts.

### Gaps
- mother-v2 spec has expanded vision
- No health metrics exposed

### Overlaps
- None

---

## `repo` — External Repository Management

### What does it do?
Manage external reference repositories for cross-project knowledge.

### Current Interface
```
patina repo add <url>             # Add reference repo
patina repo list                  # List reference repos
patina repo remove <name>         # Remove reference repo
patina repo update                # Update all repos
```

### What does it read?
- Git repositories (clone/pull)
- `.patina/repos/` or mother registry

### What does it write?
- Cloned repositories
- Mother graph (repo registration)

### Who uses it?
- User: Adding reference codebases

### When is it used?
Rare — when setting up cross-project knowledge.

### Gaps
- No selective scraping (whole repo or nothing)
- No health/staleness tracking

### Overlaps
- None

---

## `secrets` — Secret Management

### What does it do?
Secure secret management with age encryption.

### Current Interface
```
patina secrets init               # Initialize secrets
patina secrets set <key> <value>  # Set a secret
patina secrets get <key>          # Get a secret
patina secrets list               # List secret keys
```

### What does it read?
- `.patina/secrets/` (encrypted files)
- Age key files

### What does it write?
- `.patina/secrets/` (encrypted secrets)

### Who uses it?
- User: Storing API keys, tokens

### When is it used?
Rare — setup, when adding new secrets.

### Gaps
- Integration with adapters not clear
- No rotation support

### Overlaps
- None

---

## `rebuild` — Rebuild from Sources

### What does it do?
Rebuild `.patina/` from `layer/` and local sources (for portability).

### Current Interface
```
patina rebuild                    # Full rebuild
patina rebuild --dry-run          # Show what would be rebuilt
```

### What does it read?
- `layer/` (all layer content)
- Source files

### What does it write?
- `.patina/local/data/patina.db` (recreated)
- `.patina/local/data/embeddings/` (recreated)

### Who uses it?
- User: After cloning repo, restoring state

### When is it used?
Rare — after clone, or when DB is corrupted.

### Gaps
- Slow for large codebases
- No progress indicator

### Overlaps
- Essentially runs scrape + oxidize

---

## `upgrade` — Check CLI Version

### What does it do?
Check for new Patina CLI versions.

### Current Interface
```
patina upgrade                    # Check for updates
patina upgrade --install          # Install update (if available)
```

### What does it read?
- Version registry (crates.io or GitHub releases)

### What does it write?
- Binary update (if `--install`)

### Who uses it?
- User: Keeping CLI current

### When is it used?
Rare — periodic update checks.

### Gaps
- No auto-update mechanism
- No changelog display

### Overlaps
- None

---

## `version` — Project Versioning

### What does it do?
Manage project versioning (semver: MAJOR.MINOR.PATCH).

### Current Interface
```
patina version                    # Show current version
patina version bump major|minor|patch
patina version set <version>
```

### What does it read?
- `Cargo.toml` or version file

### What does it write?
- Version file updates

### Who uses it?
- User: Release management

### When is it used?
Rare — at release time.

### Gaps
- No changelog generation
- No git tag integration

### Overlaps
- None

---

## `spec` — Spec Lifecycle Management

### What does it do?
Manage spec lifecycle (archive completed specs).

### Current Interface
```
patina spec list                  # List specs by status
patina spec archive <id>          # Archive completed spec
patina spec status <id>           # Show spec status
```

### What does it read?
- `layer/surface/build/` (spec files)

### What does it write?
- Moves specs between directories (feat → dust)

### Who uses it?
- Dev: Managing spec lifecycle

### When is it used?
Rare — when completing specs.

### Gaps
- No spec creation wizard
- No dependency tracking between specs

### Overlaps
- None

---

## `yolo` — Devcontainer Generation

### What does it do?
Generate YOLO devcontainer for autonomous AI development.

### Current Interface
```
patina yolo                       # Generate devcontainer
patina yolo --config <file>       # Custom config
```

### What does it read?
- Templates
- Project config

### What does it write?
- `.devcontainer/` (devcontainer config)
- Docker files

### Who uses it?
- Dev: Setting up AI-friendly dev environment

### When is it used?
Rare — project setup.

### Gaps
- YOLO concept may be outdated
- Integration with adapters unclear

### Overlaps
- None

---

## Summary

| Command | Purpose | Frequency |
|---------|---------|-----------|
| `init` | Create project | Once |
| `adapter` | LLM adapter management | Setup |
| `model` | Embedding model management | Rare |
| `mother` | Daemon management | Rare |
| `repo` | Reference repositories | Rare |
| `secrets` | Secret management | Setup |
| `rebuild` | Rebuild from sources | Rare |
| `upgrade` | CLI updates | Rare |
| `version` | Semver management | Releases |
| `spec` | Spec lifecycle | Dev workflow |
| `yolo` | Devcontainer | Setup |

## Integration Points

1. **`adapter` + `model` → `science config`**: Config ties adapter and model together
2. **`model` → `oxidize`**: Model selection affects embedding
3. **`repo` → `mother`**: Repos register with mother for federation
4. **`secrets` → `adapter`**: API keys for adapters
