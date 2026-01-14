# Spec: .patina/local/ Directory Structure

**Status:** Implemented
**Created:** 2026-01-12
**Implemented:** 2026-01-12
**Core References:** [dependable-rust](../../core/dependable-rust.md), [unix-philosophy](../../core/unix-philosophy.md)

---

## Core Values Anchor

This spec applies two core principles:

**From [dependable-rust](../../core/dependable-rust.md):**
> "Keep your public interface small and stable. Hide implementation details... This creates black-box modules that can be completely rewritten internally without breaking users."

Applied here: `.patina/` exposes a stable interface (config, uid) while hiding changeable details (databases, cache) in `local/`. The `local/` directory can be deleted and regenerated without affecting the project identity.

**From [unix-philosophy](../../core/unix-philosophy.md):**
> "Patina follows Unix philosophy: one tool, one job, done well."

Applied here: Clear separation of concerns. Committed files do one job (project identity/config). Local files do another job (derived/cached state). No mixing.

---

## Problem

Current `.patina/` design mixes committed and ignored files:

```
.patina/
├── config.toml    # should be committed (shared)
├── uid            # should be committed (shared)
├── backups/       # should be ignored (local)
├── data/          # should be ignored (local)
│   ├── patina.db
│   └── embeddings/
└── ...
```

This causes:
1. **Gitignore pollution**: Need complex rules (`.patina/*.db`, `.patina/backups/`, etc.)
2. **Unclear boundaries**: Not obvious what's shared vs local
3. **Init commit failures**: Trying to commit `.patina/` when parts are gitignored

## Solution

Clear separation with `local/` subdirectory:

```
.patina/
├── config.toml       # COMMITTED - project settings
├── uid               # COMMITTED - project identity (8 hex chars)
├── oxidize.yaml      # COMMITTED - scrape/index recipe
├── versions.json     # COMMITTED - component versions
└── local/            # GITIGNORED - all local/derived state
    ├── data/
    │   ├── patina.db
    │   └── embeddings/
    ├── backups/
    ├── run/          # PID files, locks
    └── logs/         # local logs
```

## Design Principles

### 1. Clear Naming Signals Intent

The name `local/` explicitly communicates "this is local-only state." Anyone looking at the structure understands immediately what gets committed.

### 2. One Gitignore Entry

```gitignore
.patina/local/
```

Not scattered rules. Not the entire `.patina/`. Just the local subdirectory.

### 3. Contained Parasite

Patina lives on the `patina` branch as a contained addition:
- All patina files in one place (`.patina/`, `.claude/`, `layer/`)
- Easy to exclude from upstream PRs
- Never pollutes main branch

The `local/` design maintains containment while enabling clean git operations.

### 4. Shared Config, Local Cache

| Path | Committed | Purpose |
|------|-----------|---------|
| `.patina/config.toml` | Yes | Project settings (embeddings model, retrieval params) |
| `.patina/uid` | Yes | Project identity for federation |
| `.patina/oxidize.yaml` | Yes | Scrape/index recipe |
| `.patina/versions.json` | Yes | Component version tracking |
| `.patina/local/` | No | All derived/regeneratable state |

### 5. Regeneratable Local State

Everything in `local/` can be rebuilt:
- `data/patina.db` - regenerate with `patina scrape`
- `data/embeddings/` - regenerate with `patina oxidize`
- `backups/` - local safety copies, not needed across clones
- `run/` - transient process state
- `logs/` - local debugging

If you delete `.patina/local/`, run `patina scrape && patina oxidize` to rebuild.

## Migration

### Files to Move

| Current Location | New Location |
|------------------|--------------|
| `.patina/data/` | `.patina/local/data/` |
| `.patina/backups/` | `.patina/local/backups/` |
| `.patina/run/` | `.patina/local/run/` |
| `.patina/logs/` | `.patina/local/logs/` |

### Files That Stay

