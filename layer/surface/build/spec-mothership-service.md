# Spec: Mothership Service

**Status:** Design Phase
**Goal:** Ollama-style daemon for cross-project knowledge queries

---

## Design Philosophy

**What Would Ollama Do (WWOD)**

Ollama's genius is simplicity:
- `ollama pull llama3` → downloads model
- `ollama serve` → daemon in background
- `ollama run llama3` → talks to daemon
- Model loads on first use, unloads after idle

Patina Mothership follows the same pattern:
- `patina repo add dojo` → clones + scrapes repo
- `patina serve` → daemon in background
- `patina scry --project dojo` → talks to daemon
- DBs/indices load on first query, unload after idle

**Key Principles:**
1. **Daemon is optional** - local scry works without it
2. **Explicit registration** - nothing auto-discovered
3. **Lazy loading** - memory only used when needed
4. **Federated data** - projects keep their data, mothership has pointers

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           MOTHERSHIP DAEMON                              │
│                         (patina serve :50051)                            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                │
│   │   Registry  │    │   Router    │    │   Cache     │                │
│   │             │    │             │    │             │                │
│   │ • projects  │───▶│ • classify  │───▶│ • models    │                │
│   │ • repos      │    │ • route     │    │ • DBs       │                │
│   │ • alignment │    │ • merge     │    │ • indices   │                │
│   └─────────────┘    └─────────────┘    └─────────────┘                │
│          │                  │                  │                        │
│          ▼                  ▼                  ▼                        │
│   ~/.patina/          Query to           Lazy load,                    │
│   registry.yaml       right DB           evict after                   │
│                                          KEEP_ALIVE                    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
            ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
            │   Project   │ │   Project   │ │    Ref      │
            │   patina    │ │  hackathon  │ │    dojo     │
            │             │ │             │ │             │
            │ ~/Projects/ │ │ ~/Projects/ │ │ ~/.patina/  │
            │ patina/     │ │ hackathon/  │ │ repos/dojo/  │
            │ .patina/    │ │ .patina/    │ │             │
            │ data/       │ │ data/       │ │ dojo.db     │
            └─────────────┘ └─────────────┘ └─────────────┘
                  │               │               │
                  └───────────────┴───────────────┘
                           Federated Data
                    (stays with project/repo)
```

---

## Directory Structure

### Mothership Home (`~/.patina/`)

```
~/.patina/                          # PATINA_HOME
├── registry.yaml                   # Projects + repos registry
├── config.toml                     # Service configuration
├── repos/                           # Reference repositories
│   ├── dojo/                       # git clone of dojoengine/dojo
│   ├── dojo.db                     # scraped eventlog + FTS5
│   ├── dojo.usearch                # vector indices (if oxidized)
│   ├── starknet-foundry/
│   ├── starknet-foundry.db
│   └── ...
├── persona/                        # Cross-project beliefs (Phase 4)
│   ├── beliefs.db
│   └── beliefs.usearch
├── cache/
│   └── models/                     # ONNX models (shared)
│       └── e5-base-v2/
└── mothership.sock                 # Unix socket (optional)
```

### Project Data (stays with project)

```
~/Projects/my-hackathon/
├── .patina/
│   ├── config.toml                 # Project config
│   └── data/
│       ├── patina.db               # Project eventlog
│       └── embeddings/
│           └── e5-base-v2/
│               └── projections/
│                   ├── semantic.safetensors
│                   └── semantic.usearch
├── src/
└── ...
```

---

## Registry Schema

```yaml
# ~/.patina/registry.yaml
version: 1

# Active projects (registered via `patina project add`)
projects:
  patina:
    path: /Users/nicabar/Projects/patina
    type: primary                    # primary | contributor | dormant
    registered: 2025-11-01T10:00:00Z
    last_activity: 2025-11-25T14:45:00Z

    # Auto-detected from scrape
    domains:
      - rust
      - cli
      - embeddings

    # What's indexed
    indexed:
      eventlog: true                 # has patina.db
      semantic: true                 # has semantic.usearch
      temporal: true                 # has temporal.usearch
      fts5: true                     # has FTS5 tables

  my-hackathon:
    path: /Users/nicabar/Projects/my-hackathon
    type: primary
    registered: 2025-11-25T15:00:00Z
    last_activity: 2025-11-25T15:30:00Z
    domains:
      - cairo
      - starknet
      - dojo
    indexed:
      eventlog: true
      semantic: false
      temporal: false
      fts5: true

