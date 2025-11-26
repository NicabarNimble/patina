# Spec: Mothership Service

**Status:** Design Phase
**Goal:** Central coordinator for cross-project knowledge, external repos, and model runtime

---

## Core Principle

> **"Projects are islands, personas are gods. Knowledge flows UP (project → persona). Knowledge flows DOWN only through explicit requests."**
>
> — Session 20251107-124740 (Islands & Gods)

**Mothership is a librarian, not a library.** It tracks where knowledge lives, doesn't duplicate it.

---

## Three Responsibilities

Mothership has exactly three jobs:

| # | Responsibility | What It Does | Data Location |
|---|---------------|--------------|---------------|
| 1 | **Knowledge Graph** | Persona beliefs, cross-project patterns, domain wisdom | `~/.patina/persona/` |
| 2 | **Repo Management** | External repos (learning + contributing) | `~/.patina/repos/` |
| 3 | **Model Runtime** | Host E5 + adapters, serve queries, lazy loading | `~/.patina/cache/models/` |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           MOTHERSHIP (~/.patina/)                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐      │
│  │  1. KNOWLEDGE    │  │  2. REPOS        │  │  3. MODEL        │      │
│  │     GRAPH        │  │                  │  │     RUNTIME      │      │
│  │                  │  │  All external    │  │                  │      │
│  │  persona/        │  │  repos live here │  │  cache/models/   │      │
│  │  ├─ beliefs.db   │  │                  │  │  ├─ e5-base-v2/  │      │
│  │  └─ beliefs.     │  │  repos/          │  │  └─ adapters/    │      │
│  │      usearch     │  │  ├─ dojo/        │  │     ├─ semantic  │      │
│  │                  │  │  │  └─ .patina/  │  │     └─ temporal  │      │
│  │                  │  │  ├─ bevy/        │  │                  │      │
│  │                  │  │  │  └─ .patina/  │  │                  │      │
│  │                  │  │  └─ ...          │  │                  │      │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘      │
│                                 │                                        │
│                        registry.yaml                                     │
│                    (tracks primary + repos)                              │
└─────────────────────────────────────────────────────────────────────────┘
                                  │
          ┌───────────────────────┴───────────────────────┐
          ▼                                               ▼
┌──────────────────────────────────────┐    ┌──────────────────────────────┐
│ PRIMARY PROJECTS (yours)              │    │ EXTERNAL REPOS (mothership)   │
│ ~/Projects/patina/                    │    │ ~/.patina/repos/dojo/         │
│ ~/Projects/my-hackathon/              │    │ ~/.patina/repos/bevy/         │
│                                       │    │                               │
│ Full ownership, your code             │    │ Learning or Contributing      │
│ Sessions track YOUR decisions         │    │ Sessions track YOUR learnings │
└──────────────────────────────────────┘    └──────────────────────────────┘
```

---

## Responsibility 1: Knowledge Graph

### What It Is

Your **persona** — cross-project beliefs accumulated over time. This is YOUR knowledge, not any single project's.

### Data Flow

```
Primary Project A  ─┐
Primary Project B  ─┼──▶ Mothership Persona ──▶ Beliefs + Domains
External Repo C    ─┘         (aggregates)
                                    │
                                    ▼
                       ┌────────────────────────┐
                       │ Cross-Project Patterns │
                       │ • "I modularize when   │
                       │    complexity grows"   │
                       │ • "I validate input    │
                       │    before DB writes"   │
                       └────────────────────────┘
```

### Directory Structure

```
~/.patina/persona/
├── beliefs.db              # Cross-project beliefs (SQLite)
│   ├── beliefs             # Aggregated from all projects
│   ├── belief_evidence     # Links to source observations
│   └── domains             # Emergent domain tags
├── beliefs.usearch         # Vector index for semantic search
└── domains/                # Domain-specific knowledge packets
    ├── rust.db             # Rust patterns from all projects
    ├── cairo.db            # Cairo patterns from all projects
    └── ecs.db              # ECS patterns from all projects
```

### Query Flow

```
Project queries: "error handling patterns"
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Query Local Project (.patina/data/)                     │
│     - Search project vectors                                │
│     - Tag results as [PROJECT]                              │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Query Persona (~/.patina/persona/)                      │
│     - Search persona beliefs                                │
│     - Tag results as [PERSONA] or [PERSONA:rust]            │
│     - Apply 0.95x similarity penalty (local > personal)     │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Mark Adoptability                                       │
│     - [ADOPTABLE] if non-contradictory                      │
│     - [REFERENCE] if conflicts with project beliefs         │
└─────────────────────────────────────────────────────────────┘
```

---

## Responsibility 2: Repo Management

### The `patina repo` Command

**One command for all external repos.** Every repo is a full patina project.

```bash
patina repo <url> [--contrib]
```

### What It Always Does

```
patina repo https://github.com/dojoengine/dojo

1. Clone to ~/.patina/repos/dojo/
2. Create patina branch
3. Full .patina/ scaffolding
4. Scrape to .patina/data/patina.db
5. Register in ~/.patina/registry.yaml
6. Ready for queries
```

### What `--contrib` Adds

```
patina repo https://github.com/dojoengine/dojo --contrib

