---
type: refactor
id: scrape-simplification
status: draft
created: 2026-03-04
blocked_by:
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

`src/commands/scrape/mod.rs` `execute_all()` (lines 76-154) dispatches to:
1. `git::run()` — git commit history (LOCAL) ✓
2. `execute_code_incremental()` — code parsing via grammar plugins (LOCAL) ✓
3. `layer::run()` — layer/ markdown parsing (LOCAL) ✓
4. `beliefs::run()` — belief regrounding (LOCAL) ✓

Note: `forge::run()` is NOT part of execute_all(). It's a separate
subcommand handler (`patina scrape forge`, mod.rs:397). The delta
system (`delta.rs`) has no forge source-kind either. Forge is already
architecturally separate from the main scrape pipeline — the work here
is removing the subcommand and the `src/commands/scrape/forge/` module
(604 LOC) after [[forge-plugin-extraction]] moves it to a connector.

**Code references:**
- `src/commands/scrape/mod.rs` lines 76-154 — execute_all() dispatch
- `src/commands/scrape/mod.rs` line 397 — forge subcommand handler
- `src/commands/scrape/delta.rs` — ScrapeDelta classification (no forge)
- `src/commands/scrape/forge/mod.rs` — forge subcommand (604 LOC, to be removed)
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
2. Remove forge subcommand handler from `src/commands/scrape/mod.rs`
3. Remove `src/commands/scrape/forge/` directory
4. Remove `pub mod forge` declaration from scrape mod.rs
5. Add `patina connector sync` command that discovers installed
   connector-role plugins and runs them
6. Optional: add `patina scrape --all` convenience flag

## Design Decisions (resolved in DESIGN.md)

- **Layer/ parsing is protocol.** layer/ is Patina's own format —
  every project has it. Parsing markdown with YAML frontmatter is
  reading the declaration store, like git parsing `.git/`. Not a
  domain-specific format; not a grammar plugin.

- **Grammar dispatch stays in scrape.** Grammar plugins parse local
  files during the index phase of scrape. They're local-only work
  (read file, parse AST, emit facts). [[scrape-is-local-capture]]
  distinguishes local capture (scrape) from external capture
  (connectors). Grammars are local. They stay.

- **Connector freshness is connector-internal.** Scrape's delta
  system tracks git commits and file timestamps — local state.
  Connectors track API-level freshness: last sync timestamps,
  pagination cursors, incremental fetch markers. Different mechanisms,
  different owners. Mother's lake registry provides metadata
  freshness at the Mother level.

## Non-Goals

- **Building the connector command infrastructure.** This spec removes
  forge from scrape. The `patina connector` command may need its own
  spec if it's complex.
- **Changing how grammar plugins work.** Pipeline grammar dispatch
  via scrape is fine. Only external connectors move out.
- **Mother continuous operation.** That's [[continuous-operation]].
  This spec handles the scrape side.
