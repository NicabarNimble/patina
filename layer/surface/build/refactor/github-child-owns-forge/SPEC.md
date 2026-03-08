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
exit_criteria:
- id: projection-handles-github-events
  text: "project_from_events() projects both forge.* and github.* event types into materialized views"
  checked: false
- id: scry-returns-github-issues
  text: "patina scry --include-issues returns results from github-connector data"
  checked: false
- id: assay-searches-github-events
  text: "patina assay searches github.issue and github.pr events"
  checked: false
- id: forgewriter-replaced-by-pipe
  text: "ForgeWriter trait replaced by pipe verb — init/repo flows go through mother to github child"
  checked: false
- id: forge-artifacts-deleted
  text: "grammars/forge/ and wit/schema/forge/ deleted after github schema replaces them"
  checked: false
- id: build-diagram-updated
  text: "build.md architecture diagram reflects connector model (no more scrape forge)"
  checked: false
---
# refactor: GitHub Child Owns All GitHub Interaction

> ForgeWriter bypasses pipe by shelling out to gh CLI. github-connector emits events but they dont project into searchable views. Consolidate all GitHub interaction into the github child.

## Current State

Two parallel systems for GitHub interaction, neither complete:

**1. ForgeWriter (legacy, pre-pipe):**
- `src/git/writer.rs` — trait: `fork()`, `create_repo()`, `is_authenticated()`, `current_user()`, `repo_exists()`
- Shells out to `gh` CLI directly from init/repo commands
- Bypasses mother/pipe — no scheduling, retry, telemetry
- Consumers: `src/git/fork.rs` (init flow), `src/commands/repo/internal.rs` (repo add --contrib)

**2. github-connector (v0.40.0, pipe-native):**
- Native child binary speaking pipe protocol via broker
- Fetches issues/PRs via HTTP, emits `github.issue` / `github.pr` to events.db
- 102 facts verified (15 issues + 87 PRs)
- **Gap:** events land in events.db but never project into materialized views

**The projection gap:**
- `events.rs::project_from_events()` hardcodes `WHERE event_type = 'forge.issue'` / `'forge.pr'`
- FTS5 indexing reads from `forge_issues`/`forge_prs` tables
- `scry --include-issues` and `assay` query these tables
- Result: github-connector data is invisible to search

**Orphaned forge artifacts (reference, not yet deleted):**
- `grammars/forge/` — pipeline grammar plugin source, no producer (scrape forge deleted in v0.40.0)
- `wit/schema/forge/` — WIT schema + schema.toml with embedding/FTS5/table config
- `build.md` architecture diagram still shows "scrape forge"

## Target State

The github child owns ALL GitHub interaction:

1. **Read path** (working): fetch issues/PRs via broker → events.db
2. **Read projection** (broken, fix first): events.db → materialized views → FTS5 → searchable by scry/assay
3. **Write path** (future): fork, create-repo, auth check via pipe verbs replacing ForgeWriter

ForgeWriter trait deleted. Init/repo flows request GitHub operations through mother, which routes to the github child.

## Steps

### Phase 1: Fix the projection gap (small, do first)

Extend `events.rs` to handle both event type families:
- `project_from_events()`: `WHERE event_type IN ('forge.issue', 'github.issue')`
- Same for `forge.pr` / `github.pr`
- Dedup helpers: `issue_event_exists()`, `pr_event_exists()`
- FTS5: `populate_fts5_issues()`, `populate_fts5_prs()`
- Verify: `patina scrape` → `patina scry --include-issues` returns github-connector data

### Phase 2: Create github schema (wit/schema/github/)

Model after `wit/schema/forge/schema.toml`:
- Fact types: `github.issue`, `github.pr` (already emitted by connector)
- Embedding config with corpus query
- FTS5 field definitions
- Materialized view table references

### Phase 3: Move ForgeWriter into github child

- Define pipe verbs: `github.fork_repo`, `github.create_repo`, `github.auth_check`, `github.current_user`
- Implement in github-connector child binary
- Replace `ForgeWriter` calls in `src/git/fork.rs` with mother requests
- Replace `ForgeWriter` calls in `src/commands/repo/internal.rs` with mother requests
- Delete `src/git/writer.rs`, remove `pub mod writer` from `src/git/mod.rs`
- Init/launcher UX stays the same — just the plumbing changes

### Phase 4: Clean up forge artifacts

- Delete `grammars/forge/` (source + target/)
- Delete `wit/schema/forge/` (superseded by wit/schema/github/)
- Update `build.md` architecture diagram
- Remove forge WASM test fixtures from `plugin/internal/tests.rs` (test_schema_facts helper uses "forge" as test data — update to "github")

## Exit Criteria

- **projection-handles-github-events:** `project_from_events()` projects both `forge.*` and `github.*` events
- **scry-returns-github-issues:** `patina scry --include-issues` returns github-connector results
- **assay-searches-github-events:** `patina assay` finds `github.issue`/`github.pr` events
- **forgewriter-replaced-by-pipe:** ForgeWriter deleted, init/repo flows use mother
- **forge-artifacts-deleted:** `grammars/forge/` and `wit/schema/forge/` removed
- **build-diagram-updated:** `build.md` reflects connector model