Same as above, PLUS:
7. Fork to your-user/dojo on GitHub
8. Add fork remote
9. Can push PRs back to upstream
```

### The Mental Model

| Flag | Has .patina/ | Has Sessions | Can Query | Can Push PRs |
|------|--------------|--------------|-----------|--------------|
| (none) | ✅ | ✅ | ✅ | ❌ |
| `--contrib` | ✅ | ✅ | ✅ | ✅ |

**Every external repo is a full patina project.** The `--contrib` flag just means "I want to push changes back."

### Directory Structure

```
~/.patina/repos/
├── dojo/                       # Full patina project
│   ├── .patina/
│   │   ├── data/
│   │   │   └── patina.db       # Eventlog + FTS5
│   │   └── config.toml
│   ├── layer/
│   │   └── sessions/           # YOUR learning sessions
│   ├── .claude/                # LLM adapter
│   └── <source code>
│
├── bevy/                       # Another full project
│   ├── .patina/
│   ├── layer/
│   └── <source code>
│
└── starknet-foundry/
    └── ...
```

### Example Workflows

**Learning from bevy (no contribution):**
```bash
patina repo https://github.com/bevyengine/bevy

cd ~/.patina/repos/bevy
patina scry "entity spawning"

# Start a session to capture your learnings
/session-start "studying bevy ecs"
# ... explore code, take notes ...
/session-end
```

**OnlyDust bounty (contribution):**
```bash
patina repo https://github.com/dojoengine/dojo --contrib

cd ~/.patina/repos/dojo
# Work on bounty with full RAG support...
git push fork patina
gh pr create --base main
```

**Upgrade learning → contrib:**
```bash
# Already have bevy from learning
patina repo https://github.com/bevyengine/bevy --contrib
# Detects existing, adds fork remote
# Now can push PRs
```

### Why This Design

1. **Consistent structure** — All repos have `.patina/`, `layer/`, patina branch
2. **Session tracking everywhere** — Capture learnings even when just studying
3. **Easy upgrade path** — Decide to contribute later? Just add `--contrib`
4. **Simpler code** — One path, fork is just an optional step
5. **Deduplication** — All external repos in one place, queryable by any project

---

## Responsibility 3: Model Runtime

### What It Is

Mothership hosts the embedding models and adapters. Projects query mothership instead of loading models themselves.

### Directory Structure

```
~/.patina/cache/
└── models/
    ├── e5-base-v2/             # Base embedding model
    │   ├── model.onnx          # ONNX for cross-platform
    │   └── tokenizer.json
    └── adapters/               # Trained dimension adapters
        ├── semantic.safetensors    # 768→768 semantic projection
        ├── temporal.safetensors    # 768→256 temporal projection
        ├── dependency.safetensors  # 768→256 dependency projection
        └── syntactic.safetensors   # 768→256 syntactic projection
```

### Configuration

```toml
# ~/.patina/config.toml

[models]
base = "e5-base-v2"
adapters = ["semantic", "temporal", "dependency"]

[runtime]
backend = "onnx"              # or "mlx" on Mac
keep_alive = "5m"
max_memory_mb = 2048

[daemon]
port = 50051
socket = "~/.patina/mothership.sock"
```

---

## Registry Schema

```yaml
# ~/.patina/registry.yaml
version: 1

# Primary projects (your code, full ownership)
projects:
  patina:
    path: ~/Projects/patina
    type: primary
    registered: 2025-11-01T10:00:00Z
    domains: [rust, cli, embeddings]

  my-hackathon:
    path: ~/Projects/my-hackathon
    type: primary
    registered: 2025-11-25T15:00:00Z
    domains: [cairo, starknet]

# External repos (learning or contributing)
repos:
  dojo:
    path: ~/.patina/repos/dojo
    github: dojoengine/dojo
    contrib: true                    # Has fork, can push
    fork: nicabar/dojo
    registered: 2025-11-20T10:00:00Z
    domains: [cairo, starknet, ecs]

  bevy:
    path: ~/.patina/repos/bevy
    github: bevyengine/bevy
    contrib: false                   # Learning only
    registered: 2025-11-22T14:00:00Z
    domains: [rust, ecs, game-engine]

  starknet-foundry:
    path: ~/.patina/repos/starknet-foundry
    github: foundry-rs/starknet-foundry
    contrib: true
    fork: nicabar/starknet-foundry
    registered: 2025-11-18T09:00:00Z
    domains: [cairo, testing]
```

---

## CLI Commands

### Repo Management

```bash
# Add external repo (learning mode)
patina repo https://github.com/bevyengine/bevy
patina repo https://github.com/dojoengine/dojo

# Add external repo (contribution mode)
patina repo https://github.com/dojoengine/dojo --contrib

# What happens:
# 1. Clone to ~/.patina/repos/<name>/
# 2. Create patina branch
# 3. Full .patina/ scaffolding
# 4. Scrape codebase
# 5. Register in registry
# 6. (--contrib only) Fork on GitHub, add remote

