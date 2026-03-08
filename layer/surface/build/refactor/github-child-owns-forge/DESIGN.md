# Design: GitHub Child Owns All GitHub Interaction

## History

The forge system evolved through three eras:

1. **Era 1 (pre-v0.25):** `src/forge/` — monolithic module. ForgeReader fetched issues/PRs via `gh` CLI, ForgeWriter created repos/forks via `gh` CLI. Both shelled out directly.

2. **Era 2 (v0.25-v0.39):** Polymorphic extraction. `patina scrape forge` fetched data → wrote `.forge-issue`/`.forge-pr` staging files → `grammar-forge` WASM plugin tagged them → pipeline routed to `insert_issues`/`insert_prs` → materialized views → FTS5 → searchable. ForgeWriter stayed as-is.

3. **Era 3 (v0.40):** github-connector native child. Replaced the read path (fetch issues/PRs via broker). Deleted `src/forge/` (1,683 LOC), `src/commands/scrape/forge/` (705 LOC), `plugins/forge/` (640 LOC). But left the projection hardcoded to `forge.*` event types, creating a gap where github-connector data lands in events.db but can't be searched.

## Approach

Four phases, ordered by value. Phase 1 is a quick fix. Phases 2-4 are larger architectural work.

### Phase 1: Fix the projection gap (~10 lines of SQL)

**What's broken:**

```
github-connector → events.db (github.issue, github.pr)
                         ↓
              project_from_events()
              WHERE event_type = 'forge.issue'  ← misses github.*
                         ↓
              forge_issues / forge_prs tables   ← empty for new data
                         ↓
              FTS5 index                        ← nothing to index
                         ↓
              scry --include-issues             ← no results
```

**Fix:** In `src/commands/scrape/events.rs`, extend WHERE clauses:

```sql
-- project_from_events() issue query
WHERE e.event_type IN ('forge.issue', 'github.issue')

-- project_from_events() PR query
WHERE e.event_type IN ('forge.pr', 'github.pr')

-- issue_event_exists() dedup
WHERE event_type IN ('forge.issue', 'github.issue')

-- pr_event_exists() dedup
WHERE event_type IN ('forge.pr', 'github.pr')

-- populate_fts5_issues()
WHERE event_type IN ('forge.issue', 'github.issue')

-- populate_fts5_prs()
WHERE event_type IN ('forge.pr', 'github.pr')
```

Also fix event type matching in consumers:
- `src/commands/scry/internal/enrichment.rs:62` — matches on `forge.pr`
- `src/commands/assay/internal/search.rs:191` — matches on `forge.issue`/`forge.pr`

Both old forge data and new github-connector data coexist in the same materialized views. No table schema changes.

**Verification:**
```bash
patina mother run github                       # fetch → events.db
patina scrape                                  # project → materialized views
patina scry "open bugs" --include-issues       # should return results
```

### Phase 2: Create github schema (wit/schema/github/)

Model after `wit/schema/forge/schema.toml` (keep as reference until this is done):
- Fact types: `github.issue`, `github.pr` (already emitted by connector)
- Embedding config with corpus query for `github.*` events
- FTS5 field definitions
- Materialized view table references

### Phase 3: Move ForgeWriter into github child

**Current call chain:**
```
patina init
  → src/commands/init/internal/mod.rs
    → patina::git::ensure_fork()
      → src/git/fork.rs:317
        → writer().is_authenticated()     ← gh auth status
        → writer().current_user()         ← gh api user
        → writer().repo_exists()          ← gh repo view
        → writer().fork()                 ← gh repo fork
        → writer().create_repo()          ← gh repo create

patina repo add <url> --contrib
  → src/commands/repo/internal.rs
    → create_fork()
      → GitHubWriter.fork()              ← gh repo fork
```

**Target — pipe verb dispatch:**
```
patina init
  → mother.request("github.auth_check")
  → mother.request("github.current_user")
  → mother.request("github.repo_exists", {owner, repo})
  → mother.request("github.fork_repo", {repo_path})
  → mother.request("github.create_repo", {name, private})
```

The github child already has HTTP capability and GitHub auth. Adding write verbs is natural — same child, new actions.

### Phase 4: Clean up forge artifacts

- Delete `grammars/forge/` (source + target/)
- Delete `wit/schema/forge/` (superseded by wit/schema/github/)
- Update `build.md` architecture diagram (remove "scrape forge")
- Update forge test fixtures in `plugin/internal/tests.rs`

## Key Files

| File | Role | Phase |
|------|------|-------|
| `src/commands/scrape/events.rs` | Projection engine — fix SQL WHERE clauses | 1 |
| `src/commands/scry/internal/enrichment.rs:62` | Matches on `forge.pr` event type | 1 |
| `src/commands/assay/internal/search.rs:191` | Matches on `forge.issue`/`forge.pr` | 1 |
| `wit/schema/forge/schema.toml` | Template for github schema | 2 |
| `src/git/writer.rs` | ForgeWriter trait — delete | 3 |
| `src/git/fork.rs` | Consumer — rewrite to use mother | 3 |
| `src/commands/repo/internal.rs` | Consumer — rewrite to use mother | 3 |
| `src/git/mod.rs:12` | `pub mod writer;` — remove | 3 |
| `grammars/forge/` | Delete after github schema exists | 4 |
| `wit/schema/forge/` | Delete after github schema exists | 4 |
| `layer/core/build.md` | Update architecture diagram | 4 |

## Open Questions

1. **Init without mother:** The init flow currently works without mother running (direct `gh` shell-out). If we route through mother, init either:
   - **A)** Requires mother to be running (or auto-starts it)
   - **B)** Falls back to direct `gh` if mother is unavailable
   - **C)** Init bootstraps mother as part of setup

   Option B preserves current UX while preferring the pipe path. Option A is cleaner architecturally. Decision needed before Phase 3.

2. **Table naming:** Should `forge_issues`/`forge_prs` be renamed to `github_issues`/`github_prs`? Or kept generic since they hold data from both sources? Renaming is a migration concern — existing databases have these tables. Recommendation: keep current names, they describe the data domain (forge = code hosting platform), not the source.

3. **Event type unification:** Long-term, should `forge.*` and `github.*` be unified? Or will there be `gitea.*`, `gitlab.*` etc.? If multi-platform, the materialized views become a union across all forge-family event types. The IN clause approach in Phase 1 scales to this.
