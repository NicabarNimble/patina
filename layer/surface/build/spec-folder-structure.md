# Spec: Folder Structure Design

**Status:** Design Complete, Implementation Pending
**Phase:** 1 (Folder Restructure)
**Session:** 20251215-192032
**Created:** 2025-12-15
**Updated:** 2025-12-15

---

## Executive Summary

Define the canonical folder structure for Patina at both project and user levels. The key insight: **patina coats projects, not users**.

- `layer/` = the accumulated patina on a project (AI-authored knowledge)
- `~/.patina/` = the user's tools and preferences (observer, not observed)

---

## Design Rationale

### Reference Points (Best-in-Class CLI Tools)

| Tool | Pattern | Lesson |
|------|---------|--------|
| **Git** | Everything in `.git/`, self-contained | Hidden infrastructure, opaque to users |
| **Cargo** | `Cargo.toml` visible, `target/` hidden | Source visible, derived hidden |
| **Ollama** | `~/.ollama/` - radical simplicity | One folder, everything inside |
| **mise/asdf** | XDG compliant (config/data/cache split) | Standard, but splits across 3 locations |

### The Key Question: What IS `layer/` Content?

| Content Type | Examples | Visibility |
|--------------|----------|------------|
| Git objects | Blob hashes | Hidden, opaque |
| Config files | .gitignore | Visible, human-edited |
| Obsidian notes | Markdown vault | Visible, browsable |
| **layer/** | Sessions, patterns | AI-authored, human-reviewable |

**The tension:** `layer/` is AI-authored but human-reviewable. It's not machine-only (like git objects) but it's not human-authored (like code).

### Decision: Obsidian Model (Knowledge Visible)

`layer/` stays at project root because:

1. **It's the product** - Accumulated knowledge is the value proposition
2. **Users should see it** - Like README.md or docs/, it's part of the project
3. **Follows Cargo pattern** - `Cargo.toml` visible because it matters; `target/` hidden because derived
4. **Migration cost for hiding is real** - Moving inside `.patina/` updates every path, for what benefit?

---

## Project Level Structure

### Current State (Already Correct)

```
project/
├── layer/                  # Visible, git-tracked - THE VALUE
│   ├── core/               # Eternal patterns (7 files)
│   ├── surface/            # Active knowledge (13 files + build/)
│   ├── sessions/           # Session logs (~300 files)
│   └── dust/               # Archived
└── .patina/
    ├── config.toml         # Project config
    ├── oxidize.yaml        # Embedding config
    ├── versions.json       # Version tracking
    ├── backups/            # Backup files
    └── data/               # Derived (gitignored)
        ├── patina.db       # 43MB SQLite
        └── embeddings/     # Vector indices
```

### Changes Needed

| Item | Action | Reason |
|------|--------|--------|
| `.patina/patina.db` (0 bytes) | Delete | Stale/empty file at wrong location |

**No structural changes required at project level.**

---

## User Level Structure

### Current State (Needs Restructure)

```
~/.patina/
├── adapters/                       # LLM adapter configs
├── claude-linux/                   # Claude-specific (legacy?)
├── repos/                          # Git clones (rebuildable)
├── personas/
│   └── default/
│       ├── events/                 # Source ✓
│       │   └── 20251208.jsonl      # Only 6 events!
│       └── materialized/           # Derived (MIXED with source)
│           ├── persona.db
│           └── persona.usearch
└── registry.yaml                   # Project/repo registry
```

### Problems

1. **Mixed source/derived:** `events/` (source) and `materialized/` (derived) are siblings
2. **Cache at root:** `repos/` is rebuildable but not clearly marked
3. **Unclear backup story:** What do I back up? What can I delete?
4. **Legacy cruft:** `claude-linux/` - still needed?

### Proposed Structure

```
~/.patina/
├── adapters/                       # LLM adapter configs (keep)
├── personas/
│   └── default/
│       └── events/*.jsonl          # Source ONLY
├── cache/                          # NEW - all rebuildable
│   ├── repos/                      # Moved from root
│   └── personas/
│       └── default/
│           ├── persona.db          # Moved from materialized/
│           └── persona.usearch
├── registry.yaml                   # Config (keep at root)
└── (evaluate claude-linux/)        # USER DECISION NEEDED
```

### What This Achieves

| Goal | How |
|------|-----|
| Clear backup story | `~/.patina/` minus `cache/` = everything valuable |
| Clear rebuild story | Delete `cache/`, run `patina persona materialize` |
| Source/derived separation | `personas/*/events/` (source) vs `cache/personas/*/` (derived) |

---

## Migration Path

### Phase 1: Create `cache/` Structure

```bash
mkdir -p ~/.patina/cache/repos
mkdir -p ~/.patina/cache/personas/default
```

### Phase 2: Update Code Paths

| File | Function | Change |
|------|----------|--------|
| `src/commands/persona/mod.rs` | `materialized_dir()` | `~/.patina/cache/personas/default/` |
| `src/commands/persona/mod.rs` | `persona_dir()` | Keep as `~/.patina/personas/default/` |
| (repo code if exists) | repo clone path | `~/.patina/cache/repos/` |

### Phase 3: Migrate Existing Data

```bash
# Move materialized data
mv ~/.patina/personas/default/materialized/* ~/.patina/cache/personas/default/
rmdir ~/.patina/personas/default/materialized

# Move repos
mv ~/.patina/repos/* ~/.patina/cache/repos/
rmdir ~/.patina/repos
```

### Phase 4: Cleanup (User Decision)

- Evaluate `~/.patina/claude-linux/` - delete if unused
- Clean up empty directories

---

## Code Design: Centralized Paths Module

### Design Philosophy

From **rationale-eskil-steenberg.md**:

> "It's faster to write 5 lines of code today than to write 1 line today and edit it later."
> "Make it complete - cover all use cases, but no more"

The paths module should be **complete from day one** - not minimal, not clever, but correct. If `paths.rs` is the single source of truth for Patina filesystem layout, it must define ALL paths (user-level AND project-level).

**This is not over-engineering.** It's designing the complete, correct API once so we never have to change it.

### The Problem with Scattered Paths

Current code defines paths inline across 45+ files:

```rust
// persona/mod.rs
fn persona_dir() -> PathBuf { dirs::home_dir().join(".patina/personas/default") }

// workspace/internal.rs
pub fn mothership_dir() -> PathBuf { dirs::home_dir().join(".patina") }

// repo/internal.rs (DUPLICATE!)
pub fn mothership_dir() -> PathBuf { dirs::home_dir().join(".patina") }

// scrape/database.rs
pub const PATINA_DB: &str = ".patina/data/patina.db";

// retrieval/oracles/semantic.rs
db_path: PathBuf::from(".patina/data/patina.db"),
```

**Issues:**
- No single place to see filesystem layout
- Same paths defined in multiple places (DRY violation)
- Mix of absolute paths (user-level) and relative paths (project-level)
- Path changes require hunting through 45+ files
- Abstractions break when structure changes

### Solution: `src/paths.rs`

Single module owns ALL filesystem layout decisions for both user-level and project-level:

```rust
//! src/paths.rs - Single source of truth for ALL Patina filesystem layout
//!
//! This module defines WHERE data lives. It has no I/O, no validation,
//! no business logic. One file shows the entire filesystem layout.
//!
//! Design: Complete from day one (Eskil philosophy).
//! "Write code once, have it work forever."

use std::path::{Path, PathBuf};

// =============================================================================
// User Level (~/.patina/)
// =============================================================================

/// User's patina home directory: ~/.patina/
pub fn patina_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".patina")
}

/// Cache directory for all rebuildable data: ~/.patina/cache/
pub fn patina_cache() -> PathBuf {
    patina_home().join("cache")
}

/// Global config file: ~/.patina/config.toml
pub fn config_path() -> PathBuf {
    patina_home().join("config.toml")
}

/// Project/repo registry: ~/.patina/registry.yaml
pub fn registry_path() -> PathBuf {
    patina_home().join("registry.yaml")
}

/// LLM adapter templates: ~/.patina/adapters/
pub fn adapters_dir() -> PathBuf {
    patina_home().join("adapters")
}

/// Persona paths (cross-project user knowledge)
pub mod persona {
    use super::*;

    /// Source events (valuable): ~/.patina/personas/default/events/
    pub fn events_dir() -> PathBuf {
        patina_home().join("personas/default/events")
    }

    /// Materialized cache (rebuildable): ~/.patina/cache/personas/default/
    pub fn cache_dir() -> PathBuf {
        patina_cache().join("personas/default")
    }
}

/// Reference repository paths
pub mod repos {
    use super::*;

    /// Cloned repos (rebuildable): ~/.patina/cache/repos/
    pub fn cache_dir() -> PathBuf {
        patina_cache().join("repos")
    }
}

// =============================================================================
// Project Level (project/.patina/)
// =============================================================================

/// Project-level paths, relative to a project root
pub mod project {
    use super::*;

    /// Project's patina directory: .patina/
    pub fn patina_dir(root: &Path) -> PathBuf {
        root.join(".patina")
    }

    /// Project config: .patina/config.toml
    pub fn config_path(root: &Path) -> PathBuf {
        root.join(".patina/config.toml")
    }

    /// Derived data directory (gitignored): .patina/data/
    pub fn data_dir(root: &Path) -> PathBuf {
        root.join(".patina/data")
    }

    /// Main SQLite database: .patina/data/patina.db
    pub fn db_path(root: &Path) -> PathBuf {
        root.join(".patina/data/patina.db")
    }

    /// Embedding indices: .patina/data/embeddings/
    pub fn embeddings_dir(root: &Path) -> PathBuf {
        root.join(".patina/data/embeddings")
    }

    /// Model-specific projections: .patina/data/embeddings/{model}/projections/
    pub fn model_projections_dir(root: &Path, model: &str) -> PathBuf {
        root.join(format!(".patina/data/embeddings/{}/projections", model))
    }

    /// Oxidize recipe: .patina/oxidize.yaml
    pub fn recipe_path(root: &Path) -> PathBuf {
        root.join(".patina/oxidize.yaml")
    }

    /// Version manifest: .patina/versions.json
    pub fn versions_path(root: &Path) -> PathBuf {
        root.join(".patina/versions.json")
    }

    /// Backup directory: .patina/backups/
    pub fn backups_dir(root: &Path) -> PathBuf {
        root.join(".patina/backups")
    }
}
```

### Consumer Code Examples

**User-level (absolute paths):**
```rust
use patina::paths::{self, persona};

pub fn note(...) {
    let dir = persona::events_dir();  // Obviously source
    fs::create_dir_all(&dir)?;
}

pub fn materialize() {
    let source = persona::events_dir();   // Read from here
    let target = persona::cache_dir();    // Write to here - clear intent
}
```

**Project-level (relative to root):**
```rust
use patina::paths::project;

pub fn scrape(project_root: &Path) {
    let db = project::db_path(project_root);
    let embeddings = project::embeddings_dir(project_root);
}

pub fn oxidize(project_root: &Path, model: &str) {
    let recipe = project::recipe_path(project_root);
    let projections = project::model_projections_dir(project_root, model);
}
```

### Alignment with Core Values

**rationale-eskil-steenberg.md:**
- "Write code once, have it work forever"
- "Make it complete - cover all use cases, but no more"
- API designed once, correctly - never needs to change

**dependable-rust.md:**
- "Do X" test: "Return filesystem paths for ALL Patina data" - clear and complete
- Small interface: just path functions, no I/O
- One file shows ENTIRE layout (user + project)
- Black box: internals can change, API stays stable

**unix-philosophy.md:**
- One tool, one job: defines where data lives
- Composition: modules compose with paths
- No feature creep: no I/O, validation, or migration logic

### Current Path Definitions to Consolidate

| Location | Functions | Status |
|----------|-----------|--------|
| `workspace/internal.rs` | `mothership_dir()`, `adapters_dir()`, `config_path()` | Move to paths.rs |
| `repo/internal.rs` | `mothership_dir()` (duplicate!), `repos_dir()`, `registry_path()` | Move to paths.rs |
| `persona/mod.rs` | `persona_dir()` | Split to `persona::events_dir()`, `persona::cache_dir()` |
| `retrieval/oracles/persona.rs` | Hardcoded inline | Use `paths::persona::cache_dir()` |
| `adapters/templates.rs` | Uses `workspace::adapters_dir()` | Use `paths::adapters_dir()` |
| `scrape/database.rs` | `PATINA_DB` constant | Use `paths::project::db_path()` |
| `retrieval/oracles/*.rs` | Hardcoded `.patina/data/...` | Use `paths::project::*` |
| `oxidize/mod.rs` | Hardcoded paths | Use `paths::project::*` |
| `rebuild/mod.rs` | Hardcoded paths | Use `paths::project::*` |
| ~40 other files | Various hardcoded paths | Migrate incrementally |

---

## User Decisions Needed

| Decision | Options | Impact |
|----------|---------|--------|
| `claude-linux/` folder | Keep / Delete / Archive | Cleanup, ~unknown size |
| Migration timing | Now / Later / Never | Code complexity |
| Backward compatibility | Support old paths temporarily? | Migration friction |

---

## Validation Checklist

**Ship Criteria:**

| Criteria | Status |
|----------|--------|
| `layer/` stays at project root | ✅ Design decision |
| Project `.patina/data/` is gitignored | ✅ Already done |
| `paths.rs` exists with complete API (user + project) | ⬜ |
| User `cache/` directory structure works | ⬜ |
| `materialized/` moved to `cache/personas/` | ⬜ |
| `repos/` moved to `cache/repos/` | ⬜ |
| All user-level path functions consolidated | ⬜ |
| `patina persona` commands work | ⬜ |
| `patina repo` commands work | ⬜ |
| `patina` launcher works | ⬜ |
| Auto-migration on first run | ⬜ |
| `claude-linux/` evaluated | ⬜ |

**Deferred:** Project-level path consolidation (45+ files). See [spec-work-deferred.md](spec-work-deferred.md).

---

## Future Considerations

### User Events (Not Tracked)

Currently only 6 events exist (Dec 8, 2025). The infrastructure for capturing exists (`PersonaEvent`), but nothing auto-emits:

| Potential Trigger | Event Type | Status |
|-------------------|------------|--------|
| `patina persona note` | explicit_capture | ✅ Working |
| `/session-end` distillation | session_learning | ⬜ Not implemented |
| User correction observed | preference_observed | ⬜ Not implemented |

**Note:** This is Phase 3 work (Capture Automation), not Phase 2.9. The folder structure should support it, but implementation is separate.

### Cross-Platform

The `~/.patina/` convention works on Mac/Linux. Windows would need:
- `%APPDATA%\patina\` or
- `~\.patina\` (if ~ is expanded)

Not a priority until Windows support is needed.

---

## References

**Sessions:**
- **20251215-192032** - Current implementation session (Phase 1)
- **20251215-155622** - Design discussion, folder structure decisions
- **20251215-111922** - Initial proposal, Phase 2.8 architecture

**Core Patterns:**
- **rationale-eskil-steenberg.md** - "Complete from day one" philosophy
- **dependable-rust.md** - Black box module pattern
- **unix-philosophy.md** - One tool, one job

**External:**
- **Cargo docs:** https://doc.rust-lang.org/cargo/guide/cargo-home.html
- **XDG spec:** https://specifications.freedesktop.org/basedir-spec/
