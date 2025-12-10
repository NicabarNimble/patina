---
id: concept-rag-network
status: emerging
created: 2025-12-04
updated: 2025-12-09
oxidizer: nicabar
tags: [concept, rag, architecture, knowledge-graph]
references: [architecture-patina-system]
---

# Patina: RAG Network Architecture

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
- `personas/` - Cross-project knowledge (see spec-persona-capture.md)
- `registry.yaml` - All known projects and reference repos
- `cache/models/` - Shared model cache
- `repos/` - Reference repos (read-only knowledge bases)

### 3. Domains as Tags

Nodes tagged with domains for cross-project queries:
- rust: [patina, bevy, starknet-foundry]
- cairo: [dojo, starknet]
- ecs: [dojo, bevy]

### 4. Patina Projects

All code you work on (owner or contributor) is a Patina project:

```
<project>/
├── src/
├── layer/           # Patina content lives here
│   ├── core/        # Eternal patterns
│   ├── surface/     # Active work
│   ├── dust/        # Archived
│   └── sessions/    # Learnings
├── .patina/         # Config + local indices (gitignored)
└── CLAUDE.md        # LLM adapter
```

**Owner vs Contributor** is a git remote configuration, not a Patina concern:
- Owner: push to origin/main
- Contributor: push to fork, PR to upstream

Patina treats both the same - full RAG, sessions, all dimensions.

### 5. Reference Repos

Read-only knowledge bases (code you learn from, not work on):

```
~/.patina/repos/<name>/
├── src/             # Their code (shallow clone)
├── .patina/         # Lightweight index
│   ├── data/patina.db
│   └── config.toml
└── (no layer/)      # No sessions, no learnings
```

Reference repos get: code AST, call graph, FTS5, dependency dimension.
Reference repos don't get: sessions, temporal, semantic dimensions.

### 6. Data Flow

- **UP:** Knowledge flows from projects → Mothership personas
- **DOWN:** Knowledge flows only through explicit queries (scry)
- Reference repos don't contribute to persona (read-only knowledge sources)

### 7. Data Availability Principle

**Core insight (session 20251206-221156):** What data is available determines which dimensions work.

| Data Source | Dimension Enabled | Who Has It |
|-------------|-------------------|------------|
| `layer/sessions/` | Semantic | Patina projects only |
| Full git history | Temporal | Patina projects only |
| Call graph (AST) | Dependency | Both projects and reference repos |
| Code symbols | FTS5 lexical | Both projects and reference repos |

**Why this matters:**
- Reference repos are shallow clones with no sessions → they can't have semantic or temporal dimensions
- This isn't a limitation to fix; it's the design. Reference repos are for learning patterns, not capturing your work
- Trying to make reference repos "full RAG" was architectural drift - the two-tier model exists because the data sources are fundamentally different

**Implication for queries:**
- `patina scry` on a project: semantic + temporal + dependency + FTS5
- `patina scry --repo` on reference: dependency + FTS5 only

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

> "Projects are islands, personas are the user. Knowledge flows UP (project → persona). Knowledge flows DOWN only through explicit queries."

> "Mothership is a librarian, not a library. It tracks where knowledge lives, doesn't duplicate it."
