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
Misspell `"file"` as `"flie"` and the compiler won't catch it.

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

### Refactor 1: Two-tier `find_spec` + `mutate_spec` takes `FoundSpec`

**The problem with "always load":** `find_spec` is also used by
read-only flows — `archive_spec` (status validation), `archive_stale_specs`
(fan-out loop over all stale specs), `spec_age_days_from_list` (called
per-spec during list/ready views), and `block_spec_value` line 596
(blocker existence check). Forcing a full `read_to_string + parse` for
every call inflates latency in fan-out paths and wastes memory for
callers that only need `file_path` and `status`.

**Solution: two functions, not one mutant.**

Keep `find_spec` lightweight (DB query or filesystem lookup, returns
current `FoundSpec { file_path, status, title }`). Add `load_spec`:

```rust
pub(super) struct LoadedSpec {
    pub file_path: String,
    pub status: Option<String>,
    pub title: Option<String>,
    pub content: String,
    pub frontmatter: SpecFrontmatter,
    pub body: String,
}

/// Load a spec fully from disk (read + parse). For mutations.
pub(super) fn load_spec(id: &str) -> Result<LoadedSpec> {
    let found = find_spec(id)?;
    let content = std::fs::read_to_string(&found.file_path)
        .with_context(|| format!("Failed to read {}", found.file_path))?;
    let (frontmatter, body) = parse_spec_file(&content)
        .with_context(|| format!("Failed to parse {}", found.file_path))?;
    Ok(LoadedSpec {
        file_path: found.file_path,
        status: found.status,
        title: found.title,
        content,
        frontmatter,
        body,
    })
}
```

Read-only callers keep using `find_spec`. Mutation callers use `load_spec`.
No behavior change for `archive_stale_specs`, `spec_age_days_from_list`,
or blocker existence checks.

**`mutate_spec` takes `LoadedSpec`:**

```rust
pub(super) fn mutate_spec<F>(
    loaded: LoadedSpec,
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

Implementation: clone `loaded.frontmatter` as `pre`, apply closure to
get `post`, serialize `(post, loaded.body)`, write file, update DB.
No `find_spec` inside. No `id` parameter needed — but see "ID
source-of-truth" below.

`SpecFrontmatter` already derives `Clone` (src/spec.rs:57). The struct
is 16 fields, mostly `Option<String>` and small `Vec<String>`. Clone
cost is negligible for single-spec mutation paths. No fan-out.

**Convenience wrapper** for `promote_spec_value` (the one caller that
doesn't pre-validate):

```rust
pub(super) fn load_and_mutate<F>(id: &str, mutate: F) -> Result<MutationOutput>
where
    F: FnOnce(&mut SpecFrontmatter) -> Result<()>,
{
    let loaded = load_spec(id)?;
    mutate_spec(loaded, mutate)
}
```

**ID source-of-truth invariant:** Today `mutate_spec` takes an explicit
`id: &str` from the caller, uses it for the DB UPDATE. The new design
infers id from `loaded.frontmatter.id`. These could diverge if
`find_spec` returns a file whose frontmatter ID doesn't match the
lookup key. This can't happen via the DB path (DB stores the frontmatter
ID). It *could* happen via the filesystem fallback if two specs share
an ID in their frontmatter (a data bug). Defense: `load_spec` asserts
`frontmatter.id == id` after parse. Bail with a clear error if they
diverge. Cost: one string comparison per load.

```rust
if frontmatter.id != id {
    anyhow::bail!(
        "Frontmatter ID '{}' doesn't match lookup key '{}' in {}",
        frontmatter.id, id, found.file_path
    );
}
```

**This single change eliminates:**
- Double `find_spec` in all 6 pre-querying `_value()` functions
- The manual read-parse-mutate-write-DB in `split_spec_value`
- The extra `read_to_string + parse_spec_file` in `resume_spec_value`
  (use `pre.paused_at_tag` and `pre.blocked_by` from `MutationOutput`)

**`resume_spec_value` ordering:** The blocker check currently happens
before mutation. With `LoadedSpec`, `resume_spec_value` does:
1. `load_spec(id)` — gets `loaded.frontmatter.blocked_by` and
   `loaded.frontmatter.paused_at_tag`
2. Validate `loaded.status` (paused or blocked)
3. Check blockers using `loaded.frontmatter.blocked_by` (calls
   `find_spec` per blocker — lightweight, only needs status)
4. `mutate_spec(loaded, ...)` — one write, no re-reads

The pre-mutation data comes from `loaded` before it's consumed by
`mutate_spec`. The `MutationOutput.pre` field is also available after
mutation for anything the return payload needs.

### Rollback interaction with `with_yaml_rollback`

`pause_spec_value` and `block_spec_value` use `with_yaml_rollback`,
which reads the file content before the closure and restores it on
failure. Currently:

```
with_yaml_rollback(file_path, || {
    mutate_spec(id, |fm| { ... })?;   // re-reads file inside
    git_create_tag(...)?;              // can fail
    git_stage_and_commit(...)?;        // can fail
})
```

`with_yaml_rollback` owns the pre-mutation content. `mutate_spec`
re-reads (redundantly) and writes. If git ops fail after the write,
`with_yaml_rollback` restores from its backup. This works but is
wasteful.

With `LoadedSpec`, the flow becomes:

```
let loaded = load_spec(id)?;         // one read
let original_content = loaded.content.clone();  // backup for rollback

