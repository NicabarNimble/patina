# Spec: Model Management

**Phase:** 2
**Status:** In Progress (2a-2e complete, 2f remaining)
**Goal:** Base models are infrastructure managed at mothership level. Projects reference by name, mothership provides files.

---

## Problem

Currently base models live in the source repo (`resources/models/`):

| Issue | Impact |
|-------|--------|
| 400MB+ in repo | Every clone downloads all models |
| No sharing | Same model duplicated per-project |
| Manual sync | 3 files must agree (registry, config, recipe) |
| No provenance | Don't know where models came from |

## Solution

Mothership owns base models. Projects reference by name.

```
┌─────────────────────────────────────────────────────────────┐
│ MOTHERSHIP (~/.patina/)                                     │
├─────────────────────────────────────────────────────────────┤
│ cache/models/e5-base-v2/                                    │
│   ├── model.onnx         # Downloaded from HuggingFace      │
│   └── tokenizer.json     # 110MB total, shared              │
│                                                             │
│ models.lock              # Provenance record                │
│   [e5-base-v2]                                              │
│   downloaded = "2025-12-16T19:30:00Z"                       │
│   source = "https://huggingface.co/..."                     │
│   sha256 = "abc123..."                                      │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│ PROJECT (.patina/)                                          │
├─────────────────────────────────────────────────────────────┤
│ config.toml                                                 │
│   [embeddings]                                              │
│   model = "e5-base-v2"   # Just a name reference            │
│                                                             │
│ data/embeddings/e5-base-v2/projections/                     │
│   ├── semantic.safetensors   # Trained adaptations          │
│   └── semantic.usearch       # (stay per-project)           │
└─────────────────────────────────────────────────────────────┘
```

---

## Design Principles

### Unix Philosophy
`patina model` is one focused tool:
- Download models from registry URLs
- Track provenance in lock file
- Validate project references

### Dependable Rust
Single source of truth chain:
```
registry.toml (in binary)  →  What models exist
     ↓
models.lock (mothership)   →  What's downloaded + provenance
     ↓
config.toml (project)      →  What model this project uses
     ↓
embeddings module          →  Reads model from mothership cache
```

### Adapter Pattern
Models are external resources. We track our interface to them:
- Source URL (where we got it)
- Checksum (integrity verification)
- Download date (audit trail)

---

## Size Context

**Base models (shared, downloaded once):**
| Model | Size | Dimensions | Use Case |
|-------|------|------------|----------|
| all-minilm-l6-v2 | 109 MB | 384 | Fast, general-purpose |
| bge-small-en-v1.5 | 33 MB | 384 | Retrieval, size-constrained |
| e5-base-v2 | 106 MB | 768 | Question-answering |
| bge-base-en-v1.5 | 173 MB | 768 | SOTA retrieval |
| nomic-embed-text-v1.5 | 132 MB | 768 | Long-form, 8K context |

**Projections (per-project, trained by oxidize):**
| Type | Weights | Index | Purpose |
|------|---------|-------|---------|
| semantic | 4 MB | 4 MB | Session/code similarity |
| temporal | 4 MB | 2 MB | Co-change relationships |
| dependency | 4 MB | 6 MB | Call graph relationships |

Base models = infrastructure (~100MB each, shared).
Projections = knowledge (~20MB total, per-project).

---

## Commands

### `patina model list`

Show registry models with download status:

```
$ patina model list

Available models:
  ✓ all-minilm-l6-v2     384 dims   109 MB  (downloaded 2025-12-15)
  ✓ e5-base-v2           768 dims   106 MB  (downloaded 2025-12-16)
    bge-base-en-v1.5     768 dims   173 MB  (not downloaded)
    nomic-embed-text     768 dims   132 MB  (not downloaded)

Cache: ~/.patina/cache/models/ (215 MB used)
```

### `patina model add <name>`

Download model and record provenance:

