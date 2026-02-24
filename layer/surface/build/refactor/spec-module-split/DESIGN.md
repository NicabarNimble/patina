# Design Walkthrough: spec-module-split

Step-by-step implementation guide. Read SPEC.md first for the full
rationale — this doc is the mechanical "how."

## Before You Start

```bash
# Verify current state
wc -l src/commands/spec/internal.rs   # should be ~2063
cargo test -p patina                  # all green
cargo clippy                          # clean
```

Read these files in order:
1. `src/commands/spec/mod.rs` — the public API you must preserve
2. `src/commands/spec/internal.rs` — the file being split
3. `src/commands/assay/internal/mod.rs` — the re-export pattern to follow
4. `src/release/mod.rs` — `BumpType::from_spec_type()` to rewire

## Step 1: Create the Directory

```bash
mkdir src/commands/spec/internal
```

Don't delete `internal.rs` yet — Rust can't have both `internal.rs`
and `internal/` at the same time. You'll rename at the end.

**Strategy:** Build the new `internal/mod.rs` and submodules alongside
the old `internal.rs`, then swap in one move.

## Step 2: Create internal/mod.rs

This is the wiring hub. Start with a skeleton that re-exports
everything `mod.rs` currently imports from `internal`.

```rust
//! Internal implementation for spec command
//!
//! Follows dependable-rust pattern: private modules with curated re-exports.

mod archive;
mod mutations;
mod queries;
mod queue;
mod split;
pub(crate) mod types;

pub(crate) const DB_PATH: &str = ".patina/local/data/patina.db";

// --- Re-exports for parent mod.rs ---

// Query types + functions (used by session integration + MCP)
pub(super) use queries::{
    get_all_specs, get_blocked_specs, get_ready_specs,
    show_blocked_specs, show_ready_specs, show_spec_list,
    ListFilters, ReadySpec, BlockedSpec, SpecInfo,
};

// Mutation functions (CLI + MCP)
pub(super) use mutations::{
    promote_spec, complete_spec, abandon_spec,
    pause_spec, resume_spec, block_spec,
    promote_spec_value, complete_spec_value, abandon_spec_value,
    pause_spec_value, resume_spec_value, block_spec_value,
};

// Archive functions (CLI)
pub(super) use archive::{archive_spec, archive_stale_specs};

// Split functions (CLI + MCP)
pub(super) use split::{split_spec, split_spec_value};

// Queue functions (CLI + MCP + session integration)
pub(super) use queue::{
    next_spec, next_spec_value,
    spec_age_days_from_list, load_dep_counts,
};
```

**Key visibility rules:**
- `pub(super)` — visible to parent `mod.rs` (which re-exports selectively)
- `pub(crate)` on `types` module — needed by `release/mod.rs`
- Everything else is private to `internal/`

## Step 3: Extract types.rs (New File)

This file is entirely new — no code moves from `internal.rs`.

```rust
//! Spec type registry — single source of truth for type knowledge.
//!
//! Each spec type has a name, bump behavior, directory convention,
//! and body template. When spec becomes a WASM plugin, this data
//! moves into the plugin crate.

use patina::release::BumpType;

/// A registered spec type with all its conventions.
pub struct SpecType {
    /// Type name used in frontmatter and CLI: "feat", "fix", etc.
    pub name: &'static str,
    /// Version bump behavior. None = no release (explore).
    pub bump: Option<BumpType>,
    /// Directory under layer/surface/build/
    pub directory: &'static str,
    /// Markdown body template (section headings only).
    pub body_template: &'static str,
}

/// All registered spec types.
pub const SPEC_TYPES: &[SpecType] = &[
    SpecType {
        name: "feat",
        bump: Some(BumpType::Minor),
        directory: "feat",
        body_template: FEAT_TEMPLATE,
    },
    SpecType {
        name: "fix",
        bump: Some(BumpType::Patch),
        directory: "fix",
        body_template: FIX_TEMPLATE,
    },
    SpecType {
        name: "refactor",
        bump: Some(BumpType::Patch),
        directory: "refactor",
        body_template: REFACTOR_TEMPLATE,
    },
    SpecType {
        name: "explore",
        bump: None,
        directory: "explore",
        body_template: EXPLORE_TEMPLATE,
    },
];

/// Look up a spec type by name.
pub fn lookup(name: &str) -> Option<&'static SpecType> {
    SPEC_TYPES.iter().find(|t| t.name == name)
}

// Body templates — section headings only, LLM/user fills in content.
// Derived from survey of 117 archived specs.

const FEAT_TEMPLATE: &str = "\
## Problem

## Solution

## Design Decisions

## Implementation

## Exit Criteria

- [ ]

## Key Files

```
```

## Non-Goals

## Provenance
";

const FIX_TEMPLATE: &str = "\
## Problem

## Solution

## Exit Criteria

- [ ]

## Key Files

```
```

## Provenance
";

const REFACTOR_TEMPLATE: &str = "\
## Problem

## Solution

## Migration

## Exit Criteria

- [ ]

## Key Files

```
```

## Provenance
";

const EXPLORE_TEMPLATE: &str = "\
## Exit Criteria

- [ ]
";
```

