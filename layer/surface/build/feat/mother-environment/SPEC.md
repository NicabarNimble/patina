---
type: feat
id: mother-environment
status: design
created: 2026-02-09
sessions:
  origin: 20260209-215657
related:
  - layer/surface/build/feat/mother/SPEC.md
  - layer/surface/build/feat/mother-v2/SPEC.md
  - layer/surface/build/feat/model-runtime/SPEC.md
  - layer/surface/build/feat/mother-repos/SPEC.md
beliefs:
  - mother-is-the-daemon
  - mother-owns-ref-repo-indexing
  - four-layer-architecture
---

# feat: Mother Environment — Model & Runtime Ownership

> Mother owns embedding models centrally at `~/.patina/cache/models/`. Projects
> resolve models through Mother, not local `resources/`. Eliminates the 553MB
> duplication in the project tree and fixes the `oxidize_for_repo()` boundary
> violation where projects reach past Mother to configure shared infrastructure.

## Problem

### Models Are Duplicated and Project-Owned

Today embedding models exist in TWO places:

| Location | Size | Models | Who manages |
|----------|------|--------|-------------|
| `resources/models/` (project) | 553MB | 5 models | Project git tree |
| `~/.patina/cache/models/` (user) | 139MB | 2 models | `resolve_model_path()` fallback |

`resolve_model_path()` in `src/models/mod.rs:101` already checks the Mother
cache first, falling back to project-local. But the project still ships 553MB
of models in its git tree because nothing ensures the central cache is populated.

### `oxidize_for_repo()` Violates Ownership Boundaries

`src/commands/oxidize/mod.rs:124-196` — when oxidizing a reference repo:
1. Looks up repo path from registry
2. **Changes working directory** to that repo
3. **Symlinks the current project's `resources/`** into the target repo (for model access)
4. Runs the full oxidize pipeline
5. Cleans up and restores

Per [[mother-owns-ref-repo-indexing]], this is a boundary violation. The project
reaches into another repo's filesystem and injects its own model directory via
symlink. This works but couples ref repo indexing to whichever project happens
to run the command.

### No Vector Space Safety

When embedding models change (e.g., E5-base-v2 → Qwen3), existing indexes
become silently incompatible. Nothing tags vectors with the model that produced
them. Nothing prevents querying an index built with a different model.

Today this hasn't caused bugs because everyone uses E5-base-v2. But
[[model-runtime]] proposes model upgrades, and without space tagging, the first
model swap will produce silent retrieval failures.

## Current State (What Exists)

```
# Already exists:
~/.patina/cache/models/
├── e5-base-v2/           # 67MB — used by patina project
└── bge-small-en-v1-5/    # 72MB — from earlier experiments

# Also exists (project-local, should go away):
resources/models/
├── e5-base-v2/           # duplicate
├── bge-base-en-v1.5/
├── bge-small-en-v1.5/
├── all-minilm-l6-v2/
├── nomic-embed-text-v1.5/
├── registry.toml         # model definitions
└── tokenizer.json
```

```rust
// src/models/mod.rs:101 — already checks Mother cache first
pub fn resolve_model_path(name: &str) -> Result<PathBuf> {
    if let Some(path) = cached_model_path(name) { return Ok(path); }
    let local_path = PathBuf::from(format!("resources/models/{}", name));
    // ... fallback to local
}
```

```rust
// src/embeddings/mod.rs:21 — trait already has the right shape
pub trait EmbeddingEngine: Send {
    fn embed_query(&mut self, text: &str) -> Result<Vec<f32>>;
    fn embed_passage(&mut self, text: &str) -> Result<Vec<f32>>;
    fn dimension(&self) -> usize;
    fn model_name(&self) -> &str;
}
```

## Solution

### 1. Move `registry.toml` to Mother

Move the model registry from `resources/models/registry.toml` (project-local)
to `~/.patina/cache/models/registry.toml` (user-level). This is the source of
truth for which models exist and their specifications.

### 2. Add `EmbeddingSpec` to Model Identity

```rust
pub struct EmbeddingSpec {
    pub id: String,            // "e5-base-v2@onnx"
    pub dim: usize,            // 768
    pub normalize: bool,
    pub query_prefix: String,  // "query: "
    pub passage_prefix: String // "passage: "
}
```

Tag all index files with the spec that produced them. `meta.json` alongside
each `.usearch` index:

```json
{
  "embedding_id": "e5-base-v2@onnx",
  "dim": 768,
  "created_at": "2026-02-09T23:00:00Z",
  "corpus_fingerprint": "abc123"
}
```

### 3. Scry Validates Compatibility at Query Time

Before querying any index, check `meta.embedding_id == backend.spec.id`.
If mismatch, crisp error:

```
Index built with e5-base-v2@onnx (768), but backend is e5-large-v2@onnx (1024).
Run: patina oxidize
```

### 4. Eliminate Project-Local Models

Once the central cache is the authority:
- `patina init` ensures `~/.patina/cache/models/` has the configured model
- Remove `resources/models/` from the project git tree
- `oxidize_for_repo()` no longer needs the symlink hack

### 5. Daemon Warm Model Cache

Mother daemon loads the `EmbeddingEngine` once on startup. MCP and scry can
request embeddings through the daemon instead of cold-starting the ONNX runtime
per invocation. This was one of the original 4 motivations for the daemon
(500ms cold start → instant).

## Acceptance Criteria

1. [ ] `registry.toml` lives at `~/.patina/cache/models/registry.toml`
2. [ ] `resolve_model_path()` resolves exclusively from Mother cache (no project-local fallback)
3. [ ] `patina init` ensures model exists in Mother cache (downloads if needed)
4. [ ] Every `.usearch` index has a sibling `meta.json` with `embedding_id` and `dim`
5. [ ] `scry` validates `meta.embedding_id` matches current backend before querying
6. [ ] `oxidize_for_repo()` no longer symlinks `resources/` — models resolve through Mother
7. [ ] `resources/models/` removed from project git tree (existing projects: migration path)

## Non-Goals

- Model download from HuggingFace (manual for now, automate later)
- MLX runtime (separate spec: [[model-runtime]])
- Hot-swapping models in the daemon (restart is fine)
- Multiple simultaneous models per project (one model per project config)
