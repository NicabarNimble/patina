---
type: fix
id: session-end-cross-session-update
status: complete
created: 2026-03-12
related:
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/mother-owned-interface-bundles/SPEC.md
beliefs:
  - spec-driven-design
  - dependable-rust
  - unix-philosophy
  - safety-boundaries
  - skills-for-structured-output
exit_criteria:
  - id: end-updates-requested-session
    text: '`/session-end` updates the same live session it archives instead of resolving update and end against different sessions'
    checked: true
  - id: ambiguity-never-leaks
    text: 'When multiple active sessions exist, the session wrapper binds explicitly to the current session identity or fails fast instead of silently updating a different session'
    checked: true
  - id: human-surface-single-truth
    text: 'The human-facing session flow has one truthful backend path for interface wrappers, without MCP/CLI divergence reintroducing session-selection drift'
    checked: true
  - id: tests-cover-cross-session-regression
    text: 'Targeted tests cover multiple live sessions and prove `/session-end` cannot update one session and archive another'
    checked: true
---
# fix: session-end-cross-session-update

> session-end updates the wrong live session before archiving the requested one when multiple sessions exist

## Problem

The current interface session flow can drift across session identities
when `/session-end` performs its required final update.

Observed failure in the Patina repo on March 11, 2026:

- Claude ran `/session-end` for live session `20260311-214231-YVPT`
- the wrapper first called MCP `session.update`
- that update resolved a different active session:
  `20260311-214203-DQM9`
- the wrapper then manually recovered by ending `YVPT` explicitly

That violates the intended product contract for this slice:

- session UX remains human-driven via `/session-start`,
  `/session-update`, and `/session-end`
- `/session-end` performs update then end
- the durable truth for the session lives in the project-local artifact
  under `layer/sessions/`

The wrapper should behave like a single action bound to one live session.
Instead, it currently leaks internal multi-session ambiguity back to the
interface and then to the human.

## Root Cause

The current session surface still has two overlapping behavioral paths:

- public MCP tools: `session.start`, `session.update`, `session.end`
- CLI/JSON session capability: `patina ai session ... --json`

That overlap lets the wrapper compose update and end through independent
session resolution steps.

The specific risk points are:

- `session.update` can resolve the "current" session by environment or
  active-session heuristics
- `session.end` can target a different explicit runtime id or file id
- the wrapper prompt can therefore update one session and archive
  another

This is a classic surface-divergence bug: the same human command is not
backed by one canonical session-binding contract.

## Fix

Narrow the session wrapper surface so update and end share one explicit
session identity.

### 1. Bind update and end to the same session

`/session-end` must resolve the target live session once, then use that
same runtime/file identity for both:

- the final update
- the archive/end operation

No second "current session" lookup should happen in the middle of the
same wrapper action.

### 2. Fail fast on ambiguity

If the wrapper cannot determine one unambiguous active session for the
current interface/runtime, it should stop and tell the interface what
selector is required. It must not silently choose one session for
update and another for end.

### 3. Collapse the human-facing session surface toward one backend path

This bug is evidence that public MCP session lifecycle tools are adding
surface divergence rather than value for this phase.

The fix should move toward:

- human session UX via `/session-start`, `/session-update`,
  `/session-end`
- deterministic backend session capability via
  `patina ai session ... --json`

Whether the MCP lifecycle tools are fully removed in this exact slice
or turned into thin delegates, the product must stop exposing multiple
independent session-resolution behaviors to the wrappers.

## Exit Criteria

1. `/session-end` updates the same live session it archives.
2. Multiple active sessions cannot cause a cross-session update/end
   mismatch.
3. The interface session wrapper uses one truthful backend session path.
4. Regression tests cover the observed failure shape from the March 11,
   2026 Claude validation run.