# Reference repositories (registered via `patina repo add`)
repos:
  dojo:
    github: dojoengine/dojo
    path: ~/.patina/repos/dojo        # git clone location
    db: ~/.patina/repos/dojo.db       # scraped data
    branch: main                     # tracked branch
    last_updated: 2025-11-25T14:45:00Z
    domains:
      - cairo
      - starknet
      - ecs
      - game-engine
    indexed:
      eventlog: true
      fts5: true
      semantic: false                # repos don't need oxidize by default

  starknet-foundry:
    github: foundry-rs/starknet-foundry
    path: ~/.patina/repos/starknet-foundry
    db: ~/.patina/repos/starknet-foundry.db
    branch: main
    last_updated: 2025-11-20T10:00:00Z
    domains:
      - cairo
      - testing
      - starknet
    indexed:
      eventlog: true
      fts5: true
      semantic: false
```

---

## CLI Commands

### Repo Management (`patina repo`)

```bash
# Add a repo (like `ollama pull`)
patina repo add dojo --github dojoengine/dojo
patina repo add starknet-foundry --github foundry-rs/starknet-foundry
patina repo add bevy --github bevyengine/bevy --branch main

# What happens:
# 1. Clone repo to ~/.patina/repos/<name>/
# 2. Run scrape → ~/.patina/repos/<name>.db
# 3. Register in ~/.patina/registry.yaml

# List repos
patina repolist
# Output:
# NAME              GITHUB                        UPDATED       INDEXED
# dojo              dojoengine/dojo               2 hours ago   eventlog, fts5
# starknet-foundry  foundry-rs/starknet-foundry   5 days ago    eventlog, fts5

# Update a repo (git pull + rescrape)
patina repoupdate dojo
patina repoupdate --all

# Remove a repo
patina repo rm dojo

# Show repo details
patina reposhow dojo
# Output:
# Name: dojo
# GitHub: dojoengine/dojo
# Path: ~/.patina/repos/dojo
# Branch: main
# Last updated: 2025-11-25T14:45:00Z
# Domains: cairo, starknet, ecs, game-engine
# Events: 24,531
# Files: 847
# Functions: 3,241
```

### Project Management (`patina project`)

```bash
# Register current project with mothership
patina project add .
patina project add ~/Projects/other-project

# What happens:
# 1. Validates .patina/ exists (or runs init)
# 2. Adds to ~/.patina/registry.yaml
# 3. Records domains from scrape data

# List registered projects
patina project list
# Output:
# NAME          PATH                              TYPE      LAST ACTIVE
# patina        ~/Projects/patina                 primary   2 hours ago
# my-hackathon  ~/Projects/my-hackathon           primary   30 min ago

# Remove project from registry (data stays)
patina project rm patina

# Show project details
patina project show patina
```

### Service Daemon (`patina serve`)

```bash
# Start daemon (foreground)
patina serve

# Start daemon (background)
patina serve --daemon
patina serve -d

# Custom port
patina serve --port 8080

# Check if running
patina serve --status
# Output:
# Mothership: running on :50051
# Uptime: 2h 34m
# Projects loaded: 2 (patina, my-hackathon)
# Refs loaded: 1 (dojo)
# Memory: 124MB

# Stop daemon
patina serve --stop
```

### Cross-Project Scry

```bash
# Local project query (existing, works without daemon)
patina scry "error handling"

# Query specific project (requires daemon OR direct path)
patina scry "spawn patterns" --project dojo
patina scry "test setup" --project starknet-foundry

# Query all repos
patina scry "entity component" --repos

# Query multiple projects
patina scry "error handling" --projects patina,dojo

