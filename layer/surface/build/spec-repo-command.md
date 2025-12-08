# Spec: patina repo

**Status:** In Progress (Phase 4b)
**Location:** `src/commands/repo/`

---

## Purpose

Manage **reference repos** - read-only knowledge bases for learning patterns from other codebases. Reference repos get lightweight indexing (code AST, call graph, FTS5), not full RAG.

**Not for code you work on.** If you want to contribute to a repo, use `patina init` on it instead.

---

## Commands

### patina repo add

```bash
patina repo <url>                 # Clone + scrape + index
patina repo <url> --with-issues   # Also scrape GitHub issues
```

**What happens:**
1. Shallow clone to `~/.patina/repos/<name>/`
2. Scaffold `.patina/` with config
3. Scrape codebase: symbols, functions, call graph
4. Build FTS5 index for lexical search
5. Register in `~/.patina/registry.yaml`

**What reference repos get:**
- Code AST and symbols (FTS5 lexical search)
- Call graph for dependency queries
- GitHub issues if `--with-issues`

**What reference repos don't get:**
- `layer/` directory (no sessions)
- Temporal dimension (shallow clone = no git history)
- Semantic dimension (no session data)

### patina repo list

```bash
patina repo list
```

**Output:**
```
NAME              GITHUB                        DOMAINS
dojo              dojoengine/dojo               cairo, ecs
bevy              bevyengine/bevy               rust, ecs
SDL               libsdl-org/SDL                c, graphics
```

### patina repo update

```bash
patina repo update <name>           # Pull + rescrape
patina repo update <name> --oxidize # Also build dependency index
patina repo update --all            # Update all repos
```

**`--oxidize` builds:**
- Dependency projection (call graph → vector space)
- Enables semantic-style queries on code structure

### patina repo show

```bash
patina repo show <name>
```

**Output:**
```
Name: dojo
GitHub: dojoengine/dojo
Path: ~/.patina/repos/dojo
Events: 24,531
Indexed: dependency
```

### patina repo remove

```bash
patina repo remove <name>
```

Removes from registry and deletes from `~/.patina/repos/`.

---

## Query Integration

```bash
# Query specific reference repo
patina scry "spawn patterns" --repo dojo

# Query all (projects + reference repos)
patina scry "entity component" --all-repos
```

Reference repos support:
- FTS5 lexical search (always)
- Dependency dimension (if `--oxidize` was run)

---

## Directory Structure

```
~/.patina/
├── registry.yaml           # Tracks projects + reference repos
└── repos/
    ├── dojo/
    │   ├── .patina/
    │   │   ├── data/
    │   │   │   ├── patina.db
    │   │   │   └── embeddings/    # If --oxidize
    │   │   ├── config.toml
    │   │   └── oxidize.yaml       # If --oxidize
    │   └── <source code>
    └── bevy/
        └── ...
```

---

## Registry Schema

```yaml
# ~/.patina/registry.yaml
version: 1

repos:
  dojo:
    path: ~/.patina/repos/dojo
    github: dojoengine/dojo
    registered: 2025-11-20T10:00:00Z
    domains: [cairo, starknet, ecs]

  bevy:
    path: ~/.patina/repos/bevy
    github: bevyengine/bevy
    registered: 2025-11-22T14:00:00Z
    domains: [rust, ecs, game-engine]
```

---

## Validation Criteria

- [x] `patina repo <url>` clones, scaffolds, scrapes
- [x] `patina repo list` shows registered repos
- [x] `patina repo update <name>` pulls and rescrapes
- [x] `patina scry --repo <name>` queries repo's database
- [x] Registry persists in `~/.patina/registry.yaml`
- [ ] `patina repo update --oxidize` builds dependency index
- [ ] `--with-issues` scrapes GitHub issues
