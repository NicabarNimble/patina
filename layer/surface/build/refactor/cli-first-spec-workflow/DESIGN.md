# Design: CLI-First Spec Workflow

## Why This Design

Spec workflow is durable project truth. It should not depend on which
runtime, wrapper, or MCP client happened to be in front of the user.

This design responds to two realities:

- the CLI now has stronger deterministic spec behavior
- multiple frontends can still create drift if they act like primary
  spec surfaces

The fix is not to remove every convenience layer. The fix is to make the
CLI canonical and make everything else thinner.

## Build Target

After this refactor:

- `patina spec ...` is the primary contract for spec lifecycle work
- MCP does not provide a richer or more authoritative spec path than the
  CLI
- interface-specific spec skills mostly point to CLI-native workflows
- spec behavior is easier to verify locally without frontend drift

## Resolved Decisions

- CLI is the canonical spec workflow surface
- MCP should not remain the primary spec lifecycle path
- useful behavior should be preserved where possible, but duplication and
  drift should be removed
- interface skills should stay thin and avoid re-implementing spec
  semantics

## Commits

1. `refactor(spec): make CLI the canonical spec workflow surface`
   - tighten docs/guidance around CLI-first spec operations

2. `refactor(mcp): thin or remove spec MCP surfaces`
   - keep only compatibility layers that do not reintroduce divergent
     behavior

3. `docs(interface): point runtime skills and templates to CLI-first spec usage`
   - keep interface-specific layers thin and aligned with the CLI

## Direct Code Targets

- `src/commands/spec/mod.rs` — canonical CLI surface
- `src/mcp/server/spec.rs` — current MCP spec handlers to thin/remove
- `src/mcp/server/tools.rs` — spec MCP tool exposure and descriptions
- `src/adapters/templates.rs` — interface template guidance for spec use
- `AGENTS.md` — runtime guidance if needed to reinforce CLI-first spec
  workflow
- `layer/surface/build/feat/deterministic-spec-scaffolds/SPEC.md` —
  supporting prior work for deterministic CLI behavior

## Verification Plan

- verify users can do primary spec work entirely through `patina spec`
- verify spec guidance in interface/runtime templates points to the CLI
- verify MCP no longer carries richer or divergent spec semantics
- verify local workflow remains usable without depending on MCP

## Build Readiness

This refactor should be conservative. It is not a broad anti-MCP pass.
It is a spec-workflow ownership cleanup. The CLI should become clearer
without needlessly breaking remaining Patina functionality.

## Open Questions

- Which spec-related MCP calls should remain temporarily as thin
  compatibility wrappers, if any?
- Should MCP spec tools eventually delegate by spawning CLI commands, or
  should spec MCP be removed entirely once interface docs are updated?
