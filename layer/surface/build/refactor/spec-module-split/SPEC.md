---
type: refactor
id: spec-module-split
status: active
created: 2026-02-24
blocks:
- spec-create
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
---

# refactor: Spec Module Split — internal.rs → internal/

> `src/commands/spec/internal.rs` is 2063 lines — the largest internal
> file in the codebase. It has 7 section-delimited regions but everything
> lives in one file. Split into `internal/` directory following the
> established pattern (assay: 10 files, scry: 7, eval: 5). Also extract
> a `types.rs` with `SpecType` registry so type knowledge (bump behavior,
> directory convention, body template) has a single source of truth —
> preparing for both `spec create` and future WASM plugin extraction.

## Problem

`internal.rs` has 7 logical sections separated by `// ====` banners:

```
lines   14-210   Ready Queue queries        (~197 lines)
lines  211-370   Blocked View queries        (~160 lines)
lines  371-604   Spec List + scan_disk_specs (~234 lines)
lines  605-864   Archive + find_spec         (~260 lines)
lines  865-1590  Mutations: promote, complete, abandon, pause,
                 resume, block + shared helpers (mutate_spec,
                 with_yaml_rollback, next_tag_number)  (~726 lines)
lines 1591-1791  Split                       (~201 lines)
lines 1792-2063  Queue: next_spec, age, dep_counts,
                 tag_exists, is_tree_clean + tests (~272 lines)
```

Spec types are hardcoded in two places that don't talk to each other:
- `BumpType::from_spec_type()` in `release/mod.rs` — 6-line match arm
  mapping feat→Minor, fix/refactor→Patch, _→None
- Directory convention is implicit — `split_spec_value()` constructs
  `layer/surface/build/<type>/<id>/` paths ad hoc

When spec becomes a WASM plugin, types need to be plugin-internal
data — not scattered across modules.

## Solution

### Split internal.rs into internal/ directory

Follow the established re-export pattern. Comparison:

```
assay/internal/mod.rs:   9 submodules (10 files), pub(super) + pub(crate) re-exports
scry/internal/mod.rs:    6 submodules (7 files), pub mod re-exports
eval/internal/mod.rs:    4 submodules (5 files), pub(crate) mod declarations
```

Proposed structure for spec:

```
src/commands/spec/
├── mod.rs                    — public API (unchanged shape, 315 lines)
└── internal/
    ├── mod.rs                — re-exports only
    ├── types.rs              — SpecType registry, SPEC_TYPES, body templates
    ├── queries.rs            — ReadySpec, BlockedSpec, SpecInfo, ListFilters,
    │                           get_ready/blocked/all_specs, show_*,
    │                           scan_disk_specs  (~590 lines from 3 sections)
    ├── mutations.rs          — mutate_spec, with_yaml_rollback, next_tag_number,
    │                           promote, complete, abandon, pause, resume, block
    │                           + all _value() variants  (~726 lines)
    ├── split.rs              — split_spec, split_spec_value  (~201 lines)
    ├── archive.rs            — archive_spec, archive_stale, archive_spec_inner,
    │                           find_spec, find_spec_file_on_disk,
    │                           resolve_spec_dir  (~260 lines)
    └── queue.rs              — next_spec, next_spec_value, spec_age_days_from_list,
                                load_dep_counts, tag_exists, is_tree_clean
                                + tests  (~272 lines)
```

**7 files, not 8.** The original draft proposed a separate `util.rs` for
`find_spec`, `tag_exists`, `is_tree_clean`, `next_tag_number`, and
`resolve_spec_dir`. Code review shows these belong with their callers:

- `find_spec` + `find_spec_file_on_disk` + `resolve_spec_dir` are
  called primarily by archive functions → keep in `archive.rs`
- `tag_exists` + `is_tree_clean` are tiny delegations (1 line each)
  used by archive and queue → keep in `queue.rs` (where tests live)
