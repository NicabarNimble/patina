# Spec: Folder Structure Design

**Status:** Design Complete, Implementation Pending
**Phase:** 2.9 (Folder Restructure)
**Session:** 20251215-155622
**Created:** 2025-12-15

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

### The Problem with Scattered Paths

Current code defines paths inline in each module:

```rust
// persona/mod.rs
fn persona_dir() -> PathBuf {
    dirs::home_dir().join(".patina/personas/default")
}
// Then: persona_dir().join("events"), persona_dir().join("materialized")
```

**Issues:**
- No single place to see filesystem layout
- Path changes require hunting through multiple files
- `persona_dir()` abstraction breaks when source/derived split

### Solution: `src/paths.rs`

Single module owns ALL filesystem layout decisions:

```rust
// src/paths.rs - Single source of truth

use std::path::PathBuf;

/// ~/.patina/
pub fn patina_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".patina")
}

/// ~/.patina/cache/ - all rebuildable data
pub fn patina_cache() -> PathBuf {
    patina_home().join("cache")
}

/// Persona paths
pub mod persona {
    use super::*;

    /// Source: ~/.patina/personas/default/events/
    pub fn events_dir() -> PathBuf {
        patina_home().join("personas/default/events")
    }

    /// Cache: ~/.patina/cache/personas/default/
    pub fn cache_dir() -> PathBuf {
        patina_cache().join("personas/default")
    }
}

/// Reference repo paths
pub mod repos {
    use super::*;

    /// Cache: ~/.patina/cache/repos/
    pub fn cache_dir() -> PathBuf {
        patina_cache().join("repos")
    }
}
```

### Consumer Code

```rust
use crate::paths::persona;

pub fn note(...) {
    let dir = persona::events_dir();  // Obviously source
    fs::create_dir_all(&dir)?;
    // ...
}

pub fn materialize() {
    let source = persona::events_dir();   // Read from here
    let target = persona::cache_dir();    // Write to here
    fs::create_dir_all(&target)?;
    // Intent is clear, no ambiguity
}

pub fn query(...) {
    let cache = persona::cache_dir();
    let db_path = cache.join("persona.db");
    let index_path = cache.join("persona.usearch");
    // ...
}
```

### Alignment with Core Values

**dependable-rust.md:**
- "Do X" test: "Return filesystem paths for Patina data"
- Small interface: just path functions, no I/O
- One file shows entire layout

**unix-philosophy.md:**
- One tool, one job: defines where data lives
- Composition: modules compose with paths
- No feature creep: no I/O, validation, or migration logic

### Affected Functions

| Function | Current | New |
|----------|---------|-----|
| `note()` | `persona_dir().join("events")` | `paths::persona::events_dir()` |
| `materialize()` | `persona_dir().join("materialized")` | `paths::persona::cache_dir()` |
| `query()` | `persona_dir().join("materialized/...")` | `paths::persona::cache_dir().join(...)` |
| `list()` | `persona_dir().join("events")` | `paths::persona::events_dir()` |

---

## User Decisions Needed

| Decision | Options | Impact |
|----------|---------|--------|
| `claude-linux/` folder | Keep / Delete / Archive | Cleanup, ~unknown size |
| Migration timing | Now / Later / Never | Code complexity |
| Backward compatibility | Support old paths temporarily? | Migration friction |

---

## Validation Checklist

| Criteria | Status |
|----------|--------|
| `layer/` stays at project root | ✅ Design decision |
| Project `.patina/data/` is gitignored | ✅ Already done |
| User `cache/` directory created | ⬜ Implementation |
| `materialized/` moved to `cache/personas/` | ⬜ Implementation |
| `repos/` moved to `cache/repos/` | ⬜ Implementation |
| `patina persona materialize` uses new path | ⬜ Code change |
| `patina persona query` uses new path | ⬜ Code change |
| Old `materialized/` cleaned up | ⬜ Migration |
| `claude-linux/` evaluated | ⬜ User decision |

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

- **Session:** 20251215-155622 (design discussion)
- **Session:** 20251215-111922 (initial proposal in last session)
- **Cargo docs:** https://doc.rust-lang.org/cargo/guide/cargo-home.html
- **XDG spec:** https://specifications.freedesktop.org/basedir-spec/