## Step 4: Extract queries.rs

Move these sections from `internal.rs`:
- Lines 14-210: Ready Queue (structs + functions)
- Lines 211-370: Blocked View (structs + functions)
- Lines 371-604: Spec List + `scan_disk_specs`

**Imports this file needs:**

```rust
use anyhow::{Context, Result};
use regex::Regex;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

use patina::spec::parse_spec_file;

use super::DB_PATH;
use super::queue::{spec_age_days_from_list, load_dep_counts};
```

**Watch out:**
- `show_ready_specs()` calls `spec_age_days_from_list()` and
  `load_dep_counts()` — both live in `queue.rs`. Use `super::queue::`.
- `scan_disk_specs()` is `fn` (private) — only called within this
  file by `get_all_specs()` and exported to `archive.rs` via
  `pub(super)` if needed. Actually, `find_spec()` in archive also
  calls it — so make it `pub(super)`.

## Step 5: Extract archive.rs

Move from internal.rs:
- Lines 605-864: Archive section

**Functions:**
- `archive_spec()` — public entry point
- `archive_spec_inner()` — core logic (tag + rm + commit)
- `archive_stale_specs()` — batch cleanup
- `resolve_spec_dir()` — helper
- `find_spec()` — spec lookup (DB then filesystem)
- `find_spec_file_on_disk()` — recursive filesystem search

**Imports:**

```rust
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::process::Command;

use patina::spec::parse_spec_file;

use super::DB_PATH;
use super::queries::{get_all_specs, scan_disk_specs, ListFilters, SpecInfo};
```

**Cross-module note:** `find_spec()` is called by mutations.rs,
split.rs, and queue.rs. Make it `pub(super)`.

## Step 6: Extract mutations.rs

Move from internal.rs:
- Lines 865-1590: Shared mutation infrastructure + all mutation commands

**Functions (in order):**
- `mutate_spec()` — core YAML + DB updater
- `with_yaml_rollback()` — failure rollback wrapper
- `next_tag_number()` — tag sequence derivation
- `promote_spec()` / `promote_spec_value()`
- `complete_spec()` / `complete_spec_value()`
- `abandon_spec()` / `abandon_spec_value()`
- `pause_spec()` / `pause_spec_value()`
- `resume_spec()` / `resume_spec_value()`
- `block_spec()` / `block_spec_value()`

**Imports:**

```rust
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::process::Command;

use patina::release::{BumpType, ReleaseStrategy};
use patina::spec::{parse_spec_file, serialize_spec_file, SpecFrontmatter};

use super::DB_PATH;
use super::archive::{archive_spec_inner, find_spec, resolve_spec_dir};
use super::queries::{get_all_specs, get_blocked_specs, ListFilters};
use super::queue::tag_exists;
```

**This is the largest file (~726 lines).** That's acceptable — the
mutations are logically cohesive (all use `mutate_spec`, share the
same git commit pattern, operate on the same state machine).

## Step 7: Extract split.rs