// with_yaml_rollback now takes the backup explicitly:
with_content_rollback(&loaded.file_path, &original_content, || {
    mutate_spec(loaded, |fm| { ... })?;  // writes, no re-read
    git_create_tag(...)?;
    git_stage_and_commit(...)?;
})
```

Or simpler: `mutate_spec` stores `loaded.content` and handles its own
rollback internally if the closure fails, making `with_yaml_rollback`
unnecessary for mutation paths. The rollback is file-write-level:
`mutate_spec` writes `serialize(post, body)`, so on failure it writes
back `loaded.content`. But `pause` and `block` do git ops *after* the
YAML write that can also fail — the rollback scope is wider than
`mutate_spec`.

**Decision: keep `with_yaml_rollback` but pass the backup content
explicitly.** Rename to `with_content_rollback(file_path, backup, action)`
to make the contract clear. `LoadedSpec.content` is the backup. No
redundant reads.

### Refactor 2: Extract `release_and_archive` helper

`complete_spec_value` and `split_spec_value` share identical
release+archive logic (~20 lines). Extract to:

```rust
/// Release (version bump + archive) or archive-only for a completed spec.
pub(super) fn release_and_archive(
    id: &str,
    file_path: &str,
    frontmatter: &SpecFrontmatter,
    title: &str,
    bump: Option<BumpType>,
) -> Result<()>
```

**Responsibilities (exhaustive):**
1. Pre-check: bail if `spec/{id}` tag already exists
2. Resolve spec directory from file_path
3. If `bump` is `Some`: `ReleaseStrategy::from_project` -> `preflight`
   -> `execute` (this stages, commits, and tags the release). Then
   `create_tag_at("spec/{id}", ..., "HEAD~1")` for archive tag.
4. If `bump` is `None`: delegate to `archive_spec_inner` (which does
   `git rm -rf` + `commit` + `create_tag_at`).

**Does NOT:** call `mutate_spec`, validate status, or stage files.
Caller handles mutation before calling this. This helper is
archive-side only.

**Overlap with `archive_spec_inner`:** `release_and_archive` calls
`archive_spec_inner` for the no-release path. It does NOT duplicate
`archive_spec_inner`'s git rm / commit / tag logic — it delegates.
For the release path, `ReleaseStrategy::execute` handles staging and
committing; this helper only adds the archive tag afterward.

Caller passes `bump` directly — `complete_spec_value` computes it
from the `major` CLI flag, `split_spec_value` always uses
`BumpType::from_spec_type(...)`. The helper doesn't know about CLI
flags.

Place in `archive.rs` alongside `archive_spec_inner`.

### Refactor 3: Typed results for `_value()` functions

**Payload inventory (grounded against current code):**

| Function | Fields |
|----------|--------|
| `promote_spec_value` | command, spec_id, new_status, file |
| `complete_spec_value` | command, spec_id, new_status, archived, tag, file |
| `abandon_spec_value` | command, spec_id, new_status, reason, archived, tag, file |
| `pause_spec_value` | command, spec_id, new_status, reason, tag, paused_date |
| `resume_spec_value` | command, spec_id, new_status, previous_status, tag, paused_at_tag |
| `block_spec_value` | command, spec_id, new_status, blocker, reason, tag |
| `split_spec_value` | command, original_spec_id, new_spec_id, version_tag, archive_tag, new_spec_path, original_file, status |

**Common base** (all 6 mutations share): `command`, `spec_id`, `new_status`.
**Frequent** (5 of 6): `tag`. **Frequent** (4 of 6): `file`.
**Command-specific**: `reason` (3), `archived` (2), `blocker` (1),
`previous_status` (1), `paused_at_tag` (1), `paused_date` (1).

`split` is structurally different (two spec IDs, two tags, a path).

**Design: base struct + `#[serde(flatten)]` detail enum.**

```rust
#[derive(Debug, Serialize)]
pub struct MutationResult {
    pub command: &'static str,
    pub spec_id: String,
    pub new_status: String,
    #[serde(flatten)]
    pub detail: MutationDetail,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum MutationDetail {
    Promote {
        file: String,
    },
    Complete {
        file: String,
        tag: String,
        archived: bool,
    },
    Abandon {
        file: String,
        tag: String,
        archived: bool,
        reason: Option<String>,
    },
    Pause {
        tag: String,
        reason: String,
        paused_date: String,
    },
    Resume {
        tag: String,
        previous_status: String,
        paused_at_tag: Option<String>,
    },
    Block {
        tag: String,
        blocker: String,
        reason: String,
    },
}
```