- `mutate_spec` + `with_yaml_rollback` + `next_tag_number` are shared
  mutation infrastructure → keep in `mutations.rs`

A 30-line `util.rs` would be artificial. The assay split has `util.rs`
because `truncate()` is genuinely shared across 4 files. Here, each
helper has a clear home.

### Shared imports and constants

`internal/mod.rs` declares shared imports that all submodules need:

```rust
// Shared constant — used by queries, archive, mutations
pub(crate) const DB_PATH: &str = ".patina/local/data/patina.db";
```

Each submodule imports what it needs locally. The current file has one
`use` block at the top (lines 1-11) — these distribute to their new homes.

### Re-export contract in internal/mod.rs

`mod.rs` (the parent) currently re-exports via `pub(crate) use internal::{...}`.
The internal `mod.rs` must re-export everything that parent `mod.rs` needs.
Current re-exports from parent `mod.rs`:

```rust
// Data types + functions for session integration
pub(crate) use internal::{
    get_all_specs, get_blocked_specs, load_dep_counts,
    spec_age_days_from_list, ListFilters,
};

// Query functions for MCP
pub(crate) use internal::{get_ready_specs, next_spec_value};

// Mutation _value() functions for MCP
pub(crate) use internal::{
    abandon_spec_value, block_spec_value, complete_spec_value,
    pause_spec_value, promote_spec_value, resume_spec_value,
    split_spec_value,
};
```

Plus the `pub fn` wrappers in `mod.rs` call `internal::show_ready_specs`,
`internal::show_blocked_specs`, `internal::show_spec_list`,
`internal::archive_spec`, `internal::archive_stale_specs`,
`internal::promote_spec`, `internal::complete_spec`, etc.

The internal `mod.rs` re-exports all of these from the appropriate
submodules. Pattern: `pub(super) use queries::{...};` etc.

### Extract SpecType registry into types.rs

```rust
use patina::release::BumpType;

pub struct SpecType {
    pub name: &'static str,
    pub bump: Option<BumpType>,
    pub directory: &'static str,
    pub body_template: &'static str,
}

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

pub fn lookup(name: &str) -> Option<&'static SpecType> {
    SPEC_TYPES.iter().find(|t| t.name == name)
}
```

**Body templates per type** (from survey of 117 archived specs — 88
explore, 19 feat, 4 refactor, 5 fix — and the current draft specs):

- **feat**: Problem, Solution, Design Decisions, Implementation, Exit
  Criteria, Key Files, Non-Goals, Provenance
- **fix**: Problem, Solution, Exit Criteria, Key Files, Provenance
- **refactor**: Problem, Solution, Migration, Exit Criteria, Key Files,
  Provenance
- **explore**: Exit Criteria (checklist only — lightest weight)

Templates are `&'static str` constants (not `include_str!`). They are
short section-heading scaffolds, not full documents. The LLM and user
fill in the body.

### Wire BumpType to the registry

`BumpType::from_spec_type()` in `src/release/mod.rs` currently hardcodes:

```rust
match spec_type {
    "fix" | "refactor" => Some(BumpType::Patch),
    "feat" => Some(BumpType::Minor),
    _ => None,
}
```

After this refactor, it delegates to the registry:

```rust
pub fn from_spec_type(spec_type: &str) -> Option<Self> {
    crate::commands::spec::types::lookup(spec_type)
        .and_then(|t| t.bump)
}
```

Single source of truth. The match arm disappears.

**Note:** This requires `types.rs` to be `pub(crate)` accessible from
`release/mod.rs`. The internal `mod.rs` re-exports types at crate level:
`pub(crate) mod types;` or `pub(crate) use types::lookup;`. Parent
`mod.rs` then adds `pub(crate) use internal::types;` to its re-exports.

### Cross-module dependencies within internal/

Functions call each other across sections. These become cross-module
calls within `internal/`:

