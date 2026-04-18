---
type: feat
id: interface-launch-picker-lazy-skillpack
status: draft
created: 2026-04-16
sessions:
  origin: 20260416-133521-394965000
related:
  - src/main.rs
  - src/commands/launch/internal.rs
  - src/commands/ai/surface.rs
  - src/commands/ai/mod.rs
  - src/interface/internal/surface.rs
  - src/interface/internal/bootstrap.rs
  - src/interface/internal/bundle.rs
  - src/mother/skills/mod.rs
beliefs:
  - "[[stale-context-is-hostile-context]]"
  - "[[core-verbs-standalone-mother-additive]]"
exit_criteria:
  - id: ils1-patina-picker-tty
    text: "Running `patina` with no subcommand in an existing Patina project and interactive TTY shows interface picker every time, with `(default)` marker." 
    checked: false
  - id: ils2-picker-default-resolution
    text: "Picker default resolves to project last-used interface (runtime/local state), with fallback to project `interfaces.default` from `.patina/config.toml` when no last-used value exists."
    checked: false
  - id: ils3-auto-init-selected-only
    text: "`Are you lost?` -> init -> interface selection path prepares only the selected interface bundle before launch (no all-interface prewarm)."
    checked: false
  - id: ils4-ai-direct-launch-lazy-ensure
    text: "`patina ai <interface>` remains direct launch and lazily ensures selected interface readiness (missing/stale bundle projected before launch)."
    checked: false
  - id: ils5-setup-semantics
    text: "`patina ai setup` semantics are explicit: selected/default interface setup + skillpack freshness check by default; all-interface prewarm requires explicit opt-in (`--all` or equivalent)."
    checked: false
  - id: ils6-session-envelope-stable
    text: "Session lifecycle behavior remains unchanged (`check_in`/artifact/tag/archive/update/end semantics unchanged except launch/setup entry routing)."
    checked: false
  - id: ils7-skillpack-freshness
    text: "Selected interface launch path checks Patina-managed bundle metadata/version and refreshes stale managed projections before session start; external tool version detection/logging remains independent."
    checked: false
  - id: ils8-voice-out-of-scope
    text: "No voice behavior changes are introduced by this spec (project voice binding and `--voice` handling remain untouched)."
    checked: false
  - id: ils9-proof
    text: "`cargo check --workspace -q` passes, targeted launch/setup tests cover picker default and selected-only setup path, and manual smoke runs verify `patina` + `patina ai <interface>` parity."
    checked: false
  - id: ils10-init-prompt-wording
    text: "Non-project init prompt wording is unambiguous: `Initialize this directory as a Patina project? [y/N]` where default/no exits and yes enters interface selection."
    checked: false
  - id: ils11-launch-observability-contract
    text: "Launch/setup emits stable, minimal events with correlation fields (`project_uid`, `interface`, `session_id` when created, bundle/tool versions, decision path) and avoids duplicate/noisy emissions."
    checked: false
  - id: ils12-old-project-self-heal
    text: "Revisiting old projects self-heals to new behavior on first launch (selected-interface path, stale bundle refresh), without requiring manual cleanup before normal use."
    checked: false
---
# feat: Interface launch picker + lazy skillpack ensure

## Problem

Current behavior is split:

1. `patina` in non-project mode supports a selection flow (`Are you lost?` + picker), but setup currently prewarms all interfaces during that init route.
2. `patina` in existing projects launches default directly without always presenting a picker.
3. Interface setup/install semantics are spread across init/setup/launch paths, which makes expected behavior hard to predict.

User expectation for this slice:
- `patina` is the interactive interface picker surface.
- picker choice launches exactly like `patina ai <interface>`.
- if selected interface is not prepared/current, Patina prepares it first, then launches.
- session envelope behavior is unchanged.

## Goal

Make launch behavior coherent with one model:

- `patina` (interactive): always picker in project context.
- `patina ai <interface>`: direct launch.
- Both paths share selected-interface lazy ensure logic (bundle + skillpack freshness), then run the existing session envelope pipeline.

## Non-Goals

- Voice model changes (explicitly out of scope).
- Broad Mother skill architecture rewrite (tracked in `cross-interface-mother-skills`).
- Ephemeral interface teardown/removal on session end.
- Changing session artifact schema/sections for this slice.

## Current State Snapshot

### Already true

- Native session envelope and artifacts are in place (`patina ai session ...`).
- `patina ai <interface>` launches through unified `ai::surface::launch` path.
- Per-interface prepare path exists (`prepare_ai_bundle`) and can lazily project selected interface.
- Project default interface is persisted in `.patina/config.toml`.

### Gaps

- Auto-init path currently calls `ai setup` full-surface projection.
- `patina` existing-project UX does not always force picker.
- No explicit first-class “selected-only setup + freshness” contract.

## Target Behavior

### Behavior matrix

| Context | Command | Behavior |
|---|---|---|
| Existing Patina project + TTY | `patina` | Show interface picker every run; preselect default = last-used interface for project, fallback to `.patina/config.toml` default |
| Existing Patina project + non-TTY | `patina` | No picker; launch resolved default directly |
| Existing Patina project | `patina --interface <x>` | Direct launch of `<x>` (no picker), same as `patina ai <x>` |
| Non-project directory + TTY | `patina` | Show `Are you lost?` banner and init prompt |
| Non-project directory + non-TTY | `patina` | Fail with actionable message (no interactive prompt) |
| Any Patina project | `patina ai <interface>` | Direct launch path; selected-interface lazy ensure + session envelope |

### `patina` (no subcommand)

- In interactive TTY and existing Patina project: show interface picker every run.
- Picker `(default)` marker resolves from Mother project state (`last_interface`) then project config fallback.
- Selected interface dispatches to the same launch backend used by `patina ai <interface>`.

### `patina ai <interface>`

- No picker.
- Ensure selected interface managed surface is present/current.
- If missing/stale: setup/refresh selected interface, then launch.
- Session check-in/envelope flow remains as-is.

### Init flow wording + behavior (`Are you lost?`)

- Banner can remain `Are you lost?`.
- Prompt text becomes explicit and non-conflicting: `Initialize this directory as a Patina project? [y/N]`.
- `N` (default, including empty input): exit launcher.
- `Y`: continue to HITL interface selection.
- After user selects interface: init + selected-interface setup only.
- No mandatory all-interface setup during init.

### State authority

- Mother is source of truth for project launch state (`last_interface`, timestamps, launch metadata).
- Project config (`.patina/config.toml`) is fallback/default policy, not recency state.
- Existing `.patina/local/interface-sessions/*.toml` remains session attachment pointer state.

## Implementation Order

1. Define launch decision contract (`patina` interactive picker vs direct command path), including TTY/non-TTY behavior.
2. Add Mother-backed project `last_interface` read/write and fallback chain.
3. Change init-selected route to selected-only setup + clarified prompt wording.
4. Formalize selected-interface setup/freshness check in launch path.
5. Add launch/setup observability events with stable schema and correlation IDs.
6. Add migration/self-heal behavior for old projects on first launch.
7. Add tests for picker default, selected-only setup, non-TTY behavior, and parity between `patina` and `patina ai <interface>` launch result.

## Verification

```bash
patina spec check interface-launch-picker-lazy-skillpack --json
cargo check --workspace -q
cargo test -q --workspace
```

Manual smoke:

```bash
# empty dir interactive route
patina

# direct route parity
patina ai pi
patina ai opencode
```

## Build Readiness

High for launch/setup routing slice, medium for skillpack freshness contract details.
Core launch/session plumbing already exists; this spec mostly aligns entrypoint behavior and setup scope.