| Location | Status |
|----------|--------|
| `.patina/config.toml` | Stays (committed) |
| `.patina/uid` | Stays (committed) |
| `.patina/oxidize.yaml` | Stays (committed) |
| `.patina/versions.json` | Stays (committed) |

## Implementation

### Phase 1: Update Path Functions

In `src/paths.rs` and `src/project/internal.rs`:

```rust
// Before
pub fn data_dir(root: &Path) -> PathBuf {
    root.join(".patina/data")
}

pub fn backups_dir(root: &Path) -> PathBuf {
    root.join(".patina/backups")
}

// After
pub fn local_dir(root: &Path) -> PathBuf {
    root.join(".patina/local")
}

pub fn data_dir(root: &Path) -> PathBuf {
    root.join(".patina/local/data")
}

pub fn backups_dir(root: &Path) -> PathBuf {
    root.join(".patina/local/backups")
}
```

### Phase 2: Update Gitignore Handling

In `src/commands/init/internal/mod.rs`:

```rust
// Before (in must_have array)
(".patina/", "Patina cache"),

// After
(".patina/local/", "Patina local data (derived, not committed)"),
```

### Phase 3: Update Init Commit Logic

In `src/commands/init/internal/mod.rs`:

```rust
// Before - tried to add .patina (failed due to gitignore)
patina::git::add_paths(&[
    ".gitignore",
    ".patina",           // ← REMOVE
    ".devcontainer",
    // ...
])?;

// After - add specific files, not directory
patina::git::add_paths(&[
    ".gitignore",
    ".patina/config.toml",
    ".patina/uid",
    ".patina/oxidize.yaml",
    ".devcontainer",
    // ...
])?;
```

### Phase 4: Update All Hardcoded Paths

Search and replace across codebase:
- `.patina/data/` → `.patina/local/data/`
- `.patina/backups/` → `.patina/local/backups/`

Files with hardcoded paths (from grep):
- `src/mcp/server.rs`
- `src/retrieval/oracles/*.rs`
- `src/commands/scrape/*.rs`
- `src/commands/oxidize/*.rs`
- `src/commands/scry/internal/*.rs`
- `src/commands/rebuild/mod.rs`
- `src/commands/repo/internal.rs`

### Phase 5: Create local/ on First Use

In relevant commands (scrape, oxidize, etc.):

```rust
fn ensure_local_dir(project_path: &Path) -> Result<PathBuf> {
    let local = project_path.join(".patina/local");
    fs::create_dir_all(&local)?;
    Ok(local)
}
```

## Files to Modify

| File | Change |
|------|--------|
| `src/paths.rs` | Add `local_dir()`, update `data_dir()`, `backups_dir()` |
| `src/project/internal.rs` | Update `backups_dir()` |
| `src/commands/init/internal/mod.rs` | Update gitignore, commit logic |
| `src/mcp/server.rs` | Update DB_PATH constants |
| `src/retrieval/oracles/*.rs` | Update hardcoded paths |
| `src/commands/scrape/*.rs` | Update database paths |
| `src/commands/oxidize/*.rs` | Update output paths |
| `src/commands/rebuild/mod.rs` | Update data dir references |
| `src/commands/repo/internal.rs` | Update gitignore addition |

## Success Criteria

1. `patina init .` commits `.patina/config.toml`, `.patina/uid` without error
2. `patina scrape` creates database in `.patina/local/data/`
3. `git status` shows `.patina/local/` as ignored
4. Cloning repo + `patina scrape && patina oxidize` rebuilds local state
5. No gitignore entries except `.patina/local/`

## Non-Goals

- Moving cache to `~/.cache/patina/` (breaks Claude Code sandbox)
- Changing `~/.patina/` global structure (separate concern)
- Config schema changes (separate spec)

## References

- [dependable-rust](../../core/dependable-rust.md) - Black-box pattern: stable interface, hidden internals
- [unix-philosophy](../../core/unix-philosophy.md) - Single responsibility, clear separation
- [spec-init-hardening.md](spec-init-hardening.md) - Init refactoring context
- XDG Base Directory Specification - Inspiration for committed vs local separation