# Query with dimension
patina scry --file src/game.cairo --project my-hackathon --dimension temporal
```

---

## Daemon Behavior (WWOD Pattern)

### Startup

```
patina serve
  │
  ├─▶ Read ~/.patina/registry.yaml
  │     • Validate project paths exist
  │     • Validate repo paths exist
  │     • Log warnings for missing paths
  │
  ├─▶ Load E5 model into memory (once)
  │     • ~/.patina/cache/models/e5-base-v2/
  │     • ~500MB RAM
  │
  ├─▶ Start REST server on :50051
  │     • /health
  │     • /scry
  │     • /embed
  │     • /projects
  │     • /repos
  │
  └─▶ Wait for requests (lazy loading)
        • DBs NOT loaded yet
        • Indices NOT loaded yet
```

### First Query (Lazy Load)

```
POST /scry { query: "spawn", project: "dojo" }
  │
  ├─▶ Check cache: dojo.db loaded? NO
  │
  ├─▶ Load dojo.db connection
  │     • ~/.patina/repos/dojo.db
  │     • Add to cache with timestamp
  │
  ├─▶ Check: has FTS5? YES
  │     • Query pattern looks exact? Route to FTS5
  │     • Otherwise route to semantic
  │
  ├─▶ Execute query
  │
  └─▶ Return results
        • Mark cache entry as "hot"
```

### Keep-Alive and Eviction

```
Environment: PATINA_KEEP_ALIVE=5m (default)

Every 60 seconds:
  │
  ├─▶ Check all cached connections
  │
  ├─▶ For each connection:
  │     • Last used > KEEP_ALIVE ago?
  │     • YES → Close connection, free memory
  │     • NO → Keep hot
  │
  └─▶ Log evictions
```

### Environment Variables

```bash
PATINA_HOME=~/.patina           # Data directory (default: ~/.patina)
PATINA_PORT=50051               # Server port (default: 50051)
PATINA_KEEP_ALIVE=5m            # Connection keep-alive (default: 5m)
PATINA_MAX_CONNECTIONS=10       # Max concurrent DB connections
PATINA_LOG_LEVEL=info           # Logging level
```

---

## Query Routing

### Router Logic

```rust
pub fn route_query(query: &ScryRequest) -> QueryPlan {
    // 1. Determine target(s)
    let targets = match (&query.project, &query.projects, query.repos) {
        (Some(p), _, _) => vec![Target::Project(p.clone())],
        (_, Some(ps), _) => ps.iter().map(|p| Target::Project(p.clone())).collect(),
        (_, _, true) => registry.repos.keys().map(|r| Target::Ref(r.clone())).collect(),
        _ => vec![Target::CurrentProject],
    };

    // 2. Determine query type per target
    let queries: Vec<_> = targets.iter().map(|t| {
        let db = get_db_for_target(t);

        if query.file.is_some() {
            // File-based query → temporal/dependency
            QueryType::FileBased {
                dimension: query.dimension.unwrap_or("temporal")
            }
        } else if looks_like_exact_match(&query.query) {
            // Exact match patterns → FTS5
            QueryType::Lexical
        } else {
            // Natural language → semantic
            QueryType::Semantic
        }
    }).collect();

    QueryPlan { targets, queries }
}

