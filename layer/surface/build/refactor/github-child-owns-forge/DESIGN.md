# Design: GitHub Child Owns All GitHub Interaction

## History

The forge system evolved through three eras:

1. **Era 1 (pre-v0.25):** `src/forge/` — monolithic module. ForgeReader fetched issues/PRs via `gh` CLI, ForgeWriter created repos/forks via `gh` CLI. Both shelled out directly.

2. **Era 2 (v0.25-v0.39):** Polymorphic extraction. `patina scrape forge` fetched data → wrote `.forge-issue`/`.forge-pr` staging files → `grammar-forge` WASM plugin tagged them → pipeline routed to `insert_issues`/`insert_prs` → materialized views → FTS5 → searchable. ForgeWriter stayed as-is.

3. **Era 3 (v0.40):** github-connector native child. Replaced the read path (fetch issues/PRs via broker). Deleted `src/forge/` (1,683 LOC), `src/commands/scrape/forge/` (705 LOC), `plugins/forge/` (640 LOC). But left the projection hardcoded to `forge.*` event types, creating a gap where github-connector data lands in events.db but can't be searched.

4. **Era 3 fix (v0.40, session 20260307-234302):** Extended projection/FTS5/search/oxidize to handle both `forge.*` and `github.*` event types. Created `wit/schema/github/`. Deleted `grammars/forge/` and `wit/schema/forge/`. Gap closed — but hardcoded, see [[spec-schema-driven-projection]].

## What Was Done

### Phase 1: Fix the projection gap (DONE)

Extended WHERE clauses across 4 subsystems to handle both event type families:

| File | Changes |
|------|---------|
| `src/commands/scrape/events.rs` | 6 SQL WHERE clauses: `= 'forge.X'` → `IN ('forge.X', 'github.X')` |
| `src/commands/scry/internal/enrichment.rs` | PR kind detection: also match `github.pr` |
| `src/commands/assay/internal/search.rs` | FTS5 filter + source_id formatting: also match `github.*` |
| `src/commands/oxidize/mod.rs` | Embedding corpus query + PR kind: also match `github.*` |

### Phase 2: Create github schema (DONE)

- `wit/schema/github/schema.toml` — fact types (`github.issue`, `github.pr`), embedding config (shares offset slot 5 with forge), FTS5 indexes pointing to shared `forge_issues`/`forge_prs` tables
- `wit/schema/github/github.wit` — WIT type definitions (platform-agnostic, same shape as forge)
- Tests in `src/commands/schema/internal.rs` updated to validate github schema

### Phase 4: Clean up forge artifacts (DONE)

- Deleted `grammars/forge/` (Cargo.toml, plugin.toml, src/)
- Deleted `wit/schema/forge/` (schema.toml, forge.wit)
- Updated `build.md` architecture diagram: "scrape forge" → "github-connector"
- Updated `build.md` command table: "scrape forge" → "github-connector"

### Phase 3: Move ForgeWriter into github child (DEFERRED)

**Not part of this spec.** ForgeWriter replacement requires:
- Pipe protocol expansion for request/response verbs (not just fetch/emit)
- Design decision on init-without-mother (open question)
- Touches init/launcher flow which is a separate concern

ForgeWriter remains at `src/git/writer.rs`, consumed by `src/git/fork.rs` and `src/commands/repo/internal.rs`. Still shells out to `gh` CLI.

## Architectural Debt

The hardcoded `IN ('forge.issue', 'github.issue')` pattern doesn't scale. A new connector (gitea, gitlab) would require modifying core SQL in 4 subsystems. [[spec-schema-driven-projection]] addresses this: the pipeline should read installed schemas to discover event_type → table mappings dynamically.

## Key Files

| File | Role | Status |
|------|------|--------|
| `src/commands/scrape/events.rs` | Projection engine | DONE — handles both event families |
| `src/commands/scry/internal/enrichment.rs` | Vector search enrichment | DONE — matches github.pr |
| `src/commands/assay/internal/search.rs` | FTS5 search | DONE — filters github.* |
| `src/commands/oxidize/mod.rs` | Embedding corpus | DONE — includes github.* |
| `wit/schema/github/schema.toml` | GitHub schema definition | DONE — created |
| `wit/schema/github/github.wit` | WIT type definitions | DONE — created |
| `layer/core/build.md` | Architecture diagram | DONE — updated |
| `src/git/writer.rs` | ForgeWriter trait | DEFERRED — still shells out to gh |
| `src/git/fork.rs` | Init fork flow | DEFERRED — still uses ForgeWriter |
| `src/commands/repo/internal.rs` | Repo contrib flow | DEFERRED — still uses ForgeWriter |

## Open Questions (resolved or deferred)

1. ~~**Init without mother:**~~ DEFERRED with Phase 3. Not part of this spec.

2. **Table naming:** Kept `forge_issues`/`forge_prs` as shared tables. Both forge and github data coexist. Schema.toml declares which table to project into.

3. **Event type unification:** Deferred to [[spec-schema-driven-projection]]. The IN clause approach works for now. Schema-driven discovery will handle multi-platform scaling.
