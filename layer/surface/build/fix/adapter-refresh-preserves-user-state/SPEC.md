---
type: fix
id: adapter-refresh-preserves-user-state
status: active
created: 2026-02-24
sessions:
  origin: 20260224-053924
related:
- src/commands/adapter.rs
- src/adapters/templates.rs
beliefs:
- safety-boundaries
- plugins-are-three-prong-bundles
---

# fix: Adapter Refresh Preserves User State

> `patina adapter refresh claude` silently drops `settings.local.json`
> and doesn't know about template-managed commands added after the
> allowlist was written. Both bugs surfaced during spec-workflow-rigor
> completion when two refreshes were needed to deploy `/spec`.

## Problem

### Bug 1: settings.local.json dropped on refresh

`preserve_user_files()` in `src/commands/adapter.rs:424` preserves three
categories during refresh:

1. `context/` files (session state)
2. Custom `commands/` (non-template)
3. Custom `skills/` (non-template)

It does **not** preserve root-level files like `settings.local.json`.
When refresh removes and recreates `.claude/`, any root-level user files
are lost. The backup exists in `.patina/local/backups/` but the user
doesn't know to look there.

`settings.local.json` contains the accumulated Claude Code permission
allowlist — losing it forces the user to re-approve every tool call.

### Bug 2: TEMPLATE_COMMANDS stale after adding /spec

`TEMPLATE_COMMANDS` at `adapter.rs:413` lists the template-managed
command files that should **not** be preserved as custom commands:

```rust
const TEMPLATE_COMMANDS: &[&str] = &[
    "session-start.md",
    "session-update.md",
    "session-note.md",
    "session-end.md",
    "patina-review.md",
];
```

`spec.md` was added to the template system (both `templates.rs` and
`session_scripts.rs`) but not to `TEMPLATE_COMMANDS`. On refresh, the
old `spec.md` would be "preserved" as a custom command and then
immediately overwritten by the template copy. Not harmful — just
wasteful work and confusing if someone debugs the refresh flow.

This will recur every time a new template-managed command is added.

## Solution

### Fix 1: Preserve root-level user files

Add a fourth preservation category to `preserve_user_files()`:

```rust
// Preserve root-level user files (settings, config)
const USER_ROOT_FILES: &[&str] = &["settings.local.json"];

for filename in USER_ROOT_FILES {
    let path = adapter_dir.join(filename);
    if path.exists() {
        let content = std::fs::read(&path)?;
        preserved.push((filename.to_string(), content));
    }
}
```

Use an explicit allowlist (`USER_ROOT_FILES`) rather than preserving
all root files — we don't want to accidentally preserve stale template
artifacts. New user-created root files need to be added to the list.

### Fix 2: Add spec.md to TEMPLATE_COMMANDS

```rust
const TEMPLATE_COMMANDS: &[&str] = &[
    "session-start.md",
    "session-update.md",
    "session-note.md",
    "session-end.md",
    "patina-review.md",
    "spec.md",
];
```

### Fix 3 (optional): Print what was preserved

Add visibility to the refresh output so the user can verify:

```
📁 Restoring user files...
  ✓ Restored settings.local.json
  ✓ Restored context/active-session.md
  ✓ Restored context/last-session.md
  ✓ Restored 3 user files
```

Currently it only prints the count, not the filenames.

## Key Files

```
src/commands/adapter.rs          — preserve_user_files(), TEMPLATE_COMMANDS
src/adapters/templates.rs        — template installation (for reference)
```

## Exit Criteria

- [ ] `settings.local.json` survives `patina adapter refresh claude`
- [ ] `TEMPLATE_COMMANDS` includes `spec.md`
- [ ] Refresh output lists preserved filenames (not just count)
- [ ] Test: create a dummy root file, refresh, verify it's gone (not in allowlist)
- [ ] Test: create settings.local.json, refresh, verify it survives

## Non-Goals

- Generic "preserve everything" — explicit allowlists are safer
- Preserving files outside `.{adapter}/` — out of scope
- Migrating settings.local.json to a non-adapter location — separate concern

## Provenance

Discovered during spec-workflow-rigor Phase 7 (session 20260224-053924).
Two `adapter refresh` calls were needed to deploy `/spec` command.
Second refresh dropped `settings.local.json`. Manually restored from
`.patina/local/backups/claude-20260224-054711/settings.local.json`.
