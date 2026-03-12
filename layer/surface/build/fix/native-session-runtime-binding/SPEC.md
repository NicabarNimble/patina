---
type: fix
id: native-session-runtime-binding
status: complete
created: 2026-03-12
sessions:
  origin: 20260311-222000-JTNE
related:
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/fix/session-end-cross-session-update/SPEC.md
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/fix/session-native-wrapper-ux/SPEC.md
beliefs:
- dependable-rust
- spec-driven-design
- safety-boundaries
exit_criteria:
- id: native-pointer-written
  text: Starting a native interface session records a project-local current-session pointer for that interface runtime
  checked: true
- id: pointer-wins-over-stale-launch-env
  text: Native update, note, and end resolve the current interface pointer before stale launch-time `PATINA_SESSION_RUNTIME_ID` / `PATINA_SESSION_ID` values
  checked: true
- id: pointer-cleared-on-archive
  text: Ending a native interface session clears the pointer for that archived runtime so later commands do not keep targeting a dead session
  checked: true
- id: tests-lock-runtime-binding
  text: Tests cover pointer persistence, resolution precedence, and archive cleanup for native interface sessions
  checked: true
---
# fix: native-session-runtime-binding

## Problem

Native interface session commands still drift across sessions even after
moving lifecycle off MCP and restoring wrapper-first UX.

A real Claude transcript showed:

- `/session-start` created new session `JTNE`
- `/session-update` targeted older session `WEKU`
- `/session-end` archived `WEKU` twice
- the new `JTNE` artifact remained on disk but was not treated as the
  current native live session

So the bug is no longer just "which transport did the prompt use." The
backend is still resolving the wrong live runtime after a new native
session is started from inside an already-running interface process.

## Root Cause

The native interface path relies on launch-time environment variables:

- `PATINA_SESSION_RUNTIME_ID`
- `PATINA_SESSION_ID`
- `PATINA_AI_INTERFACE`

Those are correct when Patina launches Claude/OpenCode/Gemini, but they
do not update when the model later starts a new session from inside that
same process.

Native `patina ai session start` creates the new durable artifact and
live Mother session record, but it does not write any project-local
"current runtime for this interface" pointer. So later update/end/note
commands continue resolving against stale launch-time env or unrelated
active sessions.

## Fix

### 1. Write a native interface session pointer on start

When a native session is started, write a transient project-local
pointer under `.patina/local/` keyed by interface adapter. It should
record at least:

- adapter
- runtime_id
- file_id

This is local runtime state, not durable project truth.

### 2. Resolve the pointer before stale env

For native update/note/end resolution, the lookup order should be:

1. explicit selector
2. current interface pointer
3. launch-time env (`PATINA_SESSION_RUNTIME_ID`, `PATINA_SESSION_ID`)
4. filtered active-session lookup

That lets a new `/session-start` inside Claude immediately become the
session that `/session-update` and `/session-end` act on.

### 3. Clear the pointer on archive

When a native session ends, clear the matching interface pointer so
later commands do not keep targeting an archived runtime.

### 4. Keep compatibility isolated

Do not change the legacy `patina session ...` compatibility projection.
This fix is only for native interface runtime binding.

## Exit Criteria

1. Starting a native interface session records a project-local
   current-session pointer for that interface runtime.
2. Native update, note, and end resolve the current interface pointer
   before stale launch-time `PATINA_SESSION_RUNTIME_ID` /
   `PATINA_SESSION_ID` values.
3. Ending a native interface session clears the pointer for that
   archived runtime so later commands do not keep targeting a dead
   session.
4. Tests cover pointer persistence, resolution precedence, and archive
   cleanup for native interface sessions.
