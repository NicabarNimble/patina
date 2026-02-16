---
type: fix
id: spec-visibility
status: draft
created: 2026-02-16
sessions:
  origin: 20260216-104145
related:
- layer/core/spec-driven-design.md
- layer/core/patina-identity.md
- layer/core/dependable-rust.md
beliefs:
- spec-driven-design
---

# fix: Spec Lifecycle Visibility

> Specs that exist on disk but haven't been scraped are invisible to `spec list`,
> `spec ready`, and `spec blocked`. Completed specs that were never archived sit
> silently in the tree. Both violate spec-driven-design: "SPECs are the single
> source of truth for all non-trivial action." A spec that's invisible isn't
> governing anything.

## Problem

Two bugs in spec lifecycle governance:

### 1. Hidden Specs (unscraped drafts invisible)

`patina spec list` reads only from the `patterns` table in patina.db
(`src/commands/spec/internal.rs:260-308`). Any spec created between scrapes is
invisible to `spec list`, `spec ready`, and `spec blocked`.

**Reproduction:** Create `layer/surface/build/feat/measurement-coverage/SPEC.md`
(status: draft). Run `patina spec list` — shows 3 specs. The new draft is missing
because `patina scrape` hasn't run.

**Root cause:** `get_all_specs()` treats the database as truth for spec existence.
But per [[patina-identity]] invariant #4: "layer/ is git-tracked knowledge,
.patina/ is derived state." The filesystem is truth. The database is derived.

### 2. Unarchived Completions (completed specs linger in tree)

`patina spec status <id> complete` auto-archives (git tag + git rm). But if a user
manually sets `status: complete` in frontmatter (or `--no-archive` is used), the spec
sits in the tree with no warning. `belief-truthfulness` currently has `status: complete`
in tree with no archive warning.

**Root cause:** `show_spec_list()` doesn't check for completed specs still on disk.
`archive_stale_specs()` exists but requires manual invocation with no prompt.

### 3. Draft Invisibility in Ready Queue

`patina spec ready` shows only `ready` and `active` specs. Draft specs are completely
hidden. A developer checking "what's next?" sees no drafts — they must know to run
`spec list` separately.

## What Exists Today

### Current Implementation (`src/commands/spec/internal.rs`)

| Function | What it does | Data source |
|----------|-------------|-------------|
| `get_all_specs()` (L260-308) | Query specs with filters | DB `patterns` table only |
| `get_ready_specs()` (L31-68) | Query ready/active specs | DB `patterns` + `spec_deps` |
| `get_blocked_specs()` (L137-201) | Query blocked specs | DB `patterns` + `spec_deps` |
| `show_spec_list()` (L311-343) | Display spec list | Calls `get_all_specs()` |
| `show_ready_specs()` (L71-109) | Display ready queue | Calls `get_ready_specs()` |
| `archive_stale_specs()` (L629-694) | Archive complete/abandoned | DB query + file existence |
| `find_spec()` (L697-728) | Find spec by ID | DB `patterns` table only |

### Existing Frontmatter Parser (`src/spec.rs`)

`parse_spec_file()` already parses YAML frontmatter into `SpecFrontmatter` struct
with `id`, `status`, `target`, `blocked_by`, `blocks`, and other fields. This is
the canonical parser — reuse it, don't write a new one.

`SpecFrontmatter` does NOT have a `title` field. Title comes from the first
`# heading` in the markdown body. The filesystem scan must extract this separately.

### Existing Disk Walk (`src/commands/scrape/layer/mod.rs`)

`collect_md_files()` walks layer/ respecting .gitignore via `ignore::WalkBuilder`.
But this walks ALL markdown files, not just specs. For specs, a targeted glob of
`layer/surface/build/**/SPEC.md` is more appropriate.

## Design

### Principle: Filesystem Is Truth, Database Is Supplementary

The fix merges two sources:
1. **Filesystem** — glob `layer/surface/build/**/SPEC.md`, parse frontmatter
2. **Database** — existing `patterns` table query (when DB exists)

Filesystem wins for existence. Database provides supplementary data (title, tags)
for already-scraped specs. When a spec is on disk but not in DB, show it with an
indicator so the user knows it hasn't been indexed yet.

### Changes

#### A. `get_all_specs()` — Merge Filesystem + Database

**Current:** Query `patterns` table. Bail if DB doesn't exist.

**New:**
1. Glob `layer/surface/build/**/SPEC.md` from project root
2. For each file: parse frontmatter with `parse_spec_file()`, extract title from
   `# heading` in body
3. Build a map of `id → SpecInfo` from filesystem
4. If DB exists: query `patterns` table (existing query), merge into map
   - DB entries for IDs already in map: supplement with DB data (title from DB
     may be richer if scrape extracted it)
   - DB entries for IDs NOT on disk: skip (stale DB entry, file was deleted)
5. Return merged list

