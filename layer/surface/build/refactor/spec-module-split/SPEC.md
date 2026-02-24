---
type: refactor
id: spec-module-split
status: draft
created: 2026-02-24
sessions:
  origin: 20260224-053924
related:
  - src/commands/spec/mod.rs
  - src/commands/spec/internal.rs
  - src/release/mod.rs
beliefs:
  - dependable-rust
  - unix-philosophy
  - plugins-are-three-prong-bundles
blocks:
  - spec-create
---

# refactor: Spec Module Split — internal.rs → internal/

> `src/commands/spec/internal.rs` is 2062 lines — the largest internal
> file in the codebase. It has clear section boundaries but everything
> lives in one file. Split into `internal/` directory following the
> established pattern (assay: 10 files, scry: 7, eval: 5). Also
> extract a `types.rs` with `SpecType` registry so type knowledge
> (bump, directory, body template) has a single source of truth —
> preparing for both `spec create` and future WASM plugin extraction.

## Problem

`internal.rs` has 7 logical sections crammed into one file:

```
lines  15-211   Ready Queue queries
lines 212-371   Blocked View queries
lines 372-605   Spec List + scan_disk_specs
lines 606-865   Archive (archive_spec, archive_stale, find_spec)
lines 866-1591  Mutation infrastructure (promote, complete, abandon,
                pause, resume, block + shared helpers)
lines 1592-1792 Split
lines 1793-2062 Queue system (next_spec, age, dep_counts, util)
```

Spec types are hardcoded in two places that don't talk to each other:
- `BumpType::from_spec_type()` in `release/mod.rs` (bump mapping)
- Directory convention is implicit (code constructs paths ad hoc)

When spec becomes a WASM plugin, types need to be plugin-internal
data — not scattered across modules.

## Solution

### Split internal.rs into internal/ directory

```
src/commands/spec/
├── mod.rs                    — public API (unchanged shape)
└── internal/
    ├── mod.rs                — re-exports only
    ├── types.rs              — SpecType, SPEC_TYPES, body templates
    ├── queries.rs            — get_ready/blocked/all_specs, show_*, scan_disk
    ├── mutations.rs          — promote, complete, abandon, pause, resume,
    │                           block + _value() variants + shared helpers
    ├── split.rs              — split_spec, split_spec_value
    ├── queue.rs              — next_spec, spec_age_days, load_dep_counts
    ├── archive.rs            — archive_spec, archive_stale, archive_inner
    └── util.rs               — find_spec, tag_exists, is_tree_clean,
                                next_tag_number, resolve_spec_dir
```

### Extract SpecType registry into types.rs

```rust
pub struct SpecType {
    pub name: &'static str,
    pub bump: Option<BumpType>,
    pub directory: &'static str,
    pub body_template: &'static str,
}

pub const SPEC_TYPES: &[SpecType] = &[
    SpecType { name: "feat",     bump: Some(BumpType::Minor), directory: "feat",     body_template: FEAT_TEMPLATE },
    SpecType { name: "fix",      bump: Some(BumpType::Patch), directory: "fix",      body_template: FIX_TEMPLATE },
    SpecType { name: "refactor", bump: Some(BumpType::Patch), directory: "refactor", body_template: REFACTOR_TEMPLATE },
    SpecType { name: "explore",  bump: None,                  directory: "explore",  body_template: EXPLORE_TEMPLATE },
];

pub fn lookup(name: &str) -> Option<&'static SpecType> { ... }
```

Body templates per type (from survey of 117 archived specs):
- **feat**: Problem, Solution, Design Decisions, Implementation, Exit Criteria, Key Files, Non-Goals, Provenance
- **fix**: Problem, Solution, Exit Criteria, Key Files, Provenance
- **refactor**: Problem, Solution, Migration, Exit Criteria, Key Files, Provenance
- **explore**: Exit Criteria (checklist only — lightest)

### Wire BumpType to the registry

`BumpType::from_spec_type()` delegates to `spec::types::lookup()`
instead of hardcoding a match arm. Single source of truth.

## Migration

Pure restructuring — no behavior changes, no API changes. `mod.rs`
public interface stays identical. Tests move with their functions.

## Exit Criteria

- [ ] `internal/` directory with 8 files
- [ ] `mod.rs` public API identical (no signature changes)
- [ ] `SpecType` registry with 4 types
- [ ] `BumpType::from_spec_type()` delegates to registry
- [ ] Body templates for each type
- [ ] All existing tests pass
- [ ] `cargo clippy` clean

## Key Files

```
src/commands/spec/internal.rs         — delete (replaced by internal/)
src/commands/spec/internal/mod.rs     — re-exports
src/commands/spec/internal/types.rs   — SpecType, SPEC_TYPES, templates
src/commands/spec/internal/queries.rs
src/commands/spec/internal/mutations.rs
src/commands/spec/internal/split.rs
src/commands/spec/internal/queue.rs
src/commands/spec/internal/archive.rs
src/commands/spec/internal/util.rs
src/release/mod.rs                    — from_spec_type() refactored
```

## Provenance

Motivated by spec-create needing to add ~200 lines of create logic
plus type registry data to an already-2062-line file. Follows the
pattern established by assay, scry, and eval modules. Blocks
spec-create — do the split first so create.rs lands in a clean
directory structure.
