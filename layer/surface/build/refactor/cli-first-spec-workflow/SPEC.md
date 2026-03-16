---
type: refactor
id: cli-first-spec-workflow
status: abandoned
created: 2026-03-12
sessions:
  origin: 20260312-001728
related:
  - src/commands/spec/mod.rs
  - src/mcp/server/spec.rs
  - src/mcp/server/tools.rs
  - src/adapters/templates.rs
  - layer/surface/build/feat/deterministic-spec-scaffolds/SPEC.md
exit_criteria:
  - id: cli-is-canonical-spec-surface
    text: "`patina spec ...` becomes the canonical spec workflow surface for create/show/check/promote/complete operations and interface wrappers route through that deterministic CLI contract"
    checked: false
  - id: mcp-no-longer-primary-for-specs
    text: "MCP no longer acts as the primary or richer spec lifecycle surface than the CLI, and spec-related MCP behavior is reduced, delegated, or removed accordingly"
    checked: false
  - id: interface-spec-layers-stay-thin
    text: "Claude/OpenCode/Gemini spec skills and commands stay thin, pointing to CLI-native workflows instead of reintroducing divergent spec logic"
    checked: false
  - id: no-loss-of-working-spec-functionality
    text: "Existing useful spec capabilities still work after the shift, with drift-prone duplication removed rather than silently broken"
    checked: false
  - id: migration-story-is-explicit
    text: "The repo documents how spec workflow moves from MCP-heavy usage to CLI-first usage so future agents and interfaces do not regress"
    checked: false
---
# refactor: CLI-First Spec Workflow

> Make `patina spec ...` the canonical spec workflow surface, remove MCP primacy from spec lifecycle operations, and keep interface wrappers thin over the deterministic CLI contract.

## Problem

Patina's spec workflow has become too easy to experience through
multiple surfaces that can drift:

- source code may evolve faster than the installed binary or MCP layer
- interface skills can make spec usage look richer or more correct than
  the canonical CLI contract
- MCP tools can become a shadow spec surface with different semantics or
  update cadence than `patina spec ...`

This session exposed the problem directly: the repo had new spec
behavior in source, but the invoked `patina` binary still reflected the
older installed behavior until it was reinstalled. That kind of drift is
exactly why specs should anchor on one deterministic local contract.

## Goal

Re-center spec workflow on the CLI so Patina has one canonical spec
surface that interfaces and agents can trust.

**Target shape:**

- `patina spec ...` is the source of truth for spec lifecycle behavior
- interface skills point users and agents to CLI-native spec flows
- MCP either delegates cleanly to the CLI-backed implementation or is
  removed from spec lifecycle responsibility
- spec truth is easier to test, upgrade, and reason about locally

## Status

Abandoned.

Rationale: CLI-first is still the desired direction, but this spec's execution
shape predates the current architecture trajectory and should be replaced with
new architecture-aligned slices.

Current state:

- the CLI spec system is improving and now supports stronger scaffolds,
  readiness lint, and handoff views
- MCP still exposes spec tools and keeps a parallel spec-facing surface
- interface docs/templates still mention MCP for spec workflow in some
  places

The direction is already shifting toward stronger CLI determinism, but
the architecture has not fully committed to CLI-first spec ownership.

## Non-Goals

- Do not rip out working functionality just because it passes through
  MCP today; remove or reduce duplication carefully.
- Do not make every interface identical; the goal is thin wrappers, not
  identical UX.
- Do not rebuild the whole Patina MCP story in this spec unless it is
  directly tied to spec workflow.
- Do not break existing useful automation without replacing it with a
  clear CLI-native path.

## Current State

- `src/commands/spec/*` contains the real spec lifecycle logic.
- `src/mcp/server/spec.rs` mirrors spec operations through MCP.
- `src/mcp/server/tools.rs` advertises spec MCP tools as first-class
  workflow surfaces.
- interface instructions can still teach MCP-first spec usage depending
  on runtime.

## Target State

- The CLI is the primary spec lifecycle interface.
- Interface skills stay thin and mostly advisory.
- MCP is no longer treated as the preferred spec workflow surface.
- Spec behavior is tested and validated primarily through CLI-native
  contracts.

## Solution

### 1. Make the CLI explicitly canonical

- Document `patina spec ...` as the canonical spec workflow surface.
- Make interface/runtime guidance point to CLI commands first.

### 2. Reduce MCP spec primacy

- Audit each spec-related MCP tool.
- Keep only what still makes sense as a thin wrapper or compatibility
  layer.
- Remove or de-emphasize MCP paths that create richer or divergent spec
  behavior than the CLI.

### 3. Keep interface layers thin

- Update interface templates and skills so they steer users to the CLI
  contract.
- Avoid re-encoding spec semantics in interface prompts.

### 4. Preserve working behavior during the shift

- If an MCP surface is removed, the CLI replacement path must already be
  clear.
- If an MCP surface remains temporarily, it should delegate cleanly and
  not own independent spec logic.

### 5. Make the migration obvious

- Document the CLI-first model in repo guidance.
- Ensure future agents do not default back to MCP-first spec handling.

## Implementation Order

1. Audit current spec behavior across CLI, MCP, and interface templates.
2. Decide which spec MCP tools remain as compatibility wrappers vs get
   removed.
3. Update interface/runtime guidance to point at CLI-native flows.
4. Remove or thin MCP spec surfaces accordingly.
5. Add verification so CLI is clearly the tested primary path.

## Resolved Decisions

- CLI is the canonical spec workflow surface.
- interface-specific spec layers should stay thin.
- we are not removing useful behavior blindly; the goal is to reduce
  drift and duplicated semantics.
- this refactor is about spec workflow first, not total MCP removal from
  all of Patina.

## Verification

- verify spec create/show/check/promote/complete flows through the CLI
- verify interface guidance points to the CLI contract
- verify remaining MCP spec behavior is thin/delegated rather than a
  richer parallel surface
- verify future agents can discover the CLI-first path from repo docs
  alone

## Exit Criteria

1. `patina spec ...` becomes the canonical spec workflow surface for
   create/show/check/promote/complete operations and interface wrappers
   route through that deterministic CLI contract.
2. MCP no longer acts as the primary or richer spec lifecycle surface
   than the CLI, and spec-related MCP behavior is reduced, delegated, or
   removed accordingly.
3. Claude/OpenCode/Gemini spec skills and commands stay thin, pointing
   to CLI-native workflows instead of reintroducing divergent spec
   logic.
4. Existing useful spec capabilities still work after the shift, with
   drift-prone duplication removed rather than silently broken.
5. The repo documents how spec workflow moves from MCP-heavy usage to
   CLI-first usage so future agents and interfaces do not regress.

## Build Readiness

This spec should be implemented after the deterministic scaffold work is
landed and before more spec behavior spreads across interface-specific
surfaces. The point is to reduce future drift, not to do a disruptive
platform rewrite.
