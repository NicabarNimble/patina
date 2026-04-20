---
type: refactor
id: session-new-command-flow
status: ready
created: 2026-04-20
sessions:
  origin: 20260419-160913-422415000
related:
- src/commands/ai/mod.rs
- src/commands/ai/internal.rs
- src/commands/session/internal.rs
- src/commands/mother/daemon/interface_control.rs
- src/session/internal/live.rs
- src/interface/runtime/templates.rs
- src/interface/runtime/claude/mod.rs
- src/interface/runtime/opencode/mod.rs
- src/interface/runtime/gemini/mod.rs
- resources/claude/session-start.md
- resources/opencode/session-start.md
- resources/gemini/session-start.toml
- resources/pi/session-start.md
- .claude/commands/session-start.md
- .opencode/commands/session-start.md
- .gemini/commands/session-start.toml
- .pi/prompts/session-start.md
exit_criteria:
- id: sncf1-command-surface-rename
  text: Session creation surface is renamed from `start` to `new` across CLI and HITL slash commands.
  checked: false
- id: sncf2-no-start-alias
  text: '`session-start`/`start` compatibility aliases are removed; only `new` is accepted.'
  checked: false
- id: sncf3-auto-flow-authoritative
  text: HITL launch remains attach-or-create authoritative; `new` is explicit boundary creation only.
  checked: false
- id: sncf4-first-update-title-hook
  text: First `/session-update` in an auto-created default-title session prompts naming from observed work and can persist the new title.
  checked: false
- id: sncf5-title-persistence
  text: Session rename writes both durable artifact frontmatter and Mother session record title.
  checked: false
- id: sncf6-git-tag-continuity
  text: Start/end tag behavior remains unchanged (`session-<file_id>-<interface>-start/end`) and is linked in session outputs.
  checked: false
---
# refactor: Rename /session-start to /session-new and align auto-session naming

> Replace `session-start` with `session-new` everywhere, remove the old alias/surface entirely, keep launch auto-session behavior unchanged, and add first-update naming assistance for auto-created default-title sessions.

## Problem

Auto HITL launch already resolves session identity (`attach-or-create`). The legacy `/session-start` naming suggests initialization but actually creates a second session boundary. This causes operator confusion and can leave multiple active same-interface sessions.

## Goal

1. Rename explicit boundary creation to `session-new`.
2. Remove `session-start` command/alias/surface entirely.
3. Keep launch auto-session (`patina ai <interface>`) as canonical attach-or-create behavior.
4. Add first-update naming hook so auto-created sessions get contextual titles from actual work.
5. Preserve existing git tag identity model and range usability.

## Status

Draft complete and implementation-ready.

## Non-Goals

- Changing attach-or-create envelope resolution semantics.
- Changing session tag naming scheme.
- Introducing a long migration period with dual command names.
- Redesigning note/end behavior.

## Current State

- HITL launch creates or attaches session via Mother envelope resolution.
- `/session-start` wrappers call `patina ai session start ...` and always create a new session.
- Session titles are often default (`<interface> session`) when auto-created without `--title`.
- No dedicated first-update naming workflow exists.

## Target State

- Explicit new boundary command is `session-new` / `patina ai session new`.
- `session-start` no longer exists in CLI, wrappers, prompts, templates, or docs.
- First update can propose a better title from real work context and persist on confirmation.
- Git tags remain stable and discoverable across start..end range.

## Solution

1. **Command surface rename**
   - Replace `AiSessionCommands::Start` with `AiSessionCommands::New`.
   - Replace wrapper/script/prompt names and references from `session-start` to `session-new`.
   - Remove old command entries from runtime help/context and template generation.

2. **No alias policy**
   - Do not keep parser aliases for `start`.
   - Do not ship `session-start` compatibility scripts.
   - Make failures actionable (suggest `session-new`).

3. **First-update naming hook**
   - On first update for sessions with default auto title (`<interface> session`), prompt HITL agent to suggest a title based on update context (changed files, goals, decisions).
   - On confirmation, persist title rename to:
     - session artifact frontmatter/body heading;
     - Mother session record title.

4. **Git tag continuity**
   - Keep start/end tag derivation unchanged.
   - Include tag range references in update/end artifacts and UX copy.

## Implementation Order

1. Rename CLI subcommand `start` → `new` in `patina ai session`.
2. Rename wrappers/prompts/resources/templates to `session-new` across Claude/OpenCode/Gemini/PI.
3. Remove all `session-start` references in runtime command listings and generated context text.
4. Add first-update naming hook and title persistence path.
5. Add/adjust tests for command parsing, generated files, and title persistence.

## Resolved Decisions

- `session-start` is removed now; there is no alias.
- Auto attach-or-create remains the default entrypoint.
- `session-new` is explicit boundary creation only.
- Title refinement happens from first update context, not from launch defaults alone.
- Git tags stay identity-based (`file_id` + `interface`) and are not title-derived.

## Verification

```bash
cargo check --workspace -q
cargo test --workspace -q ai
cargo test --workspace -q session
cargo test --workspace -q interface
patina ai session --help
patina ai session new --help
patina ai session start --help   # must fail
```

Behavior checks:

- `patina ai <interface>` with no active session creates one (default title if none supplied).
- `/session-new` creates a second explicit boundary session.
- `/session-start` is unavailable.
- First `/session-update` on default-title auto session proposes rename and persists on confirm.
- Session list/show reflect renamed title.
- Start/end tags remain unchanged and range log works.

## Exit Criteria

See frontmatter `exit_criteria` checklist.

## Build Readiness

High. Change is mostly command-surface refactor plus targeted title-persistence hook.