# List repos
patina repo list
# Output:
# NAME              GITHUB                        CONTRIB   DOMAINS
# dojo              dojoengine/dojo               ✓ fork    cairo, ecs
# bevy              bevyengine/bevy               -         rust, ecs
# starknet-foundry  foundry-rs/starknet-foundry   ✓ fork    cairo, testing

# Update a repo (git pull + rescrape)
patina repo update dojo
patina repo update --all

# Remove a repo
patina repo rm bevy

# Show repo details
patina repo show dojo
# Output:
# Name: dojo
# GitHub: dojoengine/dojo
# Path: ~/.patina/repos/dojo
# Contrib: Yes (fork: nicabar/dojo)
# Branch: patina
# Domains: cairo, starknet, ecs
# Events: 24,531
# Last updated: 2 hours ago

# Upgrade learning → contrib
patina repo https://github.com/bevyengine/bevy --contrib
# Detects existing repo, adds fork
```

### Primary Project Management

```bash
# Register current project as primary
patina project add .

# List primary projects
patina project list

# Primary projects are YOUR code
# They live in ~/Projects/ (or wherever you want)
# They're separate from external repos in ~/.patina/repos/
```

### Cross-Project Scry

```bash
# Query current project (default)
patina scry "error handling"

# Query specific repo
patina scry "spawn patterns" --repo dojo
patina scry "ecs systems" --repo bevy

# Query persona (cross-project beliefs)
patina scry "test patterns" --persona

# Query multiple sources
patina scry "entity component" --repos dojo,bevy
```

### Persona Commands

```bash
# Query persona beliefs
patina persona query "error handling patterns"

# List domains in persona
patina persona domains
# Output:
# DOMAIN      BELIEFS   PROJECTS
# rust        47        3
# cairo       23        2
# ecs         15        4
```

---

## Implementation Phases

### Phase 1: MVP (Hackathon Ready)

| Feature | Why | Effort |
|---------|-----|--------|
| `patina repo <url>` | Clone + scaffold + scrape | 2 days |
| `patina repo <url> --contrib` | Add fork capability | 1 day |
| `patina repo list` | See what's available | 0.5 day |
| `--repo` flag on scry | Query specific repo | 1 day |
| Registry file | Track projects + repos | 0.5 day |

**Total: ~5 days**

**Note:** Phase 1 works WITHOUT daemon. Direct DB access.

### Phase 2: Daemon + Persona

| Feature | Why |
|---------|-----|
| `patina serve` daemon | Hot model loading, container support |
| Persona beliefs | Cross-project knowledge accumulation |
| `--persona` flag | Query persona alongside project |
| Lazy loading + eviction | Memory management |

### Phase 3: Container Ready

| Feature | Why |
|---------|-----|
| `PATINA_MOTHERSHIP` env var | Container → Mac queries |
| gRPC API | Efficient cross-process communication |
| Model runtime optimization | MLX on Mac, ONNX elsewhere |

---

## Migration from layer/dust/repos/

Current reference repos in `layer/dust/repos/` will migrate:

```bash
patina migrate-repos

# What happens:
# 1. For each repo in layer/dust/repos/:
#    - Move to ~/.patina/repos/<name>/
#    - Add .patina/ scaffolding
#    - Create patina branch
#    - Register in registry
# 2. Update .gitignore
# 3. Remove layer/dust/repos/ (or symlink for compat)
```

---

## Acceptance Criteria

### Phase 1 (MVP)
- [ ] `patina repo https://github.com/dojoengine/dojo` clones, scaffolds, scrapes
- [ ] `patina repo <url> --contrib` also forks and sets up remote
- [ ] `patina repo list` shows registered repos
- [ ] `patina repo update dojo` pulls and rescrapes
- [ ] `patina scry "query" --repo dojo` queries repo's database
- [ ] Registry persists (`~/.patina/registry.yaml`)
- [ ] Migration from `layer/dust/repos/`

### Phase 2 (Daemon + Persona)
- [ ] `patina serve` starts daemon with hot E5 model
- [ ] `patina persona query` searches cross-project beliefs
- [ ] Results tagged as `[PROJECT]`, `[PERSONA]`, `[REPO:name]`
- [ ] Lazy loading with KEEP_ALIVE eviction

### Phase 3 (Container Ready)
- [ ] `PATINA_MOTHERSHIP` env var support
- [ ] Container can query Mac mothership via gRPC
- [ ] Model adapters load on demand

---

## Key Principles (Summary)

1. **All external repos are full patina projects** — `.patina/`, `layer/`, patina branch
2. **`--contrib` just adds fork capability** — Same structure, different permissions
3. **Central storage** — All repos in `~/.patina/repos/`, queryable from anywhere
4. **Session tracking everywhere** — Learn from repos, capture insights
5. **Easy upgrade path** — Learning → Contributing is just adding a fork
6. **Mothership is a librarian** — Tracks where knowledge lives, doesn't duplicate
