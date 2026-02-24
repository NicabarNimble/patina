---
type: fix
id: spec-mutation-cleanup
status: draft
created: 2026-02-24
sessions:
  origin: 20260224-180727
related:
  - src/commands/spec/internal/mutations.rs
  - src/commands/spec/internal/split.rs
  - src/commands/spec/internal/archive.rs
  - src/commands/spec/internal/queue.rs
  - src/commands/spec/internal/queries.rs
beliefs:
  - dependable-rust
  - specs-ship-features-audits-ship-quality
provenance:
  - spec-module-split
---

# fix: Spec Mutation Cleanup

> Post-implementation audit of spec-module-split identified redundant I/O,
> stringly-typed returns, and leftover noise in `src/commands/spec/internal/`.
> None are bugs. All are making the type system work harder for us.

## Problem

After completing spec-module-split (v0.30.1), a Gjengset-style code
review found three categories of issue:

**Redundant I/O.** `mutate_spec` does its own `find_spec` internally.
Every `_value()` caller that validates status first calls `find_spec`,
then calls `mutate_spec`, which calls `find_spec` again. Two DB
round-trips per operation. `resume_spec_value` is worst: `find_spec`
(L466), `read_to_string + parse_spec_file` (L479-482) for
`paused_at_tag`, then `mutate_spec` (L518) which does
`find_spec + read + parse` again. Three reads, three parses.
`split_spec_value` also bypasses `mutate_spec` entirely (L84-100),
doing manual read-parse-mutate-write-DB.

**Stringly-typed returns.** All 7 `_value()` functions return
`serde_json::Value`. Callers do `result["file"].as_str().unwrap_or("")`.
Misspell `"file"` as `"flie"` and the compiler won't catch it. Six of
the seven share the same shape: `spec_id`, `new_status`, `tag`, `file`.

**Noise and ergonomic nits.** 7 fossil `// ====` section banners from
the pre-split era. `archive_spec_inner` takes `&Option<PathBuf>` instead
of `Option<&Path>`. `complete_spec_value` and `split_spec_value` share
~20 lines of release+archive logic with no helper.

## Solution

### Prerequisite: Mechanical cleanup (no spec needed, just do it)

These are zero-judgment changes done before the real refactoring:

- Remove all 7 `// ====` banner blocks (7 locations across 5 files)
- Change `archive_spec_inner` signature from `&Option<PathBuf>` to
  `Option<&Path>`, update 5 call sites to `spec_dir.as_deref()`

### Refactor 1: `FoundSpec` becomes a loaded spec, `mutate_spec` takes it

**Core idea:** `FoundSpec` already has `file_path`, `status`, `title`.
Extend it with `content` and `frontmatter` so it's a fully loaded spec.
Then `mutate_spec` always takes `FoundSpec` — it never does its own
lookup. One responsibility: mutate what you hand it.

```rust
pub(super) struct FoundSpec {
    pub file_path: String,
    pub status: Option<String>,
    pub title: Option<String>,
    pub content: String,
    pub frontmatter: SpecFrontmatter,
}
```

`find_spec` already reads the file for the filesystem fallback path.
For the DB path, add the `read_to_string + parse_spec_file` after the
DB query succeeds. One read per operation, always.

**`mutate_spec` new signature:**

```rust
pub(super) fn mutate_spec<F>(
    found: FoundSpec,
    mutate: F,
) -> Result<MutationOutput>
where
    F: FnOnce(&mut SpecFrontmatter) -> Result<()>,
```

```rust
pub(super) struct MutationOutput {
    pub file_path: String,
    pub pre: SpecFrontmatter,
    pub post: SpecFrontmatter,
}
```

Named fields, not `(String, SpecFrontmatter, SpecFrontmatter)` where
you need a comment to know which is which.

