---
type: fix
id: ai-interface-seat-separation
status: complete
created: 2026-03-11
sessions:
  origin: 20260311-112321-EF79
related:
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/feat/patina-ai-interface-layer/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/feat/session-narrative-system/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/agentic-surface-architecture/SPEC.md
exit_criteria:
  - id: same-interface-reattach
    text: '`patina ai` reattaches only to the same interface lane in the same project; OpenCode, Gemini, and Claude do not steal each other''s tmux seats'
    checked: true
  - id: session-seat-separation
    text: 'Mother-backed session attachment and tmux seat attachment are kept distinct in code and tests, so an interface-scoped session cannot silently attach to another interface''s tmux seat'
    checked: true
  - id: truthful-selection
    text: 'When multiple active sessions exist in a project, `patina ai` uses truthful selection or same-interface reuse rather than arbitrary first-session fallback'
    checked: true
  - id: compatibility-preserved
    text: 'The trusted `patina` compatibility launcher path keeps its existing tmux behavior unless explicitly opted into the new lane model'
    checked: true
  - id: tests-cover-collision
    text: 'Tests cover interface-scoped tmux naming, cross-interface non-attachment, and active-session selection behavior'
    checked: true
---
# fix: AI Interface Seat Separation — Interface-Scoped tmux and Safe Session Attach

> Prevent patina ai from attaching to another interface's tmux seat or session lane. Separate tmux seat identity by interface, preserve same-interface reattach behavior, and make active session selection truthful in multi-session projects.

## Problem

The new `patina ai` path introduced a real split between:

- Mother-backed live session attachment
- tmux seat attachment

But the first implementation only scoped live session reuse by
interface. tmux seat naming remained project-scoped. In practice this
means:

- `patina ai opencode` can create or attach the correct OpenCode live
  session in Mother
- then attach the terminal to a Claude tmux seat if Claude is already
  active for the project

That breaks the new session model, confuses the operator, and risks
corrupting session narrative expectations.

## Root Cause

The current implementation uses two different identity models:

- live session reuse in `src/interface/internal/checkin.rs` is scoped by
  `adapter_name` and `interface_kind`
- tmux seat naming in `src/interface/internal/tmux.rs` is derived only
  from project path

So the new path has interface-aware runtime attachment but
interface-blind transport attachment.

There is also a second drift risk: some selection flows in the broader
session system still fall back to "first active session in project,"
which is not truthful enough once many active sessions exist.

## Fix

Patch the `patina ai` path so session reuse and tmux seat reuse follow
the same interface lane model.

### 1. Make tmux seat identity interface-scoped for `patina ai`

Derive `patina ai` tmux session names from:

- project identity
- interface lane (`opencode`, `gemini`, future `claude` if added)

This preserves the good existing behavior:

- rerunning the same interface in the same project reattaches
- the prior client is displaced cleanly
- hopping machines still works

But it prevents cross-interface seat collisions.

### 2. Keep session identity and tmux seat identity separate

Do not collapse runtime sessions into tmux seats.

- session = narrative/runtime object
- tmux seat = interface transport lane

The code should model both explicitly so future web/headless interfaces
can reuse the same session substrate without inheriting terminal-only
assumptions.

### 3. Tighten active session selection

When `patina ai` starts in a project:

- if an active same-interface session exists, reuse it
- if multiple active same-interface sessions exist, require selection
  rather than arbitrary reuse
- if only other-interface sessions exist, do not auto-attach their seat
  or silently reuse them
- if none exist, create a new session

The first implementation may use a simple textual selector/prompt for
TTY mode and explicit `--session` for noninteractive use. The critical
requirement is truthful behavior, not fancy UX.

### 4. Preserve compatibility path behavior

Do not change the legacy no-subcommand `patina` launcher semantics as
part of this fix except for clearly shared bug fixes. The new interface
lane model belongs to `patina ai`.

### 5. Test the split directly

Add focused tests proving:

- tmux seat naming differs by interface for the same project
- live session reuse is same-interface only
- selection logic does not fall back to arbitrary first active session
- legacy launcher naming/behavior stays unchanged

## Exit Criteria

1. `patina ai` reattaches only to the same interface lane in the same
   project; OpenCode, Gemini, and Claude do not steal each other's tmux
   seats.
2. Mother-backed session attachment and tmux seat attachment are kept
   distinct in code and tests, so an interface-scoped session cannot
   silently attach to another interface's tmux seat.
3. When multiple active sessions exist in a project, `patina ai` uses
   truthful selection or same-interface reuse rather than arbitrary
   first-session fallback.
4. The trusted `patina` compatibility launcher path keeps its existing
   tmux behavior unless explicitly opted into the new lane model.
5. Tests cover interface-scoped tmux naming, cross-interface
   non-attachment, and active-session selection behavior.
