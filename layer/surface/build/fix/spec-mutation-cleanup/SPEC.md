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
  - dead-code-requires-decision
provenance:
  - spec-module-split
---

# fix: Spec Mutation Cleanup

> Post-implementation audit of spec-module-split found 7 code quality
> issues. None are bugs or correctness problems — all are type safety,
> redundant I/O, and noise removal. Identified via Gjengset-style review.

## Problem

After completing spec-module-split (v0.30.1), a structured code review
identified these issues in `src/commands/spec/internal/`:

1. **`split_spec_value` bypasses `mutate_spec`** — Lines 84-100 of
   `split.rs` manually do read-parse-mutate-write-DB, the exact pattern
   removed from `complete` and `abandon` in the previous session. The
   comment even says "Update status to complete" — that's a `mutate_spec`
   call.

2. **Double `find_spec` on every mutation** — 6 of 7 `_value()` functions
   call `find_spec(id)` for validation, then call `mutate_spec(id, ...)`
   which internally calls `find_spec(id)` again. Two DB round-trips per
   operation.

3. **`resume_spec_value` triple-reads** — `find_spec` (L466), then
   `read_to_string + parse_spec_file` (L479-482) to get `paused_at_tag`,
   then `mutate_spec` (L518) which does `find_spec + read + parse` again.
   Three reads, three parses of the same file. The extra read exists
   because `mutate_spec`'s closure doesn't expose the pre-mutation
   frontmatter.

4. **`serde_json::Value` as return type for all `_value()` functions** —
   All 7 `_value()` functions return `serde_json::Value`. Callers do
   `result["file"].as_str().unwrap_or("")`. If someone misspells `"file"`
   as `"flie"`, the compiler won't catch it. These should be proper
   structs, serialized to JSON only at the MCP boundary.

5. **`archive_spec_inner` takes `&Option<PathBuf>`** — Should be
   `Option<&Path>`. Borrowing inside the Option is more ergonomic and
   idiomatic. Callers use `spec_dir.as_deref()`.

6. **Section banners are fossils** — 7 `// ====` banners across 5 files.
   These were section markers when everything was in one 2063-line file.
   Now each file IS the section. `mutations.rs` doesn't need a banner
   saying it's mutations.

7. **`complete_spec_value` and `split_spec_value` share release+archive
   block** — Both do: validate status, mutate, pre-check archive tag,
   resolve spec_dir, ReleaseStrategy::from_project, preflight, execute OR
   archive_spec_inner. ~20 lines duplicated.

## Solution

### Fix 1: `split_spec_value` uses `mutate_spec` (quick win)

Replace lines 84-100 of `split.rs` with:

```rust
let (file_path, frontmatter) = mutate_spec(id, |fm| {
    fm.status = Some("complete".to_string());
    Ok(())
})?;
```

Same pattern already used by `complete_spec_value` and
`abandon_spec_value`. The manual read-parse-mutate-write-DB block
disappears.

**Prerequisite:** Fix 2 must land first if we want `split_spec_value` to
avoid the double `find_spec`, since it currently calls `find_spec`
before the manual mutation block.

### Fix 2: `mutate_spec` accepts optional `FoundSpec` (kills double lookup)

Change signature from:

```rust
pub(super) fn mutate_spec<F>(id: &str, mutate: F) -> Result<(String, SpecFrontmatter)>
```

to:

```rust
pub(super) fn mutate_spec<F>(
    id: &str,
    found: Option<FoundSpec>,
    mutate: F,
) -> Result<(String, SpecFrontmatter, SpecFrontmatter)>
//                                    ^^^^^^^^^^^^^^^^^ pre-mutation snapshot
```

Changes:
- Accept `Option<FoundSpec>` — if `Some`, skip internal `find_spec`.
  If `None`, call `find_spec` internally (backwards compatible).
- Return `(file_path, pre_mutation_fm, post_mutation_fm)` — the
  pre-mutation frontmatter gives callers access to the old state
  without re-reading.

This eliminates:
- Double `find_spec` in `complete_spec_value`, `abandon_spec_value`,
  `pause_spec_value`, `resume_spec_value`, `block_spec_value`,
  `split_spec_value`
- The explicit `read_to_string + parse_spec_file` in
  `resume_spec_value` (L479-482) — pre-mutation snapshot gives
  `paused_at_tag` directly

**Call site migration for `_value()` functions:**