fn looks_like_exact_match(query: &str) -> bool {
    // Patterns that trigger FTS5
    query.starts_with("find ") ||
    query.contains("::") ||           // Rust paths
    query.contains("()") ||           // Function calls
    query.contains("where is") ||
    query.contains("defined") ||
    is_snake_case(query) ||           // variable_name
    is_pascal_case(query)             // TypeName
}
```

### Result Merging

```rust
pub fn merge_results(results: Vec<(Target, Vec<ScryResult>)>) -> Vec<TaggedResult> {
    let mut merged = Vec::new();

    for (target, hits) in results {
        let tag = match target {
            Target::CurrentProject => "[LOCAL]",
            Target::Project(name) => &format!("[PROJECT:{}]", name),
            Target::Ref(name) => &format!("[REF:{}]", name),
        };

        for hit in hits {
            merged.push(TaggedResult {
                tag: tag.to_string(),
                source: target.name(),
                content: hit.content,
                file_path: hit.file_path,
                similarity: hit.similarity,
            });
        }
    }

    // Sort by similarity, interleaving sources
    merged.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
    merged
}
```

---

## REST API

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | /health | Health check |
| GET | /status | Daemon status, loaded DBs, memory |
| POST | /scry | Cross-project query |
| POST | /embed | Generate embeddings |
| GET | /projects | List registered projects |
| POST | /projects | Register project |
| DELETE | /projects/:name | Unregister project |
| GET | /repos | List repos |
| POST | /repos | Add repo |
| DELETE | /repos/:name | Remove repo |

### POST /scry

**Request:**
```json
{
  "query": "entity spawning patterns",
  "project": "dojo",
  "dimension": "semantic",
  "limit": 10
}
```

**Response:**
```json
{
  "results": [
    {
      "tag": "[REF:dojo]",
      "source": "dojo",
      "content": "pub fn spawn_entity(...)",
      "file_path": "crates/dojo-core/src/world.cairo",
      "line": 142,
      "similarity": 0.89
    }
  ],
  "query_type": "semantic",
  "sources_queried": ["dojo"],
  "timing_ms": 45
}
```

---

## Hackathon MVP Scope

### Must Have (v0.1)

| Feature | Why | Effort |
|---------|-----|--------|
| `patina repo add` | Get reference patterns indexed | 2 days |
| `patina repolist` | See what's available | 0.5 day |
| `--project` flag on scry | Query specific repo | 1 day |
| Registry file | Track projects and repos | 0.5 day |

**Total: ~4 days**

### Defer to v0.2

| Feature | Why Defer |
|---------|-----------|
| `patina serve` daemon | Direct DB access works for MVP |
| REST API | CLI is sufficient |
| Persona/beliefs | Not needed for hackathon |
| Alignment tracking | Nice-to-have |
| `patina project add` | Can use repos workflow |

### MVP Implementation Notes

For hackathon MVP, scry can work **without daemon**:

```rust
// In scry command, check for --project flag
if let Some(project_name) = &options.project {
    // Look up in registry
    let registry = load_registry()?;

    if let Some(repo_entry) = registry.repos.get(project_name) {
        // Query repo's database directly
        let db_path = repo_entry.db.clone();
        return scry_database(&db_path, query, options);
    }

    if let Some(project_entry) = registry.projects.get(project_name) {
        // Query project's database directly
        let db_path = project_entry.path.join(".patina/data/patina.db");
        return scry_database(&db_path, query, options);
    }

    bail!("Unknown project or repo: {}", project_name);
}

// Default: query current project
scry_local(query, options)
```

---

## Future: Container Integration (v0.3+)

When daemon exists, containers reach Mac via:

```yaml
# .devcontainer/docker-compose.yml
services:
  dev:
    environment:
      - PATINA_MOTHERSHIP=host.docker.internal:50051
```

Container's patina CLI checks `PATINA_MOTHERSHIP`:
- If set → proxy scry requests to mothership
- If not set → local mode only

---

## Acceptance Criteria

### MVP (v0.1)
- [ ] `patina repo add dojo --github dojoengine/dojo` clones and scrapes
- [ ] `patina repolist` shows registered repos
- [ ] `patina scry "query" --project dojo` queries repo's database
- [ ] Registry persists across sessions (`~/.patina/registry.yaml`)
- [ ] `patina repoupdate dojo` pulls and rescrapes
- [ ] `patina repo rm dojo` removes repo

### Full (v0.2)
- [ ] `patina serve` starts daemon
- [ ] REST API functional
- [ ] `patina project add .` registers projects
- [ ] Lazy loading with KEEP_ALIVE eviction
- [ ] `--projects` flag for multi-project queries
- [ ] `--repos` flag for querying all repos

### Container Ready (v0.3)
- [ ] `PATINA_MOTHERSHIP` env var support
- [ ] Container can query Mac mothership
- [ ] Cross-platform socket/port configuration
