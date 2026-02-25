---
type: fix
id: spec-complete-atomicity
status: active
created: 2026-02-25
sessions:
  origin: 20260224-202650
---
# fix: Spec complete atomicity gap

> complete_spec_value updates DB status to 'complete' before release+archive operations, leaving inconsistent state on failure

## Problem

`patina spec complete <id>` can leave the database in an inconsistent
state. During spec-create's own completion (session 20260224-202650),
`complete_spec_value` updated the DB status to "complete" via
`mutate_spec()`, then failed on `release_and_archive()` because of a
dirty working tree (uncommitted Cargo.lock). Result: DB said "complete"
but no version bump, no archive tag, spec files still on disk. Required
manual `sqlite3` fix to recover.

Same gap exists in `abandon_spec_value` — `mutate_spec()` runs before
`archive_spec_inner()`.

`pause_spec` and `block_spec` already handle this correctly with
`with_content_rollback()`, but that only rolls back the YAML file, not
the DB status update inside `mutate_spec()`.

## Root Cause

`mutate_spec()` writes both the YAML file and the DB status in one
call (`mutations.rs:73-103`). Callers that do post-mutation operations
(git tag, release, archive) have no way to roll back the DB if those
operations fail.

The operation order in `complete_spec_value`:
1. `mutate_spec()` — writes file + updates DB to "complete"
2. `release_and_archive()` — version bump, git tag, git rm, commit

If step 2 fails, step 1 is already committed to the DB.

## Fix

Apply the `with_content_rollback` pattern to `complete_spec_value` and
`abandon_spec_value`, extended to also roll back the DB status:

1. Save the pre-mutation status before calling `mutate_spec()`
2. Wrap the post-mutation operations in a rollback guard
3. On failure: restore YAML file content AND reset DB status to
   the pre-mutation value

Alternatively, defer the DB update out of `mutate_spec()` and into
callers, so it only happens after all operations succeed. This is a
larger change but eliminates the problem structurally.

## Key Files

```
src/commands/spec/internal/mutations.rs  — mutate_spec(), with_content_rollback()
```

## Exit Criteria

- [ ] `complete_spec_value` rolls back DB status on `release_and_archive` failure
- [ ] `abandon_spec_value` rolls back DB status on `archive_spec_inner` failure
- [ ] Simulated failure (e.g., dirty tree) leaves DB status unchanged
- [ ] Existing `pause_spec`/`block_spec` rollback behavior unaffected