```rust
// Before:
let found = find_spec(id)?;
// ... validate found.status ...
let (file_path, fm) = mutate_spec(id, |fm| { ... })?;

// After:
let found = find_spec(id)?;
// ... validate found.status ...
let (file_path, _pre, fm) = mutate_spec(id, Some(found), |fm| { ... })?;
```

**Call sites that don't pre-query** (e.g., `promote_spec_value`):

```rust
// Before:
let (file_path, fm) = mutate_spec(id, |fm| { ... })?;

// After:
let (file_path, _pre, fm) = mutate_spec(id, None, |fm| { ... })?;
```

All callers update in one pass. The `_pre` return is unused by most
callers initially but enables Fix 3 for `resume`.

### Fix 3: `resume_spec_value` uses pre-mutation snapshot (kills triple-read)

After Fix 2, replace:

```rust
// 2. Read current frontmatter to get paused_at_tag and check blockers
let content = std::fs::read_to_string(&found.file_path)?;
let (fm_snapshot, _) = parse_spec_file(&content)?;
let paused_at_tag = fm_snapshot.paused_at_tag.clone();
```

with:

```rust
// paused_at_tag comes from the pre-mutation snapshot returned by mutate_spec
```

Move the `mutate_spec` call before the blocker check, or restructure so
the validation uses `found.status` (already available) and the blocker
check uses the pre-mutation frontmatter from `mutate_spec`.

**Ordering consideration:** The blocker check currently happens before
`mutate_spec`. We need the pre-mutation frontmatter for the blocker
check. Two options:

**(A)** Read `FoundSpec` fields to get `blocked_by` — requires adding
`blocked_by` and `paused_at_tag` to `FoundSpec` (DB query or file
parse at `find_spec` time).

**(B)** Call `mutate_spec` first (it reads and parses), use `pre_fm`
for blocker check, bail after mutation if blockers incomplete, then
roll back. This is worse — mutating before validation.

**(C)** Accept the extra read for `resume` specifically. The common
path (promote, complete, abandon, pause, block) all benefit from Fix 2
without this complication. `resume` is the only one that needs
pre-mutation frontmatter fields beyond `status`.

**Recommendation:** Option A if `FoundSpec` is cheap to extend (it
already queries DB). Option C if we don't want to change `find_spec`
scope. Either way, Fix 2 still eliminates the double lookup for all
other callers.

### Fix 4: Typed result structs for `_value()` functions

Replace `serde_json::Value` returns with proper structs:

```rust
#[derive(Serialize)]
pub struct PromoteResult {
    pub command: &'static str,
    pub spec_id: String,
    pub new_status: String,
    pub file: String,
}

#[derive(Serialize)]
pub struct CompleteResult {
    pub command: &'static str,
    pub spec_id: String,
    pub new_status: String,
    pub archived: bool,
    pub tag: String,
    pub file: String,
}

// ... similar for Abandon, Pause, Resume, Block, Split, Next
```

Each `_value()` function returns `Result<XxxResult>`. The CLI callers
access fields directly (`result.file`). The MCP layer serializes to
JSON at the boundary:

```rust
// In MCP handler:
let result = promote_spec_value(id)?;
Ok(serde_json::to_value(&result)?)
```

**Scope:** 7 result structs + 7 `_value()` return type changes + ~14
CLI callers updated (each `_spec()` wrapper accesses fields) + MCP
handler serialization. The MCP handlers already receive
`serde_json::Value` and pass it through — they'd call `to_value()` on
the typed result instead.

**Where to put the structs:** In `mutations.rs` (for 6 mutation results)
and `queue.rs` (for `NextResult`). They're implementation details, not
public API. Re-export through `internal/mod.rs` only what MCP needs.

### Fix 5: `archive_spec_inner` signature (quick win)

Change:

```rust
pub(super) fn archive_spec_inner(
    ...
    spec_dir: &Option<std::path::PathBuf>,
) -> Result<()>
```

to:

```rust
pub(super) fn archive_spec_inner(
    ...
    spec_dir: Option<&Path>,
) -> Result<()>
```

Update all 5 call sites to pass `spec_dir.as_deref()`.

### Fix 6: Remove section banners (quick win)

Delete all 7 `// ====` banner blocks across 5 files:

- `mutations.rs:14-16` — "Shared Mutation Infrastructure"
- `split.rs:13-15` — "Spec Split"
- `archive.rs:11-13` — "Status Update"
- `queue.rs:13-15` — "Spec Next / Queue System"
- `queries.rs:13-15` — "Ready Queue"
- `queries.rs:210-212` — "Blocked View"
- `queries.rs:370-372` — "Spec List"

