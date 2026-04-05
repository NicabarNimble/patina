---
type: fix
id: spec-archive-db-path
status: draft
created: 2026-04-05
related:
  - src/commands/spec/internal/archive.rs
  - src/commands/spec/internal/mutations.rs
exit_criteria:
  - id: sad1-mutation-reads-mother
    text: "Spec mutation commands (complete, archive, promote, pause, resume, block, reopen) resolve spec status from the same data source as read commands (list, show, check)."
    checked: false

  - id: sad2-archive-round-trip
    text: "`patina spec archive <id>` succeeds for a spec with status complete — creates git tag, removes spec directory, commits."
    checked: false

  - id: sad3-complete-round-trip
    text: "`patina spec complete <id>` succeeds for a spec with status active — sets status to complete in SPEC.md frontmatter."
    checked: false
---
# fix: spec-archive-db-path

## Problem

`patina spec archive` and other mutation commands (`complete`, `promote`, etc.)
fail with "has no status" or "has status 'none'" even when the spec file has a
valid status and Mother's read commands (`list`, `show`, `check`) correctly
report it.

## Root Cause

`find_spec()` in `src/commands/spec/internal/archive.rs` queries a local
`patterns` table in a project-scoped DB. Read commands go through Mother's
spec-manager WASM child, which reads spec frontmatter from the indexed state.
The local DB either doesn't have a `patterns` table or has stale/empty data,
causing the status lookup to return `None`.

The mutation and read paths use different data sources for the same spec state.

## Fix

`find_spec()` should resolve spec status from the same source as the read path:
either read the SPEC.md frontmatter directly, or query Mother's indexed state.
The simplest fix is to parse the SPEC.md file on disk since `find_spec` already
has the `file_path` — just read the frontmatter status from the file instead of
relying on the stale DB column.

## Exit Criteria

See frontmatter above.
