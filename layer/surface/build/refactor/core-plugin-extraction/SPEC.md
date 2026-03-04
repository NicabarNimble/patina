---
type: refactor
id: core-plugin-extraction
status: draft
created: 2026-03-04
blocked_by:
- scrape-simplification
sessions:
  origin: 20260303-184231
related:
- knowledge-system-architecture
- forge-plugin-extraction
- persona-federation
beliefs:
- patina-is-domain-agnostic-knowledge-system
- fix-architecture-not-documentation
exit_criteria:
- id: spec-extracted-to-plugin
  text: spec subsystem runs as a plugin — lifecycle, git tags, MCP tools all routed through plugin interface
  checked: false
- id: sessions-extracted-to-plugin
  text: session subsystem runs as a plugin — start/end/update/note routed through plugin interface
  checked: false
- id: core-is-domain-agnostic
  text: Patina core has no domain-specific code — no Rust syntax knowledge, no GitHub API knowledge, no email parsing. All domain logic lives in plugins.
  checked: false
---
# refactor: Extract spec and session subsystems to WASM plugins

> Extract spec and session subsystems to plugins using the pattern
> proven by forge-plugin-extraction. Result: Patina core has no
> domain-specific code.

## Current State

- `src/spec/` — spec lifecycle management baked into core
- `src/session/` — session tracking baked into core
- Both touch filesystem, git, eventlog, and MCP surface
- Spec is the most complex subsystem (13 MCP tool handlers, git tags,
  release versioning, YAML frontmatter)

## Target State

- Spec and session run as plugins using host_emit, host filesystem,
  and host git interfaces proven by forge extraction
- Core provides: event sourcing, beliefs, search, embeddings, plugin
  dispatch, Mother. Nothing else.
- Any new domain (email, calendar, notes) is purely additive — install
  a plugin, no core changes

## Steps

1. Extract sessions first (simpler, fewer dependencies)
2. Extract spec last (most complex — needs host git ops, MCP tool registration)
3. Verify core has no domain imports remaining

## Exit Criteria

See frontmatter.

## Non-Goals

- **New host WIT interfaces beyond what forge established.** If spec
  extraction needs new host capabilities (git ops, MCP registration),
  those are part of this spec's scope, but minimize them.
- **Persona system.** That's [[persona-federation]].
