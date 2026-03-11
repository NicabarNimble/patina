# Design: OpenCode Session and Spec Capabilities — CLI, JSON, and MCP Vertical Slice

## Approach
- Add typed session result seams for CLI `--json` and MCP, while leaving the trusted compatibility prose path intact.
- Expose session lifecycle through MCP using live-session resolution rather than `.patina/local/active-session.md` as sole truth.
- Keep OpenCode projection thin by teaching MCP session/spec workflow in bootstrap/context/command templates instead of duplicating business logic.

## Commits
1. `feat(session): add typed json results for start/update/end/list` — machine-readable session output for CLI and MCP.
2. `feat(mcp): expose session lifecycle tools` — add `session.start|update|end|list` schemas and handlers.
3. `feat(opencode): teach MCP session/spec workflow` — update bootstrap, AGENTS context, and slash-command templates.
4. `test(opencode): cover tool visibility and json seams` — lock the vertical slice in with focused tests.

## Key Files
- `src/commands/session/mod.rs` — CLI `--json` surface for the operator slice.
- `src/commands/session/internal.rs` — typed session results plus shared live-session update/end helpers.
- `src/mcp/server/session.rs` — session MCP handlers on the shared typed seams.
- `src/mcp/server/tools.rs` — tool-list projection for the session/spec slice.
- `src/adapters/opencode/internal/mod.rs` — OpenCode AGENTS context projection.
- `src/adapters/launch.rs` — bootstrap projection teaching the MCP-first operator surface.
- `resources/opencode/session-start.md` — thin OpenCode workflow projection for session start.
- `resources/opencode/session-update.md` — thin OpenCode workflow projection for session update.
- `resources/opencode/session-end.md` — thin OpenCode workflow projection for session end.

## Open Questions
- Broader capability-registry unification remains in `cli-mcp-skill-unification`; this slice stays deliberately narrow.
# Design: OpenCode Session and Spec Capabilities

## Purpose

This is a deliberately narrow vertical slice.

The goal is not to finish universal skills or fully unify all CLI/MCP
capabilities across the whole system. The goal is to get OpenCode
working well through `patina ai` using the Patina tools that matter most
right now:

- session workflow
- spec workflow

with:

- strong native CLI behavior
- stable `--json`
- MCP exposure
- thin OpenCode projection

## Why Narrow Is Better Here

The architecture already improved a lot:

- runtime rebuilt
- sessions rebuilt
- `patina ai` exists

The current gap is the operator experience inside OpenCode. Solving that
well is more important than widening the platform with low-confidence
generic abstractions.

This spec protects quality by reducing scope.

## Design Rules

- Read code before writing code
- Keep shared capability logic in Rust, not adapter markdown
- Prefer typed return structs over loosely shaped maps
- Prefer one code path with multiple renderers over duplicated behavior
- Keep adapter projections thin
- Preserve the trusted `patina` compatibility path

## Target Operator Flow

1. User runs `patina ai opencode`
2. OpenCode starts through the new interface path
3. OpenCode can use session workflow commands naturally
4. OpenCode can inspect and operate on specs
5. The same underlying capability can be reached via:
   - CLI
   - CLI `--json`
   - MCP
6. Session/spec artifacts remain truthful and git-backed

## Session Slice

### Needed behaviors

- start a session
- update a session
- end a session

These should be reachable:

- directly via CLI
- via JSON output for machine use
- via MCP tools for interface discovery/invocation
- via OpenCode command projection

### Important constraint

Do not reintroduce dependence on `.patina/local/active-session.md` as
the only truth source for the new path.

## Spec Slice

### Needed behaviors

The first slice should focus on the spec operations OpenCode actually
needs to stay productive:

- list
- next
- show
- check

Mutating spec operations can be included where the workflow truly needs
them, but should not be widened casually.

### Important constraint

Spec authority stays with the spec subsystem, not with OpenCode prompt
files or ad hoc tool wrappers.

## MCP Role

MCP is the teaching and invocation surface for the interface layer in
this slice.

That means:

- the relevant session and spec tools must appear in `tools/list`
- they must have usable schemas
- handlers should call shared typed logic, not parse human CLI text

## OpenCode Projection Role

OpenCode-specific code should do only what is specific to OpenCode:

- file layout
- command file placement
- bootstrap/context projection
- launch behavior

OpenCode-specific code should not become the owner of session/spec
business logic.

## Likely File Seams

- session CLI/result types:
  - `src/commands/session/mod.rs`
  - `src/commands/session/internal.rs`
- spec CLI/result types:
  - `src/commands/spec/*`
- MCP:
  - `src/mcp/server/tools.rs`
  - `src/mcp/server/mod.rs`
  - `src/mcp/server/spec.rs`
  - new session MCP handler file if useful
- OpenCode projection:
  - `src/interface/internal/bootstrap.rs`
  - `src/adapters/templates.rs`
  - `src/adapters/opencode/*`
  - `resources/opencode/*`

## Verification

At minimum:

- unit tests for session/spec JSON result shapes where practical
- MCP tool-list assertions for new session tools
- command-level checks for session/spec JSON output
- projection check that OpenCode actually gets the session commands on
  the `patina ai` path

## Outcome

If this spec lands well, you get:

- a real OpenCode working path
- preserved code quality
- less adapter hack debt
- a credible base to later expand universal skill/MCP unification