- `queries.rs` calls `spec_age_days_from_list()` from `queue.rs`
- `queries.rs` calls `load_dep_counts()` from `queue.rs`
- `archive.rs` calls `scan_disk_specs()` from `queries.rs`
  (find_spec filesystem fallback)
- `archive.rs` calls `get_all_specs()` from `queries.rs`
  (archive_stale_specs)
- `archive.rs` calls `tag_exists()` from `queue.rs`
- `archive.rs` calls `is_tree_clean()` from `queue.rs`
- `mutations.rs` calls `find_spec()` from `archive.rs`
- `mutations.rs` calls `get_all_specs()` from `queries.rs`
  (pause checks one-paused-spec rule)
- `mutations.rs` calls `tag_exists()` from `queue.rs`
  (complete/abandon pre-check archive tag)
- `split.rs` calls `find_spec()` from `archive.rs`
- `split.rs` calls `archive_spec_inner()` from `archive.rs`
- `split.rs` calls `resolve_spec_dir()` from `archive.rs`
- `split.rs` calls `tag_exists()` from `queue.rs`
- `queue.rs` calls `get_all_specs()` from `queries.rs`
- `queue.rs` calls `get_blocked_specs()` from `queries.rs`
- `queue.rs` calls `find_spec()` from `archive.rs`
  (spec_age_days_from_list reads frontmatter)
- `queue.rs` calls `load_dep_counts()` (self)

All resolved via `super::` imports within `internal/` or `pub(super)`
visibility.

## Migration

Pure restructuring — no behavior changes, no API changes, no new
dependencies. `mod.rs` public interface stays identical. Tests move
with their functions (4 tests in the `#[cfg(test)] mod tests` block
at the bottom move to their respective files).

Implementation order:
1. Create `internal/` directory
2. Move sections to files (largest first: mutations, queries, archive)
3. Wire `internal/mod.rs` re-exports
4. Add `types.rs` with registry
5. Wire `BumpType::from_spec_type()` delegation
6. Delete `internal.rs`
7. Verify: `cargo test`, `cargo clippy`, `cargo build --release`

## Exit Criteria

- [x] `internal/` directory with 6 files (mod, queries, mutations, split, archive, queue)
- [x] `mod.rs` public API identical — no signature changes, same re-exports
- [x] All existing tests pass (4 unit tests + mod.rs clap tests)
- [x] `cargo clippy` clean
- [x] `internal.rs` deleted (replaced by `internal/`)

### Deferred to spec-create

- `types.rs` (SpecType registry, body templates) — deferred because it
  has no callers until `create.rs` uses it. Dead code violates project
  belief `dead-code-requires-decision`. Design preserved in this SPEC.md
  section "Extract SpecType registry into types.rs".
- `BumpType::from_spec_type()` delegation — `release/mod.rs` is in the
  lib crate and cannot import from the binary crate's `types.rs`.
  Delegation possible when types move to the lib crate.

## Key Files

```
src/commands/spec/internal.rs         — delete (replaced by internal/)
src/commands/spec/internal/mod.rs     — re-exports, DB_PATH constant
src/commands/spec/internal/types.rs   — SpecType, SPEC_TYPES, lookup(), templates
src/commands/spec/internal/queries.rs — ReadySpec, BlockedSpec, SpecInfo, ListFilters
src/commands/spec/internal/mutations.rs — mutate_spec, promote..block + _value()
src/commands/spec/internal/split.rs   — split_spec, split_spec_value
src/commands/spec/internal/archive.rs — archive_*, find_spec*, resolve_spec_dir
src/commands/spec/internal/queue.rs   — next_spec, age, dep_counts, tag/tree helpers
src/release/mod.rs                    — from_spec_type() delegates to registry
```

## Provenance

Motivated by spec-create needing to add ~200 lines of create logic
plus type registry data to an already-2063-line file. Follows the
pattern established by assay (10 files), scry (7 files), and eval
(5 files) modules. Blocks spec-create — do the split first so
`create.rs` lands in a clean directory structure.