```
$ patina model add bge-base-en-v1.5

Downloading bge-base-en-v1.5...
  Source: https://huggingface.co/neuralmagic/bge-base-en-v1.5-quant/...
  Size: 173 MB
  [████████████████████████] 100%

Verifying checksum... ✓
Recording provenance... ✓

✓ Model added: bge-base-en-v1.5
  Location: ~/.patina/cache/models/bge-base-en-v1.5/
```

### `patina model remove <name>`

Remove from cache (keeps lock record for re-download):

```
$ patina model remove bge-base-en-v1.5

Remove bge-base-en-v1.5 from cache? (173 MB) [y/N]: y
✓ Removed from cache
  Note: Run `patina model add bge-base-en-v1.5` to re-download
```

### `patina model status`

Show what projects need vs what's available:

```
$ patina model status

Mothership cache:
  ✓ all-minilm-l6-v2 (109 MB)
  ✓ e5-base-v2 (106 MB)

Projects using models:
  ~/projects/patina     → e5-base-v2 ✓
  ~/projects/dojo       → e5-base-v2 ✓
  ~/projects/webapp     → bge-base-en-v1.5 ✗ (not downloaded)

Action needed:
  patina model add bge-base-en-v1.5
```

---

## Lock File Format

`~/.patina/models.lock` - Provenance record (auto-generated):

```toml
# Patina Model Lock File
# Auto-generated - do not edit manually
# Re-download with: patina model add <name>

[e5-base-v2]
downloaded = "2025-12-16T19:30:00Z"
source_model = "https://huggingface.co/intfloat/e5-base-v2/resolve/main/onnx/model_qint8_avx512_vnni.onnx"
source_tokenizer = "https://huggingface.co/intfloat/e5-base-v2/resolve/main/tokenizer.json"
sha256_model = "abc123def456..."
sha256_tokenizer = "789ghi012jkl..."
size_bytes = 110_000_000
dimensions = 768

[all-minilm-l6-v2]
downloaded = "2025-12-15T10:00:00Z"
source_model = "https://huggingface.co/Xenova/all-MiniLM-L6-v2/resolve/main/onnx/model_quantized.onnx"
source_tokenizer = "https://huggingface.co/Xenova/all-MiniLM-L6-v2/resolve/main/tokenizer.json"
sha256_model = "..."
sha256_tokenizer = "..."
size_bytes = 23_000_000
dimensions = 384
```

---

## Integration Points

### Init Flow

```
patina init .
  → Read project config: model = "e5-base-v2"
  → Check mothership: ~/.patina/models.lock
  → If not downloaded:
      "Model 'e5-base-v2' not found. Download now? [Y/n]"
      → patina model add e5-base-v2
  → Proceed with scrape/oxidize
```

### Embeddings Module

```rust
// Current: reads from resources/models/{name}/
let model_dir = Path::new(&model_def.path);  // "resources/models/e5-base-v2"

// New: reads from ~/.patina/cache/models/{name}/
let model_dir = patina::paths::mothership_model_cache(&model_def.name)?;
```

### Oxidize

Recipe no longer needs `embedding_model` - derives from project config:

```yaml
# Before (oxidize.yaml) - v1 format
version: 1
embedding_model: e5-base-v2  # Redundant, can drift
projections:
  semantic:
    layers: [768, 1024, 256]  # 768 hardcoded, must match model
    epochs: 10
    batch_size: 32

# After (oxidize.yaml) - v2 format (implemented)
version: 2
# embedding_model: optional, falls back to config.toml
projections:
  semantic:
    layers: [1024, 256]  # Just [hidden, output], input_dim from registry
    epochs: 10
    batch_size: 32
```

**Note:** v2 uses a simple array `[hidden, output]` rather than named keys. The input dimension is derived from the model registry based on the model configured in `.patina/config.toml`.

---

## Migration

### For patina repo itself

1. Move `resources/models/*.onnx` to gitignore
2. Keep `resources/models/registry.toml` (metadata only)
3. CI downloads models on-demand
4. README documents first-time setup

