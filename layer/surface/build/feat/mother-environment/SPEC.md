---
type: feat
id: mother-environment
status: design
created: 2026-02-09
sessions:
  origin: 20260209-215657
  reviewed: 20260210-061323
related:
  - layer/surface/build/feat/mother-architecture/SPEC.md
  - layer/surface/build/feat/model-runtime/SPEC.md
  - layer/surface/build/feat/mother-repos/SPEC.md
beliefs:
  - mother-is-the-daemon
  - mother-owns-ref-repo-indexing
  - four-layer-architecture
---

# feat: Models Child — Embedding Model Ownership

> A `MotherChild` that owns embedding models centrally at `~/.patina/cache/models/`.
> Projects resolve models through Mother, not local `resources/`. Fixes the
> ownership boundary where projects manage user-level infrastructure.

## Problem

### Models Are User-Level, Managed Project-Level

Embedding models are shared across all projects but managed per-project:

| Location | Size | Who manages |
|----------|------|-------------|
| `~/.patina/cache/models/` | 139MB (2 models) | `resolve_model_path()` fallback |
| `resources/models/` | gitignored, 5 models | Project tree (not tracked) |

`resolve_model_path()` at `src/models/mod.rs:101` already checks the Mother
cache first, falling back to project-local. The precedence is right but the
fallback shouldn't exist — models belong to Mother.

### Registry Is Compile-Time Embedded

`src/embeddings/models.rs:53` uses `include_str!()` to embed
`resources/models/registry.toml` at compile time. Moving the registry to
Mother's cache means changing from compile-time to runtime loading.

### 6 Cold-Start Sites

`create_embedder()` is called from 6 locations across the codebase. Each one
cold-starts an ONNX session (~500ms). The MCP server already works around
this with `OnceLock` in `SemanticOracle`, but CLI commands pay the cost
every invocation.

### No Vector Space Safety

Nothing tags `.usearch` indexes with the model that produced them. When
models change (E5-base-v2 → Qwen3), indexes become silently incompatible.
Today everyone uses E5-base-v2 so this hasn't broken. The first model swap
will produce silent retrieval failures across `patina scry --all-repos`
(which merges scores from 20+ repos with zero embedding validation).

## As a MotherChild

```
name()   → "models"
state    → ~/.patina/cache/models/ (cache — rebuildable)
           ~/.patina/cache/models/registry.toml (portable)
```

**`on_load()`**: Load registry, warm default `EmbeddingEngine` in RAM.

**`handle()`**:
- `embed_query(text)` → Vec<f32> (from warm engine)
- `embed_passage(text)` → Vec<f32>
- `resolve_model(name)` → path to model directory
- `spec(name)` → EmbeddingSpec (id, dim, normalize, prefixes)

**`health()`**: Default model present? Engine loaded? Registry readable?

**`tick()`**: No-op. Models don't change between heartbeats.

## Key Design: EmbeddingSpec + meta.json

```rust
pub struct EmbeddingSpec {
    pub id: String,            // "e5-base-v2@onnx"
    pub dim: usize,            // 768
    pub normalize: bool,
    pub query_prefix: String,  // "query: "
    pub passage_prefix: String // "passage: "
}
```

Every `.usearch` index gets a sibling `meta.json`:
```json
{
  "embedding_id": "e5-base-v2@onnx",
  "dim": 768,
  "created_at": "2026-02-09T23:00:00Z"
}
```

Scry validates `meta.embedding_id == backend.spec.id` before querying.
Mismatch → crisp error: `"Index built with X, backend is Y. Run: patina oxidize"`.

## Acceptance Criteria

1. [ ] Registry loaded at runtime from `~/.patina/cache/models/registry.toml` (not `include_str!()`)
2. [ ] `resolve_model_path()` resolves exclusively from Mother cache
3. [ ] Every `.usearch` index has sibling `meta.json` with `embedding_id` and `dim`
4. [ ] `scry` validates `meta.embedding_id` matches current backend before querying
5. [ ] `MotherChild` trait implemented: `handle()` serves embed requests to warm engine
6. [ ] `patina model` commands manage Mother cache (existing CLI, verify works)

## Non-Goals

- Model download from HuggingFace (manual for now, automate later)
- MLX runtime (separate spec: [[model-runtime]])
- Hot-swapping models in the daemon (restart is fine)
- `oxidize_for_repo()` fix (that's [[mother-repos]] — repos child owns ref repo indexing)
