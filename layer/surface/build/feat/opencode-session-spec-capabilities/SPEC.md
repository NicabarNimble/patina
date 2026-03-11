---
type: feat
id: opencode-session-spec-capabilities
status: complete
created: 2026-03-11
sessions:
  origin: 20260311-112321-EF79
related:
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/cli-mcp-skill-unification/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/feat/patina-ai-interface-layer/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/feat/session-narrative-system/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/agentic-surface-architecture/SPEC.md
beliefs:
  - session-capture
  - spec-driven-design
  - dependable-rust
  - unix-philosophy
  - context-files-are-rules-not-docs
exit_criteria:
  - id: opencode-session-commands-work
    text: OpenCode launched through `patina ai` has working `/session-start`, `/session-update`, and `/session-end` operator commands on the new interface path
    checked: true
  - id: session-cli-json-stable
    text: Session operations needed by OpenCode are backed by stable CLI `--json` output rather than prose scraping
    checked: true
  - id: session-mcp-exposed
    text: Session operations needed by OpenCode are exposed through MCP from the same capability logic as the CLI path
    checked: true
  - id: spec-cli-json-stable
    text: The spec operations OpenCode needs are backed by stable CLI `--json` output and remain spec-authoritative
    checked: true
  - id: spec-mcp-exposed
    text: The spec operations OpenCode needs are exposed through MCP from the same capability logic as the CLI path
    checked: true
  - id: thin-opencode-projection
    text: The OpenCode adapter path is reduced to launch/projection wiring and does not own duplicated behavior truth for session/spec workflows
    checked: true
  - id: quality-bar-preserved
    text: The implementation follows layer/core values with narrow typed Rust seams, read-code-before-write discipline, and no broad adapter hack regression
    checked: true
---
# feat: OpenCode Session and Spec Capabilities — CLI, JSON, and MCP Vertical Slice

> Get OpenCode working well through patina ai with the Patina session and spec tools you actually use, backed by strong CLI, stable JSON, MCP exposure, and thin adapter projection without lowering code quality.

## Problem

The new Patina architecture now has:

- Mother-backed live sessions
- a native `patina ai` interface layer
- a preserved compatibility launcher

But the OpenCode operator experience is still incomplete.

Today:

- the session backend exists
- `patina session` and `patina spec` exist
- MCP exposes many Patina tools already
- OpenCode templates and wrappers exist on disk

Yet the actual `patina ai` OpenCode path does not fully expose the
session/spec workflow the operator expects. The result is a split where
the architecture is ahead of the interface experience.

This spec deliberately does not try to solve all universal skill or MCP
design at once. It delivers one high-quality vertical slice that gets
OpenCode operational using the Patina tools that matter most.

## Solution

### Goal

Make OpenCode usable through `patina ai` for real work by ensuring the
session and spec workflows are first-class capabilities across:

- CLI
- CLI `--json`
- MCP
- thin OpenCode projection/injection

The implementation must hold the existing code-quality bar:

- follow layer/core values
- keep APIs narrow and typed
- avoid adapter-local truth duplication
- prefer shared capability seams over convenience hacks

### 1. Finish OpenCode session workflow on the new path

OpenCode launched through `patina ai` must have real operator access to:

- `/session-start`
- `/session-update`
- `/session-end`

These should work on the new session substrate, not by pretending the
old singleton local session model is still the truth.

### 2. Make session commands machine-readable

The session operations OpenCode needs should expose stable `--json`
output from the same typed logic that powers the command.

The JSON output should be suitable for:

- interface wrappers
- scripts
- future automation
- MCP projection

### 3. Expose session capabilities through MCP

OpenCode should be able to learn and call the relevant session
operations through MCP, backed by the same underlying typed capability
logic as the CLI path.

### 4. Tighten spec workflow for OpenCode

The subset of spec operations OpenCode actually needs should be made
solid and machine-readable:

- listing/next/show/check at minimum
- mutations only as needed for the actual OpenCode workflow

The important rule is that specs remain authoritative and the JSON/MCP
surfaces remain consistent with the CLI path.

### 5. Keep OpenCode projection thin

The OpenCode path should not become a second owner of:

- session behavior
- spec behavior
- tool semantics

It should project and expose those capabilities, not redefine them.

### Implementation Sequence

#### Commit 1: `feat(session-cli): stabilize session json for operator slice`

Audit the `patina session` commands used by OpenCode and add/normalize
stable `--json` output where needed.

Targets:

- `src/commands/session/mod.rs`
- `src/commands/session/internal.rs`
- shared typed result structures if needed

#### Commit 2: `feat(session-mcp): expose session operator tools`

Add MCP tool definitions and handlers for the OpenCode-needed session
operations, backed by the same typed logic as the CLI path.

Targets:

- `src/mcp/server/tools.rs`
- `src/mcp/server/mod.rs`
- new or adjacent MCP handler module(s)

#### Commit 3: `feat(opencode): wire session commands into patina ai projection`

Ensure the OpenCode interface path actually surfaces the session command
workflow on the new `patina ai` path.

Targets:

- `src/interface/internal/bootstrap.rs`
- `src/adapters/templates.rs`
- `src/adapters/opencode/*`
- any OpenCode context/bootstrap projection files

#### Commit 4: `feat(spec-cli): stabilize spec json for operator slice`

Confirm and tighten the spec commands OpenCode needs so they have
reliable machine-readable output.

Targets:

- `src/commands/spec/*`

#### Commit 5: `feat(spec-mcp): align spec operator tools for OpenCode`

Ensure the spec operations OpenCode needs are available through MCP from
the same capability path.

Targets:

- `src/mcp/server/spec.rs`
- related spec command shared logic

#### Commit 6: `test(opencode): verify vertical slice end to end`

Add focused tests and command-level verification for:

- session command projection
- JSON output shape
- MCP tool presence
- OpenCode bootstrap visibility

## Exit Criteria

1. OpenCode launched through `patina ai` has working
   `/session-start`, `/session-update`, and `/session-end` operator
   commands on the new interface path.
2. Session operations needed by OpenCode are backed by stable CLI
   `--json` output rather than prose scraping.
3. Session operations needed by OpenCode are exposed through MCP from
   the same capability logic as the CLI path.
4. The spec operations OpenCode needs are backed by stable CLI `--json`
   output and remain spec-authoritative.
5. The spec operations OpenCode needs are exposed through MCP from the
   same capability logic as the CLI path.
6. The OpenCode adapter path is reduced to launch/projection wiring and
   does not own duplicated behavior truth for session/spec workflows.
7. The implementation follows layer/core values with narrow typed Rust
   seams, read-code-before-write discipline, and no broad adapter hack
   regression.

## Non-Goals

- solve all universal skill projection for every interface
- redesign every Patina command around a new capability registry in one pass
- finish Gemini or Claude parity
- replace the trusted `patina` compatibility path
- lower standards just to get OpenCode "working somehow"
