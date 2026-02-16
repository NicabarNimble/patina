---
type: fix
id: spec-visibility
status: ready
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

**Why only `layer/surface/build/`?** This is the single canonical path for specs.
The directory structure encodes spec type: `build/{feat,fix,refactor,explore}/<id>/`.
No other layer paths contain specs — `layer/core/` holds patterns, `layer/surface/
epistemic/` holds beliefs, `layer/sessions/` holds sessions. The existing DB query
(`WHERE file_path LIKE 'layer/surface/build/%'`) confirms this is already the
contract. If a future layer variant introduces specs elsewhere, this scan path is
the one place to update — and that change would be a spec of its own.
4. If DB exists: query `patterns` table (existing query), merge into map
   - DB entries for IDs already in map: filesystem wins for all fields. DB data
     is ignored for these entries — both sources derive title from the same
     `# heading` in the markdown body, so there's no meaningful divergence. If
     the filesystem title is empty (malformed file), fall back to `id` as title
     (same as scrape does today in `scrape/layer/mod.rs:298`). Never fall back
     to DB title — that creates flip-flopping when scrape timing varies.
   - DB entries for IDs NOT on disk: skip (stale DB entry, file was deleted)
5. Return merged list

**Key behavior changes:**
- No longer bails if DB doesn't exist — filesystem alone is sufficient
- Specs on disk but not in DB appear in output with `[unscraped]` suffix on status
- Specs in DB but not on disk are excluded (pruned — they'll be cleaned on next scrape)
- Filter parameters (`--status`, `--target`) apply to merged results

**SpecInfo change:** Add `unscraped: bool` field to the `SpecInfo` struct. This is
the single source of truth for the unscraped state — all output paths read it:

- **CLI human output:** Appends `[unscraped]` suffix to the status column.
  Example: `draft [unscraped]` instead of `draft`.
- **JSON output (`--json`):** Includes `"unscraped": true` (or `false`) as a
  top-level field on each spec object. Consumers filter or display as needed.
- **Internal callers** (e.g., `show_ready_specs`, `find_spec`): Check `unscraped`
  to decide behavior (e.g., `find_spec` returns file_path from disk scan when
  `unscraped` is true).

One struct field, consistent everywhere. No ambiguity for downstream consumers.

#### B. `show_spec_list()` — Warn About Stale Completions

After displaying the spec table, check for any specs with `status` in
(`complete`, `abandoned`) that are still on disk. If found, print:

```
⚠ 1 completed spec still in tree — run `patina spec archive --stale` to archive
```

**Warning scope and behavior:**
- **Where it appears:** `show_spec_list()` only — the one command that shows the
  full inventory. It does NOT appear in `show_ready_specs()`, `show_blocked_specs()`,
  or `find_spec()`. Those commands have focused jobs; cluttering them with archive
  warnings violates the single-purpose principle.
- **JSON output:** The warning is not embedded in JSON. Instead, the `status` field
  on each spec already conveys `complete`/`abandoned` — JSON consumers can detect
  stale completions programmatically. The warning is a human-output convenience.
- **No suppression flag.** The `--no-archive` flag on `spec status` is the explicit
  opt-in for keeping a completed spec in tree. The warning is the cost of that
  choice — a gentle nudge, not a blocker. If someone uses `--no-archive`, they're
  choosing to accept this one-line reminder on `spec list`. The warning has zero
  impact on exit code or machine-readable output.
- **Intentional in-tree specs:** If a project wants to keep completed specs for
  historical context, the warning will persist. This is acceptable — it's a single
  line, not a wall of alerts. The alternative (a per-spec suppression mechanism)
  is over-engineering for a case that hasn't been observed in practice. If it
  becomes a real pattern, a future `keep_in_tree: true` frontmatter field is a
  trivial addition.

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

**DRAFTS section behavior:**
- **Human output only.** `patina spec ready --json` returns only `ready` and
  `active` specs — the JSON contract is "actionable now" and agents consuming
  this should not see drafts mixed in. A human reading terminal output benefits
  from seeing what's in the pipeline; an agent parsing JSON needs a clean
  work queue.
- **No `--no-drafts` flag.** The section is visually separated with a clear
  "need promotion to ready" label. Adding a suppression flag for a 2-line
  informational section is over-engineering. If users report confusion, a flag
  is a trivial follow-up — but the bet is that seeing drafts reduces confusion
  (fewer "where did my spec go?" moments) rather than adding it.
- **Draft count in summary.** When drafts exist, the "No specs ready" empty-state
  message changes to: "No specs ready to work on. 2 draft spec(s) — promote
  with `patina spec status <id> ready`"

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
- **JSON output structure** — `spec list --json` adds `"unscraped": bool` field per
  spec object, otherwise same shape. `spec ready --json` is unchanged — returns
  only ready/active specs (no drafts in JSON, see section C).

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

**Glob approach:** Use `std::fs` recursive walk (no new dependency). The scan
covers `layer/surface/build/**/SPEC.md` — a small, bounded tree (typically < 20
files). No need for `ignore::WalkBuilder` since specs are always git-tracked.

**Performance:** No caching. The scan reads < 20 small markdown files on every
invocation. At ~5KB average per spec file, that's < 100KB of I/O — sub-millisecond
on any modern filesystem. The DB query it replaces was also uncached. If the spec
tree ever grows to hundreds of files, caching becomes a separate concern — but
that growth would itself signal a process problem (too many open specs).

**Error handling — explicit policy:**
- **File unreadable** (permissions, broken symlink): `eprintln!` warning with path
  and error, skip file, continue scan. The listing shows all parseable specs.
- **Frontmatter parse failure** (malformed YAML, missing `---` delimiters):
  `eprintln!` warning with path and parse error, skip file, continue scan.
- **Missing `id` field:** Skip with warning. A spec without an id cannot
  participate in the governance system.
- **Missing `status` field:** Include in results with `status: None` (same as
  `SpecInfo` already supports). Displayed as `-` in status column.
- **`scan_disk_specs()` itself fails** (e.g., `layer/surface/build/` doesn't exist):
  Return empty vec, not an error. A project with no build directory has no specs
  — that's a valid state, not a failure.

In all cases: the listing never aborts due to a single bad file. Partial results
are better than no results.

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
patina spec list --json             # Valid JSON; each spec has "unscraped" field
patina spec ready --json            # Valid JSON; ready/active only, NO drafts
patina spec ready                   # Human output includes DRAFTS section
patina spec blocked                 # Unchanged behavior
patina spec status <id> <status>    # Works on unscraped specs (filesystem fallback)
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
