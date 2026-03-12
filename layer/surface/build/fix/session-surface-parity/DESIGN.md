# Design: Session Surface Parity

## Purpose

This fix closes a specific drift bug in Patina's session surface:

- native interfaces are taught MCP-first session workflow
- the runtime may not actually expose MCP session tools
- fallback currently drops into legacy compatibility semantics

That violates both truthful capability projection and semantic parity.

## Design Position

Patina should keep three ideas distinct:

1. **Capability core**
   Typed session lifecycle behavior.
2. **Projection surface**
   CLI, CLI `--json`, MCP, and interface command files.
3. **Mode**
   Native interface mode vs trusted compatibility mode.

The bug happened because mode was implicit and projection assumed more
availability than the runtime guaranteed.

## Rules

- Projection must be truthful about tool availability.
- Fallback must preserve semantics unless the user explicitly asks for a
  compatibility path.
- MCP and CLI are sibling projections over shared typed session logic.
- Command markdown teaches workflow; it does not define hidden behavior.

## Acceptable Implementation Shapes

Any implementation is acceptable if it achieves the exit criteria, but
the likely good shapes are:

### Option A: Native machine-readable AI session commands

Add a machine-readable native session entrypoint under `patina ai` and
make OpenCode slash commands fall back to that instead of
`patina session --json`.

Good because:

- semantics are obvious from the command surface
- native and compatibility paths stay visibly distinct

### Option B: Explicit session mode on shared CLI command

Keep `patina session --json`, but add an explicit mode or request shape
so the caller can say:

- native OpenCode session
- native Gemini session
- compatibility session

Good because:

- one CLI namespace
- one typed request seam

Risk:

- hidden mode flags can become opaque if not exposed clearly

## Key Files

- `src/mcp/server/session.rs`
  MCP session handlers; today these already route into shared Rust logic
  but use native request semantics.
- `src/commands/session/mod.rs`
  Machine-readable CLI surface; today this still defaults to
  compatibility semantics.
- `src/commands/session/internal.rs`
  The likely place to make native-vs-compatibility mode explicit.
- `src/commands/ai/*`
  Likely home if a native machine-readable AI session fallback is added.
- `resources/opencode/session-start.md`
- `resources/opencode/session-update.md`
- `resources/opencode/session-end.md`
  OpenCode teaching layer that must remain truthful.

## Verification

- unit tests that prove native and compatibility start requests do not
  collapse into each other
- tests that prove MCP `session.start` and native CLI fallback produce
  the same mode and interface identity
- projection tests that ensure OpenCode command templates describe only
  actually available or truthful fallback behavior
- command-level check showing OpenCode-started session artifacts no
  longer report `interface = legacy-cli` when using the native fallback

## Non-Goals

- This fix does not require a full capability registry.
- This fix does not require universal MCP registration for every
  interface runtime.
- This fix does not migrate or remove the trusted legacy session path.
