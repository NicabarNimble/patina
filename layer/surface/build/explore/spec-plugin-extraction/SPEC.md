---
type: explore
id: spec-plugin-extraction
status: draft
created: 2026-02-25
sessions:
  origin: 20260225-093943
related:
- archived-blocker-resolution
- fact-crdt-substrate
beliefs:
- eventlog-is-truth
- eventlog-is-infrastructure
exit_criteria: []
---
# explore: Extract spec commands to WASM WIT plugin

> Spec lifecycle is opinionated and not patina-core — explore extracting it as a WASM WIT plugin with host-provided eventlog interface

## Question

Can the spec lifecycle system (create, promote, complete, abandon, pause,
resume, block, split, check) be extracted from the patina binary into a
WASM WIT plugin? What does the host/guest boundary look like, and how
does the eventlog fit?

## Context

The spec system is the most opinionated part of patina — it encodes a
specific governance model (draft → ready → active → complete) that may
not suit every project. Making it a plugin would:

- Let projects opt into spec governance (or swap for alternatives)
- Keep patina-core focused on knowledge infrastructure (scrape, scry, assay)
- Test the WIT plugin architecture with a real, non-trivial command set

### Current coupling

The spec system touches:

- **Filesystem**: reads/writes YAML frontmatter in `layer/surface/build/`
- **Database**: updates `patterns` table status, reads `spec_deps`
- **Git**: creates tags, stages files, commits, archives via `git rm`
- **Eventlog**: currently does NOT use the eventlog (this is the gap)
- **Release**: version bumps on completion (`patina::release`)
- **MCP**: 13 tool handlers in `server/spec.rs`

### Key design questions

1. **Host-provided vs plugin-scoped eventlog?**
   - Host-provided: plugin calls `emit-event()`, host writes to shared eventlog.
     Keeps "eventlog is infrastructure" belief. All plugins share one truth.
   - Plugin-scoped: plugin owns its own log. More isolation, but N eventlogs
     instead of one, and rebuild story gets complicated.
   - Leaning: host-provided. Plugin speaks stable interface, host owns storage.

2. **What WIT interfaces does the plugin need from the host?**
   - Filesystem read/write (YAML frontmatter)
   - Database queries (patterns table, spec_deps)
   - Git operations (tag, stage, commit, rm)
   - Eventlog (emit, query)
   - Release (version bump) — or does this stay host-side?

3. **MCP tool registration**
   - Plugins need to declare tools that the MCP server exposes
   - WIT could define tool schemas as part of the plugin interface
   - Host collects tool declarations from all plugins for `tools/list`

4. **What stays in patina-core?**
   - The eventlog module
   - Git abstractions
   - MCP server shell (dispatch, protocol)
   - Plugin loading and lifecycle
   - Scrape/scry/assay (core knowledge infrastructure)

### LiveStore inspiration

The LiveStore architecture (PR #45) already informed patina's eventlog
design. For plugins, the same principle applies: plugins emit events,
the host materializes views. The plugin doesn't need to know about
SQLite tables — it emits `spec.completed {id, version}` and the host
decides what indexes to maintain.

## Findings

_To be filled during exploration_

## Conclusions

_To be filled after exploration_
