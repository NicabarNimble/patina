---
type: fix
id: session-native-wrapper-ux
status: complete
created: 2026-03-12
sessions:
  origin: 20260311-215919-WEKU
related:
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/fix/session-end-cross-session-update/SPEC.md
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/mother-owned-interface-bundles/SPEC.md
beliefs:
- interfaces-are-not-core
- compatibility-paths-buy-trust
- dependable-rust
- spec-driven-design
exit_criteria:
- id: wrapper-first-session-ux
  text: Claude, OpenCode, and Gemini session command assets call bundle-local wrapper scripts instead of teaching raw `patina ai session ...` transport in prompt text
  checked: true
- id: native-note-parity
  text: Native interface bundles expose `/session-note` through the same live-session backend surface as start, update, and end instead of falling back to legacy compatibility projection
  checked: true
- id: runtime-guidance-clean
  text: Generated runtime guidance points interface actors at native bundled session wrappers and no longer advertises raw `patina ai session ...` lifecycle commands as the primary surface
  checked: true
- id: tests-lock-wrapper-surface
  text: Tests cover wrapper generation, projected command assets, native note parity, and runtime guidance
  checked: true
---
# fix: session-native-wrapper-ux

## Problem

Patina's session backend is semantically safer after removing public MCP
session lifecycle tools, but the interface UX regressed.

Claude now visibly reads `AGENTS.md`, decides a transport, and shells
out to raw `patina ai session ... --json` commands from the session
markdown assets. That is technically correct, but it feels more
mechanical and more MCP-shaped than the earlier adapter-era session
commands.

The old command assets felt native because they had two properties:

- the markdown was concise and workflow-oriented
- the transport lived behind local scripts rather than in the prompt

There is also a parity gap:

- `/session-note` still points at `patina session note`, which is the
  compatibility projection path rather than the native live-session
  surface used by the current interface bundles

So the session surface is now correct on start/update/end semantics, but
not coherent or product-quality as an interface skill/command layer.

## Root Cause

The cross-session fix pushed transport truth directly into the command
markdown instead of into bundle-local wrapper scripts.

That created three UX problems:

- prompt assets now expose `patina ai session ...` machinery directly
- session commands still tell the model to consult `AGENTS.md` for a
  transport decision even though session lifecycle no longer branches on
  MCP availability
- `/session-note` never got lifted onto the native live-session path

The bundle model already ships native wrapper scripts under
`.{interface}/bin/`, but the current session command assets are bypassing
them. The right seam exists; the projection is just not using it.

## Fix

Restore a wrapper-first session UX without reintroducing MCP lifecycle
ambiguity.

### 1. Make bundle-local session wrappers the authoritative native entrypoint

For Claude, OpenCode, and Gemini, generate per-interface wrapper scripts
that:

- set the current interface identity explicitly
- route to the native `patina ai session ...` backend
- keep machine-readable behavior inside the script, not in prompt text

The command markdown/TOML should call the wrapper scripts, not raw CLI
transport.

### 2. Bring `/session-note` onto the native live-session path

Add a native `ai session note` capability and route bundle wrappers to
it. The note command should resolve the current live session with the
same interface/session binding rules as update and end, then append to
the durable session artifact.

### 3. Simplify session command assets back toward the old feel

Session command assets should:

- stop teaching transport selection from `AGENTS.md`
- call local wrapper scripts directly
- keep the richer human workflow guidance from the older markdown assets

This keeps the interface-native feel while preserving one truthful
backend.

### 4. Clean runtime guidance

`AGENTS.md` runtime truth should point actors at the bundled session
wrappers as the primary lifecycle surface. It should no longer advertise
raw `patina ai session ...` commands as the preferred interface-facing
workflow.

## Exit Criteria

1. Claude, OpenCode, and Gemini session command assets call bundle-local
   wrapper scripts instead of teaching raw `patina ai session ...`
   transport in prompt text.
2. Native interface bundles expose `/session-note` through the same
   live-session backend surface as start, update, and end instead of
   falling back to legacy compatibility projection.
3. Generated runtime guidance points interface actors at native bundled
   session wrappers and no longer advertises raw `patina ai session ...`
   lifecycle commands as the primary surface.
4. Tests cover wrapper generation, projected command assets, native note
   parity, and runtime guidance.
