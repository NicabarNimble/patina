---
type: refactor
id: scrape-simplification
status: draft
created: 2026-03-04
blocked_by:
- core-extraction
- forge-plugin-extraction
sessions:
  origin: 20260304-120702
beliefs:
- scrape-is-local-capture
- patina-is-knowledge-protocol
exit_criteria:
- id: scrape-no-external-dispatch
  text: '`patina scrape` does not call any external API or connector — it reads git and local files only'
  checked: false
- id: scrape-protocol-aligned
  text: scrape implements capture + index protocol verbs for local sources (git history, layer/, code via grammar plugins)
  checked: false
- id: connectors-run-independently
  text: external data ingestion runs via `patina connector sync` or Mother continuous operation, not via scrape
  checked: false
---
# refactor: Scrape Simplification — Local Capture Only

> Decouple external data fetching from scrape. Scrape reads git and
> local files. Connectors handle external sources independently.

## Context

**Architecture context:**
- [[session-20260303-190855]] — "scrape becomes local capture from
  git ONLY. External data comes through plugins independently."
- [[session-20260304-120702]] — decomposed scrape by protocol verb:
  capture from git (local), index captured data (local), capture from
  external (connectors, NOT scrape).
- [[scrape-is-local-capture]] — scrape reads git, connectors handle
  external. Both write to eventlog, both feed beliefs, but separate.
- [[patina-is-knowledge-protocol]] — protocol verbs are
  capture/index/search/believe/evolve. Scrape = capture + index.

## Current State

`src/commands/scrape/mod.rs` `execute_all()` dispatches to:
1. `git::run()` — git commit history (LOCAL) ✓
2. `execute_code_incremental()` — code parsing via grammar plugins (LOCAL) ✓
3. `layer::run()` — layer/ markdown parsing (LOCAL) ✓
4. `beliefs::run()` — belief regrounding (LOCAL) ✓
5. **`forge::run()`** — GitHub API calls (EXTERNAL) ✗ doesn't belong

The delta system (`delta.rs`) computes `ScrapeDelta` including a
`forge` source-kind for staged forge files. This forge path needs
removal after forge extraction.

**Code references:**
- `src/commands/scrape/mod.rs` lines 97-147 — dispatch routes
- `src/commands/scrape/delta.rs` — ScrapeDelta classification
- `src/commands/scrape/forge/mod.rs` — forge dispatcher (604 LOC)
- `src/commands/scrape/code/extract_v2.rs` — pipeline plugin dispatch

## Target State

`patina scrape` does:
1. `git::run()` — capture git history ✓ (unchanged)
2. Grammar plugins — index code/files ✓ (unchanged)
3. `layer::run()` — capture layer/ data ✓ (unchanged)
4. `beliefs::run()` — regrounding ✓ (unchanged)

`patina scrape` does NOT:
5. ~~forge::run()~~ — removed, forge is a connector
6. ~~any external API call~~ — connectors handle this

External data ingestion:
- `patina connector sync [name]` — run a specific connector
- `patina connector sync --all` — run all project connectors
- Mother continuous operation — connectors run on schedule via daemon

**Convenience:** `patina scrape --all` could be sugar for
"scrape local + sync all connectors" to ease transition. But
architecturally they're separate operations.

## Steps

1. **Prerequisite:** [[forge-plugin-extraction]] complete (forge code
   moved to plugin, `src/forge/` deleted)
2. Remove forge dispatch path from `src/commands/scrape/mod.rs`
3. Remove forge source-kind from `delta.rs` ScrapeDelta
4. Remove `src/commands/scrape/forge/` directory
5. Add `patina connector sync` command that discovers installed
   connector-role plugins and runs them
6. Optional: add `patina scrape --all` convenience flag

## Exploration Needed

- **Is layer/ parsing protocol or domain?** Parsing `layer/sessions/*.md`
  and `layer/surface/epistemic/beliefs/*.md` feels protocol-adjacent —
  it reads the declaration store. But it assumes markdown format and
  YAML frontmatter. Should this be a grammar plugin too? **Lean toward:
  layer/ parsing is protocol. It reads Patina's own format, not a
  domain-specific format.**

- **Code grammar dispatch.** Pipeline grammar plugins already handle
  code parsing. Should they continue to be dispatched by scrape? Or
  should grammar-role plugins be explicitly invoked? **Lean toward:
  scrape dispatches grammar plugins for local files. Grammars are
  local-file parsers, not external connectors. They belong in scrape.**

- **Delta system for connectors.** Scrape has a sophisticated delta
  system (only re-parse changed files). Connectors need their own
  freshness tracking (last sync timestamp, incremental fetch). This
  is connector-internal, not scrape's concern.

## Non-Goals

- **Building the connector command infrastructure.** This spec removes
  forge from scrape. The `patina connector` command may need its own
  spec if it's complex.
- **Changing how grammar plugins work.** Pipeline grammar dispatch
  via scrape is fine. Only external connectors move out.
- **Mother continuous operation.** That's [[continuous-operation]].
  This spec handles the scrape side.
