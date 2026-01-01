# Spec: Three-Layer Architecture

**Status**: Workshop
**Created**: 2025-12-29
**Updated**: 2025-12-30
**Purpose**: Define the separation of concerns across mother, patina, and awaken

---

## The Three Layers

| Layer | Need | Focus |
|-------|------|-------|
| **mother** | Infrastructure | Central orchestration, user identity, coordination |
| **patina** | Product | Knowledge extraction and retrieval (RAG) |
| **awaken** | Shipping | Build, deploy, make it run |

This is a responsibility separation, not necessarily a binary separation (that's an implementation detail).

---

## What Exists Today

All commands currently live in the `patina` binary. Here's where they belong by responsibility:

### mother (infrastructure)

Central services, user identity, cross-project coordination.

| Command | Lines | Status | Notes |
|---------|------:|--------|-------|
| `serve` | 303 | Exists | Daemon, MCP server |
| `secrets` | 325 + 1,764 lib | Exists | Vault, identity, recipients |
| `persona` | 609 | Exists | User knowledge capture |
| `repo` | 1,126 | Exists | External repo registry |
| `model` | 211 | Exists | Embedding model management |
| `adapter` | 363 | Exists | LLM configuration |

**Location**: `~/.patina/`

**Observation**: These exist and work. They're scattered in the patina binary but functionally complete.

### patina (product)

Knowledge extraction, embedding, structural analysis, retrieval. The core.

| Command | Lines | Status | Notes |
|---------|------:|--------|-------|
| `scrape` | 5,600+ | Exists | Extract facts from code/git/sessions |
| `oxidize` | 1,604 | Exists | Build embeddings |
| `assay` | 1,058 | Exists | Structural signals and queries |
| `scry` | 2,077 | Exists | Query knowledge (the oracle) |
| `doctor` | 278 | Exists | Project health checks |
| `rebuild` | 259 | Exists | Reconstruct derived data |
| `init` | 1,080 | Exists | Initialize project |
| `eval` | 596 | Exists | Retrieval quality measurement |
| `bench` | 448 | Exists | Benchmarking |

**Location**: `project/.patina/`

**Observation**: This is the product. Most complete because it's what we're building.

### awaken (shipping)

Build, test, deploy, make it run. The action layer.

| Command | Lines | Status | Notes |
|---------|------:|--------|-------|
| `yolo` | 1,613 | Exists | Container generation |
| `build` | 32 | Exists | Thin wrapper |
| `test` | 31 | Exists | Thin wrapper |
| `deploy` | - | Missing | The gap |

**Location**: Containers, CI, production

**Observation**: Sparse. yolo exists but deploy doesn't. This is the "missing vercel" - the layer that takes marked projects and ships them.

---

## The Relationship

```
mother          patina          awaken
(infra)    →    (know)     →    (ship)
   │               │               │
   │               │               │
~/.patina/    .patina/      containers/prod
```

- **mother** provides: identity, secrets, coordination, daemon
- **patina** provides: knowledge about the project
- **awaken** consumes: knowledge + infra to ship

---

## Open Questions

### 1. Binary separation?

Should these become three binaries, or stay as one binary with clear internal boundaries?

**Arguments for three binaries:**
- Clear user mental model (`mother serve`, `patina scry`, `awaken deploy`)
- Can install only what you need
- Forces clean interfaces

**Arguments for one binary:**
- Simpler distribution
- Shared code is easier
- Users already know `patina`

**Current lean**: TBD. The responsibility separation matters more than the binary separation.

### 2. What does awaken actually need?

The gap is clear but the solution isn't. Options:

- `awaken deploy` - push to where?
- `awaken ci` - integrate with what?
- `awaken run` - local or remote?

This needs design work. yolo generates containers, but what's the workflow from there?

### 3. mother consolidation

mother commands exist but are accessed via `patina`. Should they:
- Stay as `patina serve`, `patina secrets`, etc.?
- Move to `mother serve`, `mother secrets`, etc.?
- Something else?

---

## Not Decided Yet

- Binary structure (one vs three)
- awaken feature set
- mother command migration
- CLI naming conventions

This spec grounds the vision in what exists. Implementation decisions come after clarity on the above.

---

## References

- [spec-architectural-alignment.md](./spec-architectural-alignment.md) - Internal code quality
- [spec-pipeline.md](./spec-pipeline.md) - Knowledge pipeline (patina's core)
- [dependable-rust.md](../../core/dependable-rust.md) - Module pattern
- [unix-philosophy.md](../../core/unix-philosophy.md) - Single responsibility