Move from internal.rs:
- Lines 1591-1791: Split

**Functions:**
- `split_spec()` — CLI entry
- `split_spec_value()` — MCP entry

**Imports:**

```rust
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::process::Command;

use patina::release::{BumpType, ReleaseStrategy};
use patina::spec::{parse_spec_file, serialize_spec_file};

use super::DB_PATH;
use super::archive::{archive_spec_inner, find_spec, resolve_spec_dir};
use super::queue::tag_exists;
```

## Step 8: Extract queue.rs

Move from internal.rs:
- Lines 1792-2063: Queue system + tests

**Functions:**
- `next_spec()` / `next_spec_value()`
- `spec_age_days_from_list()` — pub(crate), used by session
- `load_dep_counts()` — pub(crate), used by session
- `tag_exists()` — delegation to `patina::git`
- `is_tree_clean()` — delegation to `patina::git`

**Tests (move with this file):**
- `test_tag_name_format`
- `test_resolve_spec_dir_with_directory` → actually move to archive.rs
- `test_resolve_spec_dir_root_file` → actually move to archive.rs
- `test_archive_requires_complete_or_abandoned` → move to archive.rs

**Correction:** Only `test_tag_name_format` stays in queue.rs. The
three archive-related tests move to archive.rs.

**Imports:**

```rust
use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

use patina::spec::parse_spec_file;

use super::DB_PATH;
use super::archive::find_spec;
use super::queries::{get_all_specs, get_blocked_specs, ListFilters, SpecInfo};
```

## Step 9: Wire BumpType Delegation

In `src/release/mod.rs`, change `from_spec_type()`:

```rust
// Before:
pub fn from_spec_type(spec_type: &str) -> Option<Self> {
    match spec_type {
        "fix" | "refactor" => Some(BumpType::Patch),
        "feat" => Some(BumpType::Minor),
        _ => None,
    }
}

// After:
pub fn from_spec_type(spec_type: &str) -> Option<Self> {
    crate::commands::spec::types::lookup(spec_type)
        .and_then(|t| t.bump)
}
```

This requires the parent `mod.rs` to re-export types:

```rust
// In src/commands/spec/mod.rs, add:
pub(crate) use internal::types;
```

## Step 10: The Swap

Now replace the old file with the new directory:

```bash
# Rename old file out of the way
mv src/commands/spec/internal.rs src/commands/spec/internal.rs.bak

# The internal/ directory is already in place
# Rust now resolves `mod internal;` to `internal/mod.rs`

# Verify
cargo test
cargo clippy
cargo build --release

# If green, delete backup
rm src/commands/spec/internal.rs.bak
```

**If it fails:** `mv src/commands/spec/internal.rs.bak src/commands/spec/internal.rs`
and `rm -rf src/commands/spec/internal/` to revert.

## Commit Strategy

One commit per logical step, or batch into 2-3:

```
1. spec: add internal/ directory with types.rs
   (types.rs is new code, can coexist with internal.rs)

2. spec: split internal.rs into internal/ directory
   (the big move — queries, mutations, archive, split, queue, mod.rs)

3. spec: wire BumpType to SpecType registry
   (release/mod.rs delegation)
```

## Verification Checklist

After each step, verify nothing broke:

```bash
cargo test 2>&1 | tail -5          # all tests pass
cargo clippy 2>&1 | tail -5        # no warnings
cargo build --release               # release builds
cargo install --path . && patina spec list   # live test
```

Final check — diff the public API:

```bash
# Before (from git): the mod.rs pub interface
git show HEAD:src/commands/spec/mod.rs | grep 'pub '

# After: should be identical
grep 'pub ' src/commands/spec/mod.rs
```

## Circular Dependency Risk

The cross-module calls create potential circular imports:
- `queries.rs` → `queue.rs` (for age/deps)
- `queue.rs` → `queries.rs` (for get_all_specs)

Rust handles this fine within a single crate — sibling modules under
the same parent can import each other via `super::`. No circular
dependency issues because Rust resolves at the item level, not the
module level. The assay module already does this (search → util,
derive → util, etc.).
