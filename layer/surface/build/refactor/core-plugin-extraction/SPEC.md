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
  text: spec subsystem runs as a plugin (role=extension) — lifecycle, git tags, MCP tools all routed through plugin interface
  checked: false
- id: core-is-domain-agnostic
  text: Patina core has no domain-specific code — no Rust syntax knowledge, no GitHub API knowledge, no spec lifecycle management. All domain logic lives in plugins.
  checked: false
---
# refactor: Extract spec subsystem to WASM plugin

> Extract the spec subsystem to a plugin. Result: Patina core has no
> development workflow tooling. This is the hardest extraction and
> depends on patterns proven by forge extraction.

## Context

**Architecture context:**
- [[session-20260303-190855]] — abandoned knowledge-system-architecture
  had spec extraction as EC 5 of a 12-EC mega-spec.
  Now broken out as a focused spec with proper prerequisites.
- [[code-is-not-core]] — core should have no domain code
- [[patina-is-domain-agnostic-knowledge-system]] — a Patina project
  that tracks legal documents doesn't need a spec lifecycle engine
  designed for software development
- [[wit-is-contract-wasm-is-one-runtime]] — the spec subsystem
  communicates through WIT contracts, but needs new host capabilities

**Sessions and adapters stay in core.** See [[spec-core-extraction]]
DESIGN.md — "Don't extract the interaction layer until there's an
alternative." Sessions are the primary interaction path. Adapters are
how Patina connects to AI tools. Extraction is revisited when a native
Patina UI exists or the CLI can stand alone without an AI tool driving
it. At that point sessions and adapters become extension plugins.

## Current State

**`src/spec.rs`** (640 LOC) — spec types and operations
**`src/commands/spec/`** — most complex subsystem:
- `mod.rs` (528 LOC) — CLI command dispatch
- `internal/` (3,091 LOC) — queries, mutations, archive, create,
  split, queue
- 16 MCP tool handlers in `src/mcp/server/spec.rs` (361 LOC):
  spec.list, spec.ready, spec.blocked, spec.next, spec.show,
  spec.check, spec.promote, spec.complete, spec.abandon, spec.pause,
  spec.resume, spec.block, spec.split, spec.set, spec.create,
  spec.history
- Git operations: tags, staging, commits, archive
- Release versioning (semver bump on completion)
- YAML frontmatter parsing in `layer/surface/build/`
- Database: `patterns` table, `spec_deps` table
- Dependencies: filesystem, git CLI, rusqlite, YAML parser

## Target State

- Spec runs as a plugin (role=extension, world=command or task)
- Core provides: event sourcing, beliefs, search, embeddings, plugin
  dispatch, Mother. Nothing domain-specific.
- Any new workflow tool (e.g., project management, CRM contacts) is
  purely additive — install a plugin, no core changes.

## Steps

1. **Prerequisite:** [[scrape-simplification]] complete, host_emit
   proven, plugin roles established
2. Audit spec subsystem's host capability needs (filesystem, git, MCP)
3. Design new WIT host interfaces needed (host_filesystem, host_git,
   host_mcp_register)
4. Extract spec subsystem (complex — 16 MCP tools, git tags, releases)
5. Verify core has no domain imports remaining

## Exploration Needed (SIGNIFICANT)

This spec has the most open questions in the entire roadmap.

- **New host capabilities.** Spec needs filesystem write, git tag/commit,
  and MCP tool registration. None of these exist in the current WIT
  host. Designing these interfaces is a major effort. Each one needs
  its own security model (filesystem scoped to project, git operations
  audited, MCP registration validated).

- **MCP tool registration from plugins.** Currently MCP tools are
  hardcoded in the binary. If spec becomes a plugin, its 16 MCP tools
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

- **Split this spec?** Given the exploration needed, this might need
  to be split into: (a) design host capabilities, (b) extract spec.
  Two specs instead of one.

## Non-Goals

- **Persona system.** That's [[persona-federation]].
- **Session or adapter extraction.** Sessions and adapters stay in core
  until there's an alternative interaction path. See [[spec-core-extraction]]
  DESIGN.md.
- **Redesigning spec functionality.** Extract as-is first. Improve later.
- **Making spec work on Cloudflare.** Local-first extraction.
  Edge deployment is future.
