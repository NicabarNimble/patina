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
- forge-plugin-extraction
- persona-federation
beliefs:
- patina-is-domain-agnostic-knowledge-system
- code-is-not-core
- wit-is-contract-wasm-is-one-runtime
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

> Extract spec and session subsystems to plugins. Result: Patina core
> has no domain-specific code. This is the hardest extraction and
> depends on patterns proven by forge extraction.

## Context

**Architecture context:**
- [[session-20260303-190855]] — abandoned knowledge-system-architecture
  had spec/session extraction as ECs 5 and 6 of a 12-EC mega-spec.
  Now broken out as a focused spec with proper prerequisites.
- [[code-is-not-core]] — core should have no domain code
- [[patina-is-domain-agnostic-knowledge-system]] — a Patina project
  that tracks legal documents doesn't need spec or session subsystems
  designed for software development
- [[wit-is-contract-wasm-is-one-runtime]] — subsystems communicate
  through WIT contracts, but may need new host capabilities

## Current State

**`src/spec/`** — most complex subsystem:
- 13 MCP tool handlers (spec_list, spec_show, spec_create, spec_promote,
  spec_complete, spec_pause, spec_resume, spec_block, spec_split,
  spec_set, spec_check, spec_history, spec_next, etc.)
- Git operations: tags, staging, commits, archive
- Release versioning (semver bump on completion)
- YAML frontmatter parsing in `layer/surface/build/`
- Database: `patterns` table, `spec_deps` table
- Dependencies: filesystem, git CLI, rusqlite, YAML parser

**`src/session/`** — simpler but still touches many surfaces:
- Session start/end/update/note lifecycle
- Git branch handling and tagging
- Active session file management (`.patina/local/active-session.md`)
- Session archiving to `layer/sessions/`
- Event emission (session.start, session.end)

## Target State

- Spec and session run as plugins (role=subsystem, world=mother-child)
- Core provides: event sourcing, beliefs, search, embeddings, plugin
  dispatch, Mother. Nothing domain-specific.
- Any new subsystem (e.g., project management, CRM contacts) is purely
  additive — install a plugin, no core changes.

## Steps

1. **Prerequisite:** [[scrape-simplification]] complete, host_emit
   proven, plugin roles established
2. Audit spec subsystem's host capability needs (filesystem, git, MCP)
3. Design new WIT host interfaces needed (host_filesystem, host_git,
   host_mcp_register)
4. Extract session subsystem first (simpler, fewer dependencies)
5. Extract spec subsystem (complex — 13 MCP tools, git tags, releases)
6. Verify core has no domain imports remaining

## Exploration Needed (SIGNIFICANT)

This spec has the most open questions in the entire roadmap.

- **New host capabilities.** Spec needs filesystem write, git tag/commit,
  and MCP tool registration. None of these exist in the current WIT
  host. Designing these interfaces is a major effort. Each one needs
  its own security model (filesystem scoped to project, git operations
  audited, MCP registration validated).

- **MCP tool registration from plugins.** Currently MCP tools are
  hardcoded in the binary. If spec becomes a plugin, its 13 MCP tools
  need dynamic registration. This is a fundamental change to how Patina
  exposes tools to LLMs. **This may be the hardest single problem in
  the entire roadmap.**

- **Is spec really domain-specific?** Spec-driven-design is a core
  value ([[spec-driven-design]]). If specs are core to how Patina
  operates, is extracting them to a plugin the right move? Or should
  spec stay in core because it's part of the protocol (specs are how
  the "evolve" verb manifests)? **Strong tension with
  [[patina-is-domain-agnostic-knowledge-system]].** A law firm doesn't
  need specs. But specs might be to Patina what branches are to git —
  not domain-specific, but a core workflow concept.

- **Session similarly.** Sessions track work. Is that domain-specific
  or protocol? A chat agent on Cloudflare has "conversations" not
  "sessions." A manufacturing QA system has "inspections" not "sessions."
  The concept might be protocol (track units of work) but the
  implementation is development-specific.

- **Split this spec?** Given the exploration needed, this might need
  to be split into: (a) design host capabilities, (b) extract sessions,
  (c) extract spec. Three specs instead of one.

## Non-Goals

- **Persona system.** That's [[persona-federation]].
- **Redesigning spec or session functionality.** Extract as-is first.
  Improve later.
- **Making spec/session work on Cloudflare.** Local-first extraction.
  Edge deployment is future.