`mutate_spec` clones the frontmatter before mutation (that's `pre`),
applies the closure (that's `post`), writes the file, updates the DB.
No `find_spec` inside. No `id` parameter — it gets `id` from
`found.frontmatter.id`.

**Convenience wrapper** for the one caller (`promote_spec_value`) that
doesn't pre-query:

```rust
pub(super) fn find_and_mutate<F>(id: &str, mutate: F) -> Result<MutationOutput>
where
    F: FnOnce(&mut SpecFrontmatter) -> Result<()>,
{
    let found = find_spec(id)?;
    mutate_spec(found, mutate)
}
```

**This single change eliminates:**
- Double `find_spec` in all 6 pre-querying `_value()` functions
- The manual read-parse-mutate-write-DB in `split_spec_value`
- The extra `read_to_string + parse_spec_file` in `resume_spec_value`
  (use `pre.paused_at_tag` and `pre.blocked_by` from `MutationOutput`)
- The `resume` ordering problem: validate with `found.status` and
  `found.frontmatter.blocked_by` before calling `mutate_spec`

### Refactor 2: Extract `release_and_archive` helper

`complete_spec_value` and `split_spec_value` share identical
release+archive logic (~20 lines). Extract to:

```rust
pub(super) fn release_and_archive(
    id: &str,
    file_path: &str,
    frontmatter: &SpecFrontmatter,
    title: &str,
    bump: Option<BumpType>,  // caller decides, not the helper
) -> Result<()>
```

Caller passes `bump` directly — `complete_spec_value` computes it from
the `major` CLI flag, `split_spec_value` always uses
`BumpType::from_spec_type(...)`. The helper doesn't know about CLI
flags. Separation of concerns.

Place in `archive.rs` alongside `archive_spec_inner`. Both callers
import via `super::archive::release_and_archive`.

### Refactor 3: Typed `MutationResult` for `_value()` functions

One struct, not seven. Six of seven share the same shape:

```rust
#[derive(Serialize)]
pub struct MutationResult {
    pub command: &'static str,
    pub spec_id: String,
    pub new_status: String,
    pub file: String,
    pub tag: Option<String>,
    pub archived: bool,
    // Command-specific extras
    pub reason: Option<String>,
    pub previous_status: Option<String>,
    pub blocker: Option<String>,
}
```

`split_spec_value` is the outlier (has `new_spec_id`, `version_tag`,
`archive_tag`, `new_spec_path`, `original_file`). It gets its own:

```rust
#[derive(Serialize)]
pub struct SplitResult {
    pub command: &'static str,
    pub original_spec_id: String,
    pub new_spec_id: String,
    pub version_tag: String,
    pub archive_tag: String,
    pub new_spec_path: String,
    pub original_file: String,
}
```

Two structs total. CLI wrappers access fields directly (`result.file`
instead of `result["file"].as_str().unwrap_or("")`). MCP layer calls
`serde_json::to_value(&result)?` at the boundary.

## Implementation Order

```
1. cleanup: remove banners + archive_spec_inner signature     [mechanical]
2. refactor: FoundSpec loaded, mutate_spec takes it            [Refactor 1]
3. refactor: extract release_and_archive helper                [Refactor 2]
4. refactor: MutationResult + SplitResult typed returns        [Refactor 3]
```

Commit 2 is the one that matters. Everything else is either trivial
cleanup or optional polish.

## Exit Criteria

- [ ] No manual read-parse-mutate-write-DB outside `mutate_spec`
- [ ] No double `find_spec` calls in any `_value()` function
- [ ] `resume_spec_value` reads the spec file exactly once
- [ ] `_value()` functions return `MutationResult` or `SplitResult`
- [ ] `archive_spec_inner` takes `Option<&Path>`
- [ ] Zero `// ====` section banners in `internal/` files
- [ ] `complete_spec_value` and `split_spec_value` share release logic
- [ ] All existing tests pass
- [ ] `cargo clippy` clean
- [ ] Pre-push checks green

## Key Files

```
src/commands/spec/internal/mutations.rs  — mutate_spec, MutationResult, find_and_mutate
src/commands/spec/internal/split.rs      — uses mutate_spec, SplitResult
src/commands/spec/internal/archive.rs    — FoundSpec loaded, release_and_archive, signature fix
src/commands/spec/internal/queue.rs      — MutationResult for next_spec_value (or keep json?)
src/commands/spec/internal/queries.rs    — banner removal only
src/commands/spec/internal/mod.rs        — re-export MutationResult, SplitResult
src/mcp/spec_tools.rs                   — serde_json::to_value at boundary
```

## Non-Goals

- Changing public CLI behavior or output format
- Restructuring the module layout (that was spec-module-split)
- Adding new spec commands or features

## Provenance

Identified by Gjengset-style code review after spec-module-split
(v0.30.1). All items are pre-existing issues the split exposed. Previous
session (20260224-141429) fixed 4 items (git helper, FoundSpec struct,
mutate_spec usage, git API consolidation). This spec covers the
remaining issues with the feedback incorporated from a second review
pass that caught over-counting (7 fixes were really 3 refactors +
mechanical cleanup) and design mistakes (Option<FoundSpec> parameter,
tuple returns, 7 result structs, major:bool leaking into helpers).
