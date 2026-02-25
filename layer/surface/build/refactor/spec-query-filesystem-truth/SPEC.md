---
type: refactor
id: spec-query-filesystem-truth
status: draft
created: 2026-02-25
sessions:
  origin: 20260225-104204
related:
- spec-scan-efficiency
exit_criteria: []
---
# refactor: Make spec queries use filesystem truth

> get_ready_specs and get_blocked_specs query DB directly while get_all_specs uses filesystem — dual-truth divergence risk

## Current State

The spec module has **two query architectures** running side by side:

1. **`get_all_specs()` (queries.rs:556)** — Filesystem-first: scans `layer/surface/build/` for SPEC.md files, parses frontmatter, then merges with DB to mark scraped/unscraped. Filesystem wins for existence and all fields.

2. **`get_ready_specs()` (queries.rs:128) and `get_blocked_specs()` (queries.rs:329)** — DB-only: queries the `patterns` table directly with SQL JOINs on `spec_deps`. If a spec exists on disk but hasn't been scraped, these functions **won't see it**.

This means:
- `patina spec list` shows unscraped specs (filesystem truth)
- `patina spec ready` silently omits them (DB truth)
- `patina spec blocked` silently omits them (DB truth)
- A freshly created spec that hasn't been scraped is invisible to `ready`/`blocked`

The DB can also contain stale entries for specs that have been archived — `get_all_specs` excludes these (no disk file), but `get_ready_specs`/`get_blocked_specs` would include them.

**Flagged by:** Rich Sutton (The Bitter Lesson — don't maintain two representations of the same state; the DB should be a read cache, not a mutable twin), Jon Gjengset (type/state drift — two code paths that should agree but can't be verified by the compiler), Andrew Ng (freshness — `unscraped: false` doesn't mean the DB content matches the current file).

## Target State

All spec query functions use filesystem as truth for spec existence and status. The DB is consulted only for derived data that doesn't exist in the filesystem (e.g., `spec_deps` dependency graph from the scraper).

Specifically:
- `get_ready_specs()` should start from `get_all_specs()` and filter, using `spec_deps` only for blocker resolution
- `get_blocked_specs()` should start from `get_all_specs()` and filter, using `spec_deps` for blocker details
- The `blocked_by` field in SPEC.md frontmatter should be the source of truth for dependencies, not `spec_deps` table alone

## Steps

1. Refactor `get_ready_specs()` to call `get_all_specs()` then filter by status IN (ready, active) with blocker check against filesystem-resolved blocked_by
2. Refactor `get_blocked_specs()` to call `get_all_specs()` then enrich with blocker status from filesystem
3. Keep `spec_deps` as supplementary — used by `load_dep_counts()` for impact scoring (this is a scraped signal, not truth)
4. Add test: create spec on disk without scraping → verify it appears in `ready`/`blocked` output
5. Verify `show_ready_specs` enhanced view still works with the new data source

## Key Files

```
src/commands/spec/internal/queries.rs   — refactor get_ready_specs, get_blocked_specs
src/commands/spec/internal/archive.rs   — load_spec already uses filesystem, no change
src/commands/spec/internal/queue.rs     — next_spec_value calls get_all_specs, may need update
```

## Exit Criteria

- [ ] `get_ready_specs()` uses filesystem truth (not DB-only query)
- [ ] `get_blocked_specs()` uses filesystem truth
- [ ] Unscraped specs appear in ready/blocked output when they qualify
- [ ] Archived specs don't appear in ready/blocked (no disk file = not included)
- [ ] `load_dep_counts()` still works for impact scoring (DB is supplementary, not removed)