No silent field drops — every current payload field has a home. No
`Option` soup — each variant carries exactly its fields. The enum is
`#[serde(untagged)]` so JSON output is flat (no wrapper key),
preserving the current MCP contract.

`SplitResult` stays separate (different shape, different file).

**CLI wrappers** access detail fields via match:

```rust
let result = promote_spec_value(id)?;
match &result.detail {
    MutationDetail::Promote { file } => {
        println!("Promoted: {} → {}", id, result.new_status);
        println!("  File: {}", file);
    }
    _ => unreachable!(),
}
```

Or — simpler — each `_spec()` wrapper already knows what command it
called, so it can destructure directly. The `unreachable!` is safe
because the wrapper and `_value()` are coupled by design.

**MCP layer:** `serde_json::to_value(&result)?` at the boundary.
Current JSON shape preserved.

**`next_spec_value` stays as `serde_json::Value`** — it returns an
array of `Recommendation` structs that are already typed (local
struct with `#[derive(Serialize)]` in `queue.rs:88`). Its return type
is fine; the problem was only in the mutation functions.

## Implementation Order

```
1. cleanup: remove banners + archive_spec_inner signature      [mechanical]
2. refactor: add load_spec, mutate_spec takes LoadedSpec       [Refactor 1]
   - add LoadedSpec, load_spec, MutationOutput
   - mutate_spec takes LoadedSpec, returns MutationOutput
   - add load_and_mutate convenience wrapper
   - add ID assertion in load_spec
   - rename with_yaml_rollback → with_content_rollback
   - update all _value() callers
   - split_spec_value uses mutate_spec (no more manual path)
   - resume_spec_value uses loaded.frontmatter (no triple-read)
3. refactor: extract release_and_archive helper                [Refactor 2]
4. refactor: MutationResult + MutationDetail + SplitResult     [Refactor 3]
```

Commit 2 is the one that matters. Everything else is either trivial
cleanup or type-safety polish.

## Exit Criteria

- [x] No manual read-parse-mutate-write-DB outside `mutate_spec`
- [x] No double `find_spec` calls in any `_value()` function
- [x] `resume_spec_value` reads the spec file exactly once
- [x] `find_spec` remains lightweight (no file reads) for read-only callers
- [x] `load_spec` asserts `frontmatter.id == lookup_key`
- [x] `with_content_rollback` takes explicit backup (no redundant reads)
- [x] `_value()` functions return `MutationResult` or `SplitResult`
- [x] Every current JSON field has a home (no silent drops)
- [x] `archive_spec_inner` takes `Option<&Path>`
- [x] Zero `// ====` section banners in `internal/` files
- [x] `complete_spec_value` and `split_spec_value` share release logic
- [x] All existing tests pass
- [x] `cargo clippy` clean
- [x] Pre-push checks green

All 4 steps complete. Spec ready for promote → complete.

## Key Files

```
src/commands/spec/internal/mutations.rs  — mutate_spec, MutationOutput, MutationResult,
                                           MutationDetail, load_and_mutate,
                                           with_content_rollback
src/commands/spec/internal/split.rs      — uses mutate_spec via load_spec, SplitResult
src/commands/spec/internal/archive.rs    — LoadedSpec, load_spec, find_spec (unchanged),
                                           release_and_archive, signature fix
src/commands/spec/internal/queue.rs      — no changes (next_spec_value already typed)
src/commands/spec/internal/queries.rs    — banner removal only
src/commands/spec/internal/mod.rs        — re-export MutationResult, SplitResult
src/mcp/spec_tools.rs                   — serde_json::to_value at boundary
```

## Non-Goals

- Changing public CLI behavior or output format
- Restructuring the module layout (that was spec-module-split)
- Adding new spec commands or features
- Loading full content in `find_spec` (read-only paths stay lightweight)

## Provenance

Identified by Gjengset-style code review after spec-module-split
(v0.30.1). All items are pre-existing issues the split exposed. Previous
session (20260224-141429) fixed 4 items (git helper, FoundSpec struct,
mutate_spec usage, git API consolidation). This spec covers the
remaining issues with feedback incorporated from two review passes
that caught: over-counting (7 fixes → 3 refactors + mechanical),
design mistakes (Option<FoundSpec> → two-tier find/load, tuple returns
→ named struct, 7 result structs → enum, major:bool → Option<BumpType>),
and potential pitfalls (fan-out latency for loaded find_spec, ID
source-of-truth drift, rollback interaction with loaded content,
silent field drops in typed results).
