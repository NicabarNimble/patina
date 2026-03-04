---
type: refactor
id: forge-plugin-extraction
status: draft
created: 2026-03-04
blocked_by:
- plugin-infrastructure
- host-emit-wit
sessions:
  origin: 20260303-184231
related:
- knowledge-system-architecture
- core-plugin-extraction
- persona-federation
beliefs:
- patina-is-domain-agnostic-knowledge-system
- patina-is-knowledge-protocol
exit_criteria:
- id: host-emit-wit
  text: WIT `emit` interface exists in host — plugins can emit facts to the eventlog via `emit-fact(event-type, data)`
  checked: false
- id: schema-ships-with-plugin
  text: forge plugin ships its own schema (WIT types + table defs) — host auto-installs on plugin load
  checked: false
- id: source-kind-dispatch
  text: '`patina scrape` dispatches to plugins by source kind — plugin manifest declares `source_kinds` field, host routes accordingly'
  checked: false
- id: forge-is-plugin
  text: forge connector (GitHub issues/PRs) runs as a mother-child WASM plugin, not built into `src/forge/`
  checked: false
---
# refactor: Extract forge connector to WASM plugin

> Move src/forge/ to a mother-child WASM plugin. Proves host_emit,
> schema-with-plugin, and source-kind dispatch end-to-end. First
> extraction from the knowledge-system-architecture vision.

## Current State

- `src/forge/` contains GitHub-specific API code, `gh` CLI integration
- Forge schema (`forge.wit`) lives in `.patina/schemas/`, managed centrally
- `patina scrape` has hardcoded code/forge paths — no plugin dispatch
- Plugins can read (scry/assay/context) but not write facts to the eventlog
- SDK supports 4 worlds but no data ingestion pattern

## Target State

- Forge runs as a mother-child plugin in `plugins/forge/`
- Plugin uses `host_emit` to write forge facts (issues, PRs) to eventlog
- Plugin ships its own schema — host auto-installs on load
- `patina scrape forge` routes through source-kind dispatch, not hardcoded path
- Pattern is proven and repeatable for future extractions (spec, sessions)

## Steps

1. Add `emit` interface to WIT host — `emit-fact(event-type, data) -> result<u64, string>`
2. Implement `host_emit` in mother-child world runtime
3. Add `source_kinds` field to `PluginProvides` manifest
4. Wire source-kind dispatch into `patina scrape`
5. Move `src/forge/` to `plugins/forge/` mother-child plugin
6. Plugin ships forge schema, host auto-installs

## Exit Criteria

See frontmatter.

## Non-Goals

- **Extracting spec or session subsystems.** That's [[core-plugin-extraction]].
- **Building new connectors** (Google Workspace, Obsidian, etc.). This spec
  builds the infrastructure they'll use; actual connectors are separate specs.
- **Delta-driven scraping for forge.** Forge is command-driven (`patina scrape forge`),
  not git-diff-driven. Delta dispatch is a separate concern.
