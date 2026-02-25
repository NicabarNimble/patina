---
type: feat
id: spec-history
status: ready
created: 2026-02-25
sessions:
  origin: 20260225-104204
related:
- spec-system-audit-2026-02
exit_criteria: []
---
# feat: Add spec history command for lifecycle audit

> No way to view a spec's lifecycle events from CLI — tags exist but aren't surfaced

## Problem

The spec system creates rich lifecycle events via git tags (`spec/{id}-start`, `spec/{id}-paused-1`, `spec/{id}-resumed-1`, `spec/{id}-blocked-1`, `spec/{id}-v1-complete`) but provides **no command to view them**. Users must know the tag naming convention and run manual `git tag -l "spec/{id}*"` + `git log` commands.

For a system where specs are the primary coordination mechanism between human and AI agent, the lifecycle should be auditable through the tool itself.

**Flagged by:** Andrew Ng (no measurement of spec effectiveness — how long in each state? what's the cycle time?), Steve Yegge (platform gap — the data exists but isn't surfaced through the CLI; `show` dumps the full body, `list` shows current status, but nothing shows the journey).

## Solution

Add `patina spec history <id>` command that reconstructs lifecycle from git tags:

1. List all tags matching `spec/{id}*` pattern
2. For each tag, extract: timestamp (from annotated tag), event type (from tag name pattern), message (from tag annotation)
3. Display as chronological timeline
4. Include time-in-state calculations (days between events)

Example output:
```
History: my-feature

  2026-01-15  draft      spec: draft my-feature
  2026-01-17  ready      spec: promote my-feature to ready
  2026-01-18  active     spec: promote my-feature to active (start)
  2026-01-20  paused     spec: pause my-feature — waiting on API design  [2d active]
  2026-01-25  active     spec: resume my-feature  [5d paused]
  2026-01-28  complete   release: v0.28.0 — My Feature  [3d active, 13d total]
```

Also expose as `history_spec_value(id) -> Result<Vec<LifecycleEvent>>` for MCP with `--json`.

## Key Files

```
src/commands/spec/internal/queries.rs  — new history_spec_value() + display
src/commands/spec/mod.rs               — new History subcommand + re-export
src/mcp/server.rs                      — new spec.history tool handler
```

## Exit Criteria

- [ ] `patina spec history <id>` shows chronological lifecycle from tags
- [ ] Each event shows timestamp, state, message, and time-in-state
- [ ] `--json` output available for MCP
- [ ] Works for archived specs (tags still exist after archive)
- [ ] Graceful output when spec has no tags (newly created drafts)

## Non-Goals

- Aggregate analytics across all specs (that's a separate reporting concern)
- Modifying how tags are created (existing convention is fine)
- Diffstat between events (resume already shows this inline)
