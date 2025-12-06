# Patina: RAG Network Architecture

**Crystallized:** 2025-12-04
**Session:** 20251204-173633

---

## Key Insight: Patina Identity

**Patina is an LLM-agnostic agentic RAG network that captures organic knowledge across projects and domains.**

The name "Patina" comes from the protective layer that forms on metal over time - your development wisdom accumulates and transfers between projects.

---

## Architecture

### 1. Projects as RAG Nodes

Each project is a knowledge node:
- `layer/` = Patina content (patterns, sessions, learnings) - **git-tracked**
- `.patina/` = Config and local data (indices, embeddings) - **gitignored**

### 2. Mothership as Hub

`~/.patina/` contains:
- `persona/` - Cross-project beliefs and facts
- `registry.yaml` - All known nodes
- `cache/models/` - Shared model cache
- `repos/` - External repos (learning + contributing)

### 3. Domains as Tags

Nodes tagged with domains for cross-project queries:
- rust: [patina, bevy, starknet-foundry]
- cairo: [dojo, starknet]
- ecs: [dojo, bevy]

### 4. Branch Strategy (Owner vs Contributor)

**Owner repos (your code):**
```
main/master:
├── src/
├── layer/           # Patina content lives here
│   ├── core/        # Eternal patterns
│   ├── surface/     # Active work
│   ├── dust/        # Archived
│   └── sessions/    # Learnings
├── .patina/         # Config + local indices
└── CLAUDE.md        # LLM adapter
```

**Contributor repos (others' code):**
```
upstream/main:       # Clean, their code, for PRs
├── src/
└── (no layer/)

patina branch:       # YOUR overlay
├── src/             # Their code
├── layer/           # YOUR learnings about their code
│   └── sessions/    # Your study sessions
└── .patina/         # YOUR config for this repo

Workflow:
- Work on feature-branch from main
- PR to upstream (clean, no layer/)
- Merge learnings to patina branch
```

### 5. Data Flow

- **UP:** Learnings flow from project → Mothership persona
- **DOWN:** Knowledge flows only through explicit queries

---

## Source Architecture (Bundles + Modules)

### Core Sources (universal, always available)
- `git/` - Commits, branches, co-changes
- `code/` - AST, functions, call graph (tree-sitter)
- `sessions/` - layer/sessions/*.md

### Optional Source Adapters (pluggable)
- `github/` - Issues, PRs, discussions
- `jira/` - Future
- `linear/` - Future

### Modules on Adapters (use-case specific)
- `github/modules/bounty/` - Bounty detection (Algora, OnlyDust)
- `github/modules/ci-status/` - CI integration

**Key insight:** Bounty detection is a MODULE on the GitHub adapter, not a core feature.

---

## Model Management (Hybrid Design)

**Projects declare DIMENSIONS (what they need):**
```yaml
# project/.patina/oxidize.yaml
dimensions:
  - semantic     # Same session = related
  - temporal     # Same commit = related
  - dependency   # Caller/callee = related
```

**Mothership provides MODELS (how to implement):**
```toml
# ~/.patina/config.toml
[models.semantic]
provider = "e5-base-v2"
backend = "onnx"     # or "mlx" on Apple Silicon
```

This is the adapter pattern applied to models.

---

## Key Quotes

> "Patina is an organic agentic RAG that captures knowledge across projects and domains"

> "Projects are islands, personas are gods. Knowledge flows UP (project → persona). Knowledge flows DOWN only through explicit requests."

> "Mothership is a librarian, not a library. It tracks where knowledge lives, doesn't duplicate it."