The file name IS the section name. Module-level `//!` doc comments
(already present in `mod.rs`) are sufficient.

### Fix 7: Extract `release_and_archive` helper

After Fix 1 lands, `complete_spec_value` and `split_spec_value` share
identical release+archive logic (~20 lines):

```rust
/// Release (version bump + archive) or archive-only for a completed spec.
pub(super) fn release_and_archive(
    id: &str,
    file_path: &str,
    frontmatter: &SpecFrontmatter,
    title: &str,
    major: bool,
) -> Result<()> {
    let tag_name = format!("spec/{}", id);
    if tag_exists(&tag_name)? {
        anyhow::bail!("Tag '{}' already exists.", tag_name);
    }
    let spec_dir = resolve_spec_dir(file_path);

    let strategy = ReleaseStrategy::from_project(Path::new("."));
    let bump = if major {
        Some(BumpType::Major)
    } else {
        BumpType::from_spec_type(&frontmatter.r#type)
    };

    if let Some(bump) = bump {
        let prepared = strategy.preflight(bump, file_path)?;
        let archive_dir = spec_dir.as_deref()
            .and_then(|d| d.to_str())
            .or(Some(file_path));
        prepared.execute(title, file_path, archive_dir)?;
        patina::git::create_tag_at(&tag_name, &format!("Archived spec: {}", title), "HEAD~1")?;
    } else {
        archive_spec_inner(id, file_path, "complete", title, spec_dir.as_deref())?;
    }

    Ok(())
}
```

Place in `mutations.rs` (where both callers live after Fix 1 converts
`split_spec_value` to use `mutate_spec`). Wait — `split_spec_value`
lives in `split.rs`. The helper goes in `archive.rs` (where
`archive_spec_inner` already lives) and both callers import it via
`super::archive::release_and_archive`.

**Note:** `split_spec_value` passes `major: false` always. The `major`
param only comes from `complete_spec_value`'s CLI flag.

## Implementation Order

```
Fix 6 (banners)           — independent, zero risk
Fix 5 (signature)         — independent, mechanical
Fix 1 (split uses mutate) — depends on nothing, enables Fix 7
Fix 7 (release helper)    — depends on Fix 1
Fix 2 (mutate_spec sig)   — independent, all callers update
Fix 3 (resume triple)     — depends on Fix 2
Fix 4 (typed results)     — independent, largest change
```

Suggested commit grouping:

```
1. fix: remove fossil section banners from spec internal/     [Fix 6]
2. fix: archive_spec_inner takes Option<&Path>                [Fix 5]
3. fix: split_spec_value uses mutate_spec                     [Fix 1]
4. refactor: extract release_and_archive helper               [Fix 7]
5. refactor: mutate_spec accepts FoundSpec, returns snapshot   [Fix 2+3]
6. refactor: typed result structs for _value() functions       [Fix 4]
```

## Exit Criteria

- [ ] No manual read-parse-mutate-write-DB outside `mutate_spec`
- [ ] No double `find_spec` calls in any `_value()` function
- [ ] `resume_spec_value` reads the spec file at most once
- [ ] All `_value()` functions return typed structs, not `serde_json::Value`
- [ ] `archive_spec_inner` takes `Option<&Path>`, not `&Option<PathBuf>`
- [ ] Zero `// ====` section banners in `internal/` files
- [ ] `complete_spec_value` and `split_spec_value` share release logic via helper
- [ ] All existing tests pass
- [ ] `cargo clippy` clean
- [ ] Pre-push checks green

## Key Files

```
src/commands/spec/internal/mutations.rs  — mutate_spec sig, result structs, release helper
src/commands/spec/internal/split.rs      — convert to mutate_spec
src/commands/spec/internal/archive.rs    — signature fix, release_and_archive helper
src/commands/spec/internal/queue.rs      — NextResult struct
src/commands/spec/internal/queries.rs    — banner removal
src/commands/spec/internal/mod.rs        — re-export new result types
src/commands/spec/mod.rs                 — may need re-export updates
src/mcp/spec_tools.rs                   — serialize typed results at MCP boundary
```

## Non-Goals

- Changing the public CLI behavior or output format
- Restructuring the module layout (that was spec-module-split)
- Adding new spec commands or features
- Changing `FoundSpec` to carry full frontmatter (Option A in Fix 3
  discussion — evaluate during implementation)

## Provenance

Identified by Gjengset-style code review after spec-module-split
completion. All items are pre-existing issues the split exposed by
making the code structure visible. Previous session (20260224-141429)
fixed 4 items; this spec covers the remaining 7.