**Key behavior changes:**
- No longer bails if DB doesn't exist — filesystem alone is sufficient
- Specs on disk but not in DB appear in output with `[unscraped]` suffix on status
- Specs in DB but not on disk are excluded (pruned — they'll be cleaned on next scrape)
- Filter parameters (`--status`, `--target`) apply to merged results

**SpecInfo change:** Add an `unscraped: bool` field (or embed the indicator in the
status display). This field does not affect JSON output structure beyond adding the
boolean.

#### B. `show_spec_list()` — Warn About Stale Completions

After displaying the spec table, check for any specs with `status` in
(`complete`, `abandoned`) that are still on disk. If found, print:

```
⚠ 1 completed spec still in tree — run `patina spec archive --stale` to archive
```

This is a read-path warning only. No automatic archiving.

#### C. `show_ready_specs()` — Add DRAFTS Section

**Current:** Shows READY and ACTIVE sections from DB query.

**New:** After READY and ACTIVE sections, add a DRAFTS section showing specs with
`status = "draft"`. These come from the same merged filesystem+DB source as
`get_all_specs()`.

```
READY (can start now):
  cross-project-beliefs        -          feat: Cross-Project Beliefs...

ACTIVE (in progress):
  git-tag-system               -          feat: Git Tag System...

DRAFTS (need promotion to ready):
  measurement-coverage         -          feat: Measurement Coverage System
  spec-visibility              -          fix: Spec Lifecycle Visibility [unscraped]
```

Draft specs are visible but clearly marked as not-ready-to-work.

#### D. `find_spec()` — Filesystem Fallback

**Current:** Queries DB only. Bails if spec not found in `patterns` table.

**New:** If DB lookup fails (spec not found), fall back to filesystem glob for
`layer/surface/build/**/SPEC.md`, parse each until `id` matches. This allows
`patina spec status <id> <status>` to work on unscraped specs.

### What Does NOT Change

- **Database schema** — no new tables, no new columns. This is a read-path fix.
- **`scrape` pipeline** — scrape continues to populate the `patterns` table as today.
  The fix makes the spec command work WITHOUT scrape, not instead of scrape.
- **`archive` logic** — archive_spec() and archive_stale_specs() unchanged.
- **`mod.rs` public interface** — all function signatures stay the same. The changes
  are entirely in `internal.rs`.
- **JSON output structure** — `--json` output adds `unscraped: bool` field, otherwise
  same shape.

### Implementation Notes

**Filesystem scan helper:** Create a private function in `internal.rs`:

```rust
/// Scan layer/surface/build/ for SPEC.md files and parse frontmatter.
/// Returns specs found on disk, keyed by id.
fn scan_disk_specs() -> Result<Vec<SpecInfo>> {
    // glob layer/surface/build/**/SPEC.md
    // for each: read file, parse_spec_file(), extract title from body
    // return SpecInfo { id, status, target, title, unscraped: true }
}
```

This uses `parse_spec_file()` from `src/spec.rs` (already a dependency). Title
extraction: find first line matching `^# (.+)$` in the body (same regex used in
`scrape/layer/mod.rs:295-299`).

**Glob approach:** Use `glob` crate or `std::fs` walk. Since we're only looking at
`layer/surface/build/**/SPEC.md` (a small, bounded directory tree), a simple
recursive `read_dir` is fine. No need for `ignore::WalkBuilder` — these files are
always git-tracked, never gitignored.

**Error handling:** If a SPEC.md file fails to parse, warn to stderr and skip it.
Don't fail the entire list because one file has malformed frontmatter.

## Verification

### Bug 1: Hidden Specs

```bash
# Create an unscraped spec (already exists: measurement-coverage)
patina spec list
# MUST show measurement-coverage with [unscraped] indicator
# MUST show all 4 specs (was showing 3)
```

### Bug 2: Unarchived Completions

```bash
patina spec list
# MUST show warning: "1 completed spec still in tree..."
# After running: patina spec archive --stale
# Warning disappears
```

### Bug 3: Draft Visibility

```bash
patina spec ready
# MUST show DRAFTS section with measurement-coverage and spec-visibility
```

### Regression Checks

```bash
# Existing behavior preserved
patina spec list --status active    # Shows only active specs
patina spec list --json             # Valid JSON with unscraped field
patina spec ready --json            # Valid JSON including drafts
patina spec blocked                 # Unchanged behavior
patina spec status <id> <status>    # Works on unscraped specs
```

### Edge Cases

- No DB exists (fresh project, never scraped): `spec list` shows filesystem specs
- No specs on disk: `spec list` shows "No specs found."
- Spec on disk with malformed frontmatter: warning to stderr, skip, continue
- Spec in DB but deleted from disk: excluded from results

## Exit Criteria

1. `patina spec list` shows ALL specs on disk, even if never scraped
2. `patina spec list` warns when completed/abandoned specs remain in tree
3. `patina spec ready` includes a DRAFTS section
4. `patina spec status` works on specs not yet in the database
5. No database schema changes
6. All existing tests pass
7. `SpecFrontmatter` from `src/spec.rs` is the only frontmatter parser used

## Risks

1. **Filesystem scan performance** — bounded: only `layer/surface/build/` subtree,
   typically < 20 files. Negligible cost.

2. **Frontmatter parse failures** — handled: warn and skip. User sees partial list
   rather than error.

3. **Race with scrape** — non-issue: if scrape runs concurrently, the DB and disk
   converge. Worst case: a spec shows [unscraped] briefly until scrape finishes.
