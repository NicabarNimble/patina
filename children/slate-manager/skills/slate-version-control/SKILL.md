---
name: slate-version-control
description: Slate child skill for git-aware work boundaries, checkpoint commits, and spec-parity archive/version behavior for Slate work.
---

# Slate Version Control Skill

Use this child skill when completing, checkpointing, archiving, recovering, or reasoning about version boundaries for Slate work.

## Current decision

For now, Slate version/archive behavior should match `patina spec` behavior as closely as possible. We may diverge later if Slate remains 1:1 with spec long enough to reveal a better shape.

## Spec-parity archive semantics

Target behavior for `archive-work`:

1. Work must be terminal: `complete` or `abandoned` unless forced.
2. Tracked working tree must be clean before archive.
3. Archive preserves the pre-removal artifact with a recovery tag:
   - specs use `spec/<id>`
   - Slate should use `slate/<id>`
4. Archive removes the durable work directory from the working tree:
   - `layer/slate/work/<id>/`
5. Archive creates a commit explaining recovery:
   - `docs: archive slate/<id> (<status>)`
   - include `Recover with: git show slate/<id>:layer/slate/work/<id>/work.toml`
6. Tag points to the commit before the archive-removal commit, matching spec archive behavior.

## Completion semantics

`complete-work` should remain a proof gate:

- status must be `active` unless forced
- proof plan must be fully checked
- closure evidence must exist
- belief harvest decision must be recorded

Completion does not necessarily create a release tag. Archive creates the durable recovery tag.

## Checkpoint guidance

For large/risky Slate changes:

- checkpoint before continuing when there are 100+ lines changed or 30+ minutes of work
- update Slate closure evidence with commit ids using `[[commit-SHA]]`
- record session context with `[[session-YYYYMMDD-HHMMSS]]`

## Current known gap

Native Slate currently tracks `work.toml` plus `layer/slate/events.jsonl`; the full spec-parity archive behavior is the target for this skill/work item and should be implemented by `slate-manager`.