### For existing projects

On first run after upgrade:
```
$ patina oxidize

Model 'e5-base-v2' not in mothership cache.
Migrating from resources/models/...
  → Copied to ~/.patina/cache/models/e5-base-v2/
  → Recorded provenance in ~/.patina/models.lock

✓ Migration complete. You can delete resources/models/ from your project.
```

---

## Tasks

### 2a: Mothership Model Cache ✅
- [x] Create `~/.patina/cache/models/` directory structure
- [x] Add `models.lock` TOML format + parser
- [x] Update `src/embeddings/models.rs` to read from cache
- [x] Add `patina::paths::models` module with cache helpers

**Implementation:**
- `src/models/mod.rs` - Public API: `resolve_model_path()`, `add_model()`, `model_status()`
- `src/models/internal.rs` - `ModelLock`, `LockedModel` types with TOML serde
- `src/paths.rs::models` - `cache_dir()`, `model_dir()`, `model_onnx()`, `model_tokenizer()`, `lock_path()`

### 2b: Model Command ✅
- [x] `patina model list` - show registry + download status
- [x] `patina model add <name>` - download with progress bar
- [x] `patina model remove <name>` - remove from cache
- [x] `patina model status` - show project needs vs cache

**Implementation:**
- `src/commands/model.rs` - CLI subcommands via clap
- Downloads quantized INT8 models (~30-50MB vs ~100-170MB full)

### 2c: Download Infrastructure ✅
- [x] HTTP download with progress (reqwest)
- [x] SHA256 verification
- [x] Provenance recording to lock file
- [ ] Resume interrupted downloads (deferred - not critical)

**Implementation:**
- `src/models/download.rs` - `download_file()`, `sha256_file()`, `download_and_verify()`
- Uses `shasum -a 256` for verification (macOS built-in)
- Note: Could use `sha2` crate for cross-platform, but shasum works and avoids dependency

### 2d: Init Integration ✅
- [x] Check model availability on init
- [x] Prompt to download if missing
- [x] Validate project model against registry

**Implementation:**
- `src/commands/init/internal/mod.rs::ensure_model_available()` - Called before oxidize step
- Interactive Y/n prompt with sensible default (Y)

### 2e: Oxidize Updates ✅
- [x] Derive `input_dim` from registry (not recipe)
- [x] Recipe v2 format (optional `embedding_model`)
- [x] Backwards compat with v1 recipes

**Implementation:**
- `src/commands/oxidize/recipe.rs` - `OxidizeRecipe` with optional `embedding_model`
- Recipe v2: layers can be `[hidden, output]` instead of `[input, hidden, output]`
- `get_model_name()` falls back to `config.toml` if not in recipe
- `input_dim(&recipe)` derives from registry for v2, from layers[0] for v1

### 2f: Migration Path
- [ ] Detect models in `resources/models/`
- [ ] Copy to mothership cache
- [ ] Record provenance
- [ ] Update gitignore guidance

---

## Validation (Exit Criteria)

| Criteria | Status |
|----------|--------|
| `patina model list` shows registry + status | [x] |
| `patina model add` downloads with provenance | [x] |
| Models stored in `~/.patina/cache/models/` | [x] |
| `models.lock` tracks downloads + checksums | [x] |
| Init validates model availability | [x] |
| Oxidize derives dimensions from registry | [x] |
| Existing projects can migrate | [ ] |
| `resources/models/` can be gitignored | [ ] |

**Progress:** 6/8 criteria met. Migration path (2f) is the remaining work.

---

## Future Considerations

**Not in scope for Phase 2:**

- Model versioning (e.g., e5-base-v2.1)
- Custom model URLs (user-provided models)
- Container embedding service (daemon provides embeddings)
- Multi-platform optimization (CoreML vs CUDA selection)

These can be Phase 3+ once base infrastructure is solid.
