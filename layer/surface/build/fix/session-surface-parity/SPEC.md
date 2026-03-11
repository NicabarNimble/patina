---
type: fix
id: session-surface-parity
status: complete
created: 2026-03-11
sessions:
  origin: 20260311-135625-KH7V
related:
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/feat/opencode-session-spec-capabilities/SPEC.md
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/cli-mcp-skill-unification/SPEC.md
- /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/feat/patina-ai-interface-layer/SPEC.md
beliefs:
- interfaces-are-not-core
- compatibility-paths-buy-trust
- dependable-rust
- unix-philosophy
- spec-driven-design
exit_criteria:
- id: truthful-availability
  text: OpenCode and other native interface projections only teach MCP session tools when those tools are actually available in the runtime surface they are projecting into
  checked: true
- id: fallback-semantic-parity
  text: When a session command falls back from MCP, the fallback preserves native session semantics instead of silently creating a legacy-cli compatibility session
  checked: true
- id: single-session-core
  text: MCP session handlers and machine-readable CLI fallbacks route through one typed session capability seam with explicit mode selection rather than drifting behavior flags
  checked: true
- id: projection-explicitness
  text: Generated OpenCode session command files explicitly encode discovery and fallback rules so interface actors do not have to guess whether MCP or CLI is authoritative
  checked: true
- id: compatibility-preserved
  text: The trusted legacy `patina session` compatibility path remains available for explicit compatibility use and is not broken by native parity fixes
  checked: true
- id: tests-lock-surface
  text: Tests cover MCP availability truth, native-fallback parity, and OpenCode command projection behavior
  checked: true
---
# fix: Session Surface Parity — Truthful MCP Availability and Native Fallback Semantics

> Fix the drift between MCP-exposed session tools, OpenCode slash-command teaching, and CLI fallback semantics so interfaces use truthful availability and preserve native session behavior.

## Problem

Patina now has three overlapping ways to start or continue a session:

- native `patina ai` interface flow
- MCP session tools
- legacy `patina session` compatibility commands

The architecture intended these to share one capability core while
projecting into different frontends.

But in practice a real operator test exposed drift:

- OpenCode slash commands taught the model to call MCP `session.start`
- the runtime surface available inside OpenCode did not actually expose
  that MCP tool
- the command then fell back to `patina session start --json`
- the fallback succeeded, but created a `legacy-cli` compatibility
  session instead of preserving native OpenCode session semantics

This creates two problems at once:

1. the interface is taught capabilities that are not truthfully
   available
2. the fallback path changes the meaning of the operation

The result is exactly the kind of agent-surface drift Patina is trying
to eliminate:

- prompts say one thing
- MCP exposes another
- CLI fallback does a third

## Root Cause

The session surface is only partially unified.

The current implementation has one good property:

- MCP handlers call into the same Rust session subsystem rather than
  shelling out to CLI prose parsing

But it still has a semantic split:

- MCP `session.start` builds a native session request
- CLI `patina session start --json` uses a legacy compatibility request

At the same time, OpenCode command projection currently teaches
MCP-first session behavior without a truthful way to know whether MCP is
actually available inside the current interface runtime.

So Patina has shared code, but not shared semantics or truthful
projection.

## Fix

Patch the session surface so capability discovery and fallback behavior
line up.

### 1. Make session capability availability truthful in projection

OpenCode and future native interface projections must not unconditionally
teach MCP session tools unless the projected runtime can actually use
them.

That can be solved in one of two acceptable ways:

- ensure native OpenCode runtime actually exposes the required MCP tools,
  or
- project commands that first verify MCP availability and otherwise use
  a truthful native fallback

The important rule is that generated instructions cannot promise a tool
that is absent.

### 2. Separate compatibility fallback from native fallback

Today the CLI JSON fallback for `/session-start` routes through the old
compatibility semantics. That is trustworthy for the old path, but wrong
for the native path.

The fix is to define an explicit native machine-readable session start
entrypoint that preserves:

- native adapter identity
- native interface kind
- native narrative/live-session behavior
- no silent compatibility projection unless explicitly requested

This may take the form of:

- a new `patina ai session ...` machine-readable path, or
- expanded session CLI requests with an explicit native mode, or
- a shared capability function projected into both MCP and a native CLI
  JSON command

The exact route is less important than preserving semantics.

### 3. Make the shared session seam explicit

Codify the shared session capability seam so there is one typed place
that decides:

- native vs compatibility mode
- adapter/interface identity
- whether compatibility markdown projection should be written

CLI and MCP should become sibling renderers over this seam, not two
entrypoints that happen to call the same module with different implicit
defaults.

### 4. Keep command projection thin and honest

OpenCode command files should remain a teaching layer, not a business
logic layer.

They should instruct the model to:

- use MCP when available
- use the correct native machine-readable fallback when MCP is not
  available
- avoid inventing compatibility behavior when the user is on the native
  path

### 5. Preserve the trusted compatibility path

Do not break explicit use of:

- `patina session start`
- `patina session update`
- `patina session end`

Those remain the compatibility path and should continue to work for
operators who intentionally use them.

The fix is about preventing accidental compatibility downgrade from the
native interface path, not deleting the trusted old path.

### Commit Shape

1. `fix(session): make native and compatibility mode explicit`
2. `fix(ai): add truthful native machine-readable session fallback`
3. `fix(opencode): project truthful MCP-or-native session commands`
4. `test(session): lock availability and parity behavior`

## Exit Criteria

1. OpenCode and other native interface projections only teach MCP
   session tools when those tools are actually available in the runtime
   surface they are projecting into.
2. When a session command falls back from MCP, the fallback preserves
   native session semantics instead of silently creating a `legacy-cli`
   compatibility session.
3. MCP session handlers and machine-readable CLI fallbacks route through
   one typed session capability seam with explicit mode selection rather
   than drifting behavior flags.
4. Generated OpenCode session command files explicitly encode discovery
   and fallback rules so interface actors do not have to guess whether
   MCP or CLI is authoritative.
5. The trusted legacy `patina session` compatibility path remains
   available for explicit compatibility use and is not broken by native
   parity fixes.
6. Tests cover MCP availability truth, native-fallback parity, and
   OpenCode command projection behavior.
