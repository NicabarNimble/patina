---
type: refactor
id: core-extraction
status: draft
created: 2026-03-04
blocked_by:
- plugin-infrastructure
sessions:
  origin: 20260304-120702
beliefs:
- code-is-not-core
- scrape-is-local-capture
- patina-is-domain-agnostic-knowledge-system
exit_criteria:
- id: children-complete
  text: All child specs (forge-plugin-extraction, scrape-simplification, core-plugin-extraction) are complete
  checked: false
---
# refactor: Core Extraction — Shrink Patina to Protocol + Stores

> Extract all domain-specific code from Patina core into plugins.
> Forge first (proves the pattern), then simplify scrape, then
> extract spec/session subsystems.

## Context

This is a **container spec** tracking three child specs that together
shrink Patina core to protocol + stores.

**Architecture context:**
- [[session-20260303-190855]] — "scrape code is NOT core — it's a
  capability added when a project needs code analysis"
- [[session-20260304-120702]] — decomposed scrape by protocol verb,
  established that external data is not scrape's job
- [[code-is-not-core]] — code analysis is a plugin, not protocol
- [[scrape-is-local-capture]] — scrape reads git, connectors handle external
- [[patina-is-domain-agnostic-knowledge-system]] — domain-agnostic means
  no domain code in core

**What's currently in core that shouldn't be:**
- Forge (2,345 LOC total) — GitHub API via `gh` CLI, sync engine,
  rate limiting, staging pipeline:
  - `src/forge/` (533 LOC: mod.rs, none.rs, types.rs, writer.rs)
  - `src/forge/github/` (442 LOC: mod.rs, internal.rs)
  - `src/forge/sync/` (708 LOC: mod.rs, internal.rs)
  - `src/commands/scrape/forge/` (604 LOC: forge subcommand handler)
  - `src/generated/schemas/forge.rs` (58 LOC)
- `src/commands/scrape/code/` — tree-sitter code parsing (should be
  grammar plugins only, partially done)
- Spec subsystem: `src/spec.rs` + `src/commands/spec/` + `src/mcp/server/spec.rs`
- Session subsystem: `src/session.rs` + `src/commands/session/`

**What IS core (the protocol + stores):**
- Event sourcing (eventlog, events.db)
- Belief system (evidence chains, supports/attacks, grounding)
- Search (FTS5 via assay, vectors via scry, progressive disclosure via context)
- Embeddings (ONNX Runtime, model management)
- Plugin dispatch (load WASM, route by world/role)
- Mother (federation, registries)

## Children

| Spec | What it delivers | Build order |
|------|-----------------|-------------|
| [[forge-plugin-extraction]] | Forge as a connector plugin, proves host_emit end-to-end | First |
| [[scrape-simplification]] | Scrape = local git capture only, external via connectors | Second |
| [[core-plugin-extraction]] | Spec + session subsystems as plugins, core is domain-agnostic | Third (hardest) |

## Exploration Needed

- **How much of scrape is protocol vs domain?** Git commit parsing and
  layer/ markdown parsing feel like protocol (they read the declaration
  store). Code parsing is clearly domain. Where's the line?
- **Can core-plugin-extraction work with current WIT?** Spec and session
  subsystems need filesystem, git, and MCP tool registration. Those host
  capabilities don't exist yet. This may need new WIT interfaces.

## Exit Criteria

This spec is complete when all three children are complete.
