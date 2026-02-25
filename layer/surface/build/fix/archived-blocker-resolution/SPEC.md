---
type: fix
id: archived-blocker-resolution
status: draft
created: 2026-02-25
sessions:
  origin: 20260225-093943
related:
- spec-precompletion-gate
beliefs:
- eventlog-is-truth
exit_criteria:
- id: tag-fallback
  text: "find_spec returns status=complete for archived specs via git tag lookup"
  checked: false
- id: resume-unblocks
  text: "resume_spec_value resolves archived blockers as complete"
  checked: false
- id: backward-compat
  text: "Existing specs without git tags still resolve via DB and filesystem"
  checked: false
---
# fix: Resolve blockers from archived specs via git tags

> find_spec fails for archived specs — completed blockers become phantom blockers after scrape rebuild

## Problem

When a spec is completed and archived, `archive_spec_inner` removes the
SPEC.md file via `git rm` and preserves it as a git tag (`spec/<id>`).
The DB row retains `status=complete` temporarily, but after a `patina scrape`
rebuild the row disappears (no file to scrape).

`resume_spec_value()` checks blockers via `find_spec()`, which tries DB
first then filesystem. For an archived spec with a stale or missing DB row:

```rust
Err(_) => Some(format!("{} (not found)", blocker_id)),
```

A completed blocker becomes a phantom blocker — permanently blocking the
dependent spec with "(not found)" even though the work is done and tagged.

Discovered during spec-precompletion-gate: `spec-structured-exit-criteria`
was completed and archived, `spec_show` failed on it.

## Root Cause

`find_spec()` has two resolution paths (DB, filesystem) and no git-tag
fallback. Archived specs exist only as annotated git tags, which neither
path checks.

## Fix

Add a git-tag fallback to `find_spec()`:

1. After DB and filesystem both miss, check if `spec/<id>` tag exists
2. If it does, return `FoundSpec { status: Some("complete"), ... }`
   (archived specs are always complete or abandoned — check tag message)
3. `file_path` can reference the tag path: `spec/<id>:SPEC.md`

~10 lines in `archive.rs`. No new dependencies, no structural changes.

## Key Files

```
src/commands/spec/internal/archive.rs  — find_spec() add git-tag fallback
```

## Exit Criteria

- [ ] `find_spec` returns `status=complete` for archived specs via git tag
- [ ] `resume_spec_value` resolves archived blockers as complete
- [ ] Existing specs without git tags still resolve via DB and filesystem
