---
type: refactor
id: github-child-owns-forge
status: draft
created: 2026-03-08
sessions:
  origin: 20260307-222328
related:
- github-connector
- pipe-architecture
- schema-driven-projection
- mother-broker-github
exit_criteria:
- id: projection-handles-github-events
  text: "project_from_events() projects both forge.* and github.* event types into materialized views"
  checked: true
- id: scry-returns-github-issues
  text: "patina scry --include-issues returns results from github-connector data"
  checked: true
- id: assay-searches-github-events
  text: "patina assay searches github.issue and github.pr events"
  checked: true
- id: forge-artifacts-deleted
  text: "grammars/forge/ and wit/schema/forge/ deleted after github schema replaces them"
  checked: true
- id: build-diagram-updated
  text: "build.md architecture diagram reflects connector model (no more scrape forge)"
  checked: true
---
# refactor: GitHub Child Owns All GitHub Interaction

> ForgeWriter bypasses pipe by shelling out to gh CLI. github-connector emits events but they dont project into searchable views. Consolidate all GitHub interaction into the github child.

## Current State (post session 20260307-234302)

**Read path: COMPLETE.** The github-connector fetches issues/PRs via HTTP, broker routes to events.db, projection handles both `forge.*` and `github.*` event types, FTS5 indexes them, scry and assay find them.

**Write path: DEFERRED.** ForgeWriter (`src/git/writer.rs`) still shells out to `gh` CLI for fork/create-repo/auth. This is a separate concern — the init/launcher flow needs pipe protocol expansion (request/response verbs, not just fetch/emit). Deferred to a future spec.

**Forge artifacts: DELETED.** `grammars/forge/` and `wit/schema/forge/` removed. `wit/schema/github/` created as replacement. `build.md` updated.

**Architectural note:** The projection fix hardcodes `IN ('forge.issue', 'github.issue')` in core code. This works but doesn't scale to new connectors. [[spec-schema-driven-projection]] will make the pipeline read installed schemas instead of hardcoded event types.

## What Was Done (session 20260307-234302)

### Phase 1: Fix the projection gap (DONE)

Extended 8+ SQL WHERE clauses across 4 subsystems:
- `src/commands/scrape/events.rs` — projection engine (6 clauses)
- `src/commands/scry/internal/enrichment.rs` — vector search enrichment
- `src/commands/assay/internal/search.rs` — FTS5 search filter + source_id formatting
- `src/commands/oxidize/mod.rs` — embedding corpus query

All now handle both `forge.*` and `github.*` event types.

### Phase 2: Create github schema (DONE)

- `wit/schema/github/schema.toml` — fact types, embedding config (shares offset slot 5), FTS5 indexes
- `wit/schema/github/github.wit` — WIT type definitions (same shape as forge, platform-agnostic)
- Tests updated in `src/commands/schema/internal.rs`

### Phase 4: Clean up forge artifacts (DONE)

- Deleted `grammars/forge/` (plugin source, Cargo.toml, plugin.toml)
- Deleted `wit/schema/forge/` (schema.toml, forge.wit)
- Updated `build.md` architecture diagram and command table
- Updated schema tests to point to `wit/schema/github/`

### Phase 3: Move ForgeWriter into github child (DEFERRED)

ForgeWriter replacement is a separate concern from the read-path fix:
- Requires pipe protocol expansion for request/response verbs (not just fetch/emit)
- Open design question: init flow without mother running
- Depends on pipe-architecture evolution
- Tracked separately, not part of this spec's exit criteria

## Exit Criteria

- **projection-handles-github-events:** DONE — projects both `forge.*` and `github.*` events
- **scry-returns-github-issues:** DONE — verified with `patina scry --include-issues`
- **assay-searches-github-events:** DONE — verified with `patina assay search --include-issues`
- **forge-artifacts-deleted:** DONE — `grammars/forge/` and `wit/schema/forge/` removed, `wit/schema/github/` created
- **build-diagram-updated:** DONE — `build.md` shows github-connector instead of scrape forge
