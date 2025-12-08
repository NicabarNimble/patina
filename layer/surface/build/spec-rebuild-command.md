# Spec: patina rebuild

**Status:** ✅ Complete (2025-12-06)
**Phase:** 4 (Core Infrastructure)
**Priority:** High - enables portability story

---

## Purpose

Regenerate `.patina/` from `layer/` and local sources for **Patina projects**. This is the "clone and go" command that makes projects portable.

**Applies to:** Patina projects (code you work on, created via `patina init`)
**Not for:** Reference repos (use `patina repo update --oxidize` instead)

**Use cases:**
1. Clone a project with `layer/` → `patina rebuild` → working local RAG
2. Corrupted `.patina/data/` → `patina rebuild` → fresh indices
3. Upgrade embedding model → `patina rebuild` → new projections

---

## Design

### What Gets Rebuilt

```
.patina/
├── data/
│   ├── patina.db           # Unified eventlog (from scrape)
│   ├── embeddings/
│   │   └── e5-base-v2/
│   │       └── projections/
│   │           ├── semantic.safetensors
│   │           ├── semantic.usearch
│   │           ├── temporal.safetensors
│   │           ├── temporal.usearch
│   │           ├── dependency.safetensors
│   │           └── dependency.usearch
│   └── *.db                # Legacy DBs (git.db, sessions.db, code.db)
```

### What's Preserved

```
.patina/
├── config.toml             # User config (NOT rebuilt)
├── oxidize.yaml            # Recipe (NOT rebuilt, source of truth)
├── config.json             # Legacy (preserved)
└── versions.json           # Version tracking (preserved)
```

### Source of Truth

| Output | Source |
|--------|--------|
| patina.db | `.git/`, `layer/sessions/*.md`, `src/**/*` |
| semantic projection | patina.db sessions + oxidize.yaml recipe |
| temporal projection | patina.db co_changes + oxidize.yaml recipe |
| dependency projection | patina.db call_graph + oxidize.yaml recipe |

---

## Implementation

### Command Interface

```bash
patina rebuild              # Full rebuild
patina rebuild --scrape     # Only scrape (skip oxidize)
patina rebuild --oxidize    # Only oxidize (assume db exists)
patina rebuild --force      # Delete existing data first
patina rebuild --dry-run    # Show what would be rebuilt
```

### Pipeline Steps

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Validate  │───▶│   Scrape    │───▶│   Oxidize   │
│   (layer/)  │    │ (git,sess,  │    │ (projections│
│             │    │  code)      │    │  + indices) │
└─────────────┘    └─────────────┘    └─────────────┘
```

#### Step 1: Validate
- Check `layer/` exists (fail fast if not)
- Check `.patina/oxidize.yaml` exists (fail fast if not)
- Check `.git/` exists (warn if not, skip git scrape)

#### Step 2: Scrape
- Initialize `.patina/data/patina.db` if missing
- Run scrapers in order:
  1. `git` → commits, co_changes views
  2. `sessions` → session events from `layer/sessions/*.md`
  3. `code` → functions, call_graph from source files

#### Step 3: Oxidize
- Load `oxidize.yaml` recipe
- For each projection in recipe:
  1. Generate training pairs from patina.db
  2. Train MLP projection
  3. Export `.safetensors` weights
  4. Build `.usearch` index

### Error Handling

| Error | Behavior |
|-------|----------|
| No `layer/` | Exit with error: "Not a Patina project (no layer/ found)" |
| No `oxidize.yaml` | Exit with error: "No recipe found. Run: patina init" |
| No `.git/` | Warn, skip git scrape, continue |
| Scrape fails | Exit with error, preserve partial state |
| Oxidize fails | Exit with error, preserve db (can retry with --oxidize) |
| Model download fails | Exit with error, suggest checking network |

### Progress Output

```
$ patina rebuild

🔄 Rebuilding .patina/ from layer/

📋 Validation
   ✓ layer/ found (127 sessions)
   ✓ oxidize.yaml found (3 projections)
   ✓ .git/ found (707 commits)

📥 Scrape (Step 1/2)
   • git: 707 commits, 17,685 co-changes
   • sessions: 2,174 events
   • code: 13,146 events (790 functions)
   ✓ patina.db: 15,920 events

🧪 Oxidize (Step 2/2)
   • semantic: 100 pairs → training...
   • temporal: 100 pairs → training...
   • dependency: 100 pairs → training...
   ✓ 3 projections built

✅ Rebuild complete!
   Database: .patina/data/patina.db (2.1 MB)
   Indices: .patina/data/embeddings/e5-base-v2/projections/ (8.4 MB)
```

---

## File Structure

```
src/commands/rebuild/
├── mod.rs              # Public interface, CLI args
└── internal.rs         # Pipeline implementation
```

### mod.rs

```rust
use clap::Args;

#[derive(Args)]
pub struct RebuildArgs {
    /// Only run scrape step (skip oxidize)
    #[arg(long)]
    pub scrape: bool,

    /// Only run oxidize step (assume db exists)
    #[arg(long)]
    pub oxidize: bool,

    /// Delete existing data before rebuild
    #[arg(long)]
    pub force: bool,

    /// Show what would be rebuilt without doing it
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(args: RebuildArgs) -> anyhow::Result<()> {
    internal::rebuild(args)
}
```

---

## Validation Criteria

**rebuild is complete when:**
1. [x] `git clone <repo-with-layer>` + `patina rebuild` produces working `.patina/`
2. [x] `patina scry "query"` works after rebuild
3. [x] `--force` clears existing data before rebuild
4. [x] `--dry-run` shows plan without executing
5. [x] Appropriate errors for missing layer/, oxidize.yaml

---

## Dependencies

- Existing: `scrape` module (git, sessions, code)
- Existing: `oxidize` module (training, safetensors, usearch)
- New: Validation logic, pipeline orchestration

---

## Future Considerations

- `patina rebuild --parallel` - parallelize scrape steps
- Incremental rebuild (only changed files) - deferred, full rebuild is fine for now
