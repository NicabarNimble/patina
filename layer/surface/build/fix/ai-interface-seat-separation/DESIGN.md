# Design: AI Interface Seat Separation

## Context

The first `patina ai` build correctly introduced:

- Mother-backed live sessions
- interface check-in
- new native adapter contracts

But it left tmux seat identity on the old project-only naming model.

That creates a split-brain risk:

- session check-in is interface-aware
- tmux attach is interface-blind

If Claude already owns the project tmux seat, `patina ai opencode` can
correctly create or attach an OpenCode live session and then land the
operator in Claude anyway.

This patch fixes that mismatch without destabilizing the trusted legacy
launcher path.

## Design Goals

- Preserve the "reattach to what is already running" feel
- Prevent cross-interface seat theft
- Keep the new multi-session model truthful
- Preserve legacy `patina` launcher behavior
- Make future web/headless interfaces possible without terminal-shaped
  leakage

## Non-Goals

- redesign the entire launcher UX
- solve persona federation here
- move legacy `patina` onto the new lane model
- introduce a complex session browser UI

## Current Code Seams

- `src/interface/internal/checkin.rs`
  This already scopes live session reuse by `adapter_name` and
  `interface_kind`.
- `src/interface/internal/tmux.rs`
  This currently derives a single tmux session name per project.
- `src/commands/ai/internal.rs`
  This wires check-in and launch together and is where selection policy
  should stay visible.
- `src/session/internal/live.rs`
  This contains project-wide active session queries and still exposes
  "first active session" style fallbacks that should not control the AI
  path.

## Proposed Design

### 1. Introduce interface-scoped tmux lane naming

Add a new helper for the AI path, for example:

- `derive_interface_session_name(project_path, adapter_name)`

This should produce stable names that differ across interfaces while
remaining deterministic per project, for example:

- `patina_<projecthash>_opencode`
- `patina_<projecthash>_gemini`

The existing `derive_session_name(project_path)` can remain as the
legacy compatibility helper.

### 2. Keep lane naming explicit in `patina ai`

`patina ai` should request launch with an interface-scoped tmux lane
name. Do not hide this inside shared legacy code that may later be used
by the no-subcommand launcher.

This keeps the migration boundary obvious.

### 3. Tighten interface check-in selection

The AI path should use these rules:

1. If `--session` is provided, attach only to that exact session.
2. Otherwise find active sessions for the same interface lane.
3. If exactly one exists, reuse it.
4. If more than one exists, require explicit selection.
5. If none exist, create a new session.

Other-interface sessions may be listed as context, but they must not be
selected automatically.

The first patch can implement "require selection" as:

- a TTY prompt, or
- a clean error telling the user to choose with `patina ai list` and
  `patina ai --session <id>`

Either is acceptable. Silent arbitrary reuse is not.

### 4. Preserve compatibility path

The no-subcommand `patina` launcher should keep using the current
project-scoped tmux naming behavior unless another spec changes that on
purpose.

That means this patch should not replace the shared legacy helper. It
should add a new AI-path-specific one.

## File Targets

- `src/interface/internal/tmux.rs`
  Add interface-scoped lane naming for `patina ai`.
- `src/interface/internal/checkin.rs`
  Tighten reuse/selection behavior for active same-interface sessions.
- `src/commands/ai/internal.rs`
  Use interface-scoped tmux lane naming and surface truthful selection
  behavior.
- `src/session/internal/live.rs`
  Add helpers for listing active sessions by interface lane if needed.
- tests near the above modules

## Verification

- unit test that legacy and AI tmux naming are distinct
- unit test that OpenCode and Gemini produce different lane names for
  the same project
- unit test that same-interface relaunch reuses the same lane name
- unit test that multiple same-interface active sessions do not
  auto-select arbitrarily
- command-level verification that rerunning `patina ai opencode`
  reattaches only to the OpenCode lane

## Relation To Existing Specs

This is a patch against:

- `patina-ai-interface-layer`
- `session-narrative-system`
- `agentic-surface-architecture`

It does not replace them. It closes a concrete integration bug between
their first implementation and the behavior users expect from the tmux
launcher model.
