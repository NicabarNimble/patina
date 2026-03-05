---
type: refactor
id: forge-plugin-extraction
status: draft
created: 2026-03-04
blocked_by:
- host-emit-wit
- plugin-roles
sessions:
  origin: 20260303-184231
related:
- core-plugin-extraction
- scrape-simplification
beliefs:
- patina-is-domain-agnostic-knowledge-system
- scrape-is-local-capture
- code-is-not-core
exit_criteria:
- id: forge-is-connector-plugin
  text: forge connector (GitHub issues/PRs) runs as a mother-child WASM plugin with role=connector, not built into `src/forge/`
  checked: false
- id: forge-emits-via-host
  text: forge plugin uses host_emit to write forge.issue and forge.pr events to eventlog with provenance=external
  checked: false
- id: schema-ships-with-plugin
  text: forge plugin declares its schema (WIT types + table defs + embedding config) — schema available when plugin runs
  checked: false
- id: forge-removed-from-core
  text: '`src/forge/` deleted. No GitHub API knowledge in Patina core.'
  checked: false
---
# refactor: Extract forge connector to WASM plugin

> Move src/forge/ to a mother-child WASM plugin with role=connector.
> First extraction from core, proves host_emit and plugin roles
> end-to-end.

## Context

**This spec was rewritten in session 20260304-120702.** The original
version (from session 20260303-184231) had 4 ECs including
`source-kind-dispatch` which is now the WRONG design — scrape doesn't
dispatch to external plugins. See [[scrape-is-local-capture]].

**Architecture context:**
- [[session-20260303-190855]] — forge audit revealed the plugin system
  couldn't support extraction (no host_emit, no role metadata)
- [[session-20260304-120702]] — refined architecture: forge is a
  connector, connectors are reusable, scrape is local only
- [[scrape-is-local-capture]] — scrape reads git. Forge runs
  independently as a connector, not via scrape dispatch.
- [[code-is-not-core]] — forge is domain-specific (GitHub). Must be a
  plugin.

## Current State

**`src/forge/` (1,683 LOC):**
- `mod.rs` — platform detection, `ForgeReader` trait, URL parsing
- `types.rs` — Issue, PullRequest, Comment domain types
- `github/mod.rs` + `github/internal.rs` — GitHub API via `gh` CLI
- `writer.rs` — fork/create repo operations via `gh` CLI
- `sync/mod.rs` + `sync/internal.rs` — incremental sync engine with
  750ms rate limiting, background process management, staging pipeline
- `none.rs` — null implementation for repos without forge

**`src/commands/scrape/forge/mod.rs` (604 LOC):**
- Hardcoded forge dispatch in scrape
- Detects forge from git remote, calls GitHub API, writes to eventlog
- Materializes forge_issues and forge_prs tables in patina.db

**`.patina/schemas/forge/`:**
- `schema.toml` — fact definitions, embedding config, FTS5 indexes
- `forge.wit` — WIT record types for issue and pull-request

## Target State

- `plugins/forge/` — mother-child WASM plugin, role=connector
- Plugin uses `host_emit` to write facts (not direct DB access)
- Plugin uses `host_http` for GitHub API calls (credentials via vault)
- Plugin ships its own schema (currently in `.patina/schemas/forge/`)
- `patina scrape` does NOT dispatch to forge — forge runs independently
  via `patina plugin run forge` or Mother continuous sync
- `src/forge/` deleted from core

**Data flow (new):**
```
GitHub API → forge connector plugin → host_emit → events.db
                                                    ↓
                                          patina.db projections
                                          (forge_issues, forge_prs)
```

**Data flow (old, being replaced):**
```
GitHub API → src/forge/ → scrape/forge/ → events.db + patina.db
```

## Steps

1. **Prerequisite:** [[host-emit-wit]] complete, [[plugin-roles]] complete
2. Create `plugins/forge/` with plugin.toml (world=mother-child, role=connector)
3. Move forge types to plugin (Issue, PullRequest, Comment)
4. Implement `tick()` → discover refs from commits, return fetch intents
5. Implement `handle("sync", ...)` → use host_http to call GitHub API,
   host_emit to write facts
6. Move schema from `.patina/schemas/forge/` to ship with plugin
7. Remove `src/forge/` from core
8. Remove `src/commands/scrape/forge/` dispatch path
9. Update `patina scrape` to NOT call forge (it's a connector now)
10. Integration test: plugin emits forge facts, verify projection tables

## Design Decisions (resolved in DESIGN.md)

- **Background sync: Mother daemon.** The plugin doesn't manage its
  own background process. Mother's daemon IS the background process —
  it calls `tick()` on schedule, the plugin discovers pending refs and
  syncs them. Replaces the `libc::fork()` pattern entirely.

- **Rate limiting: plugin-internal.** The plugin sleeps 750ms between
  API calls within a single `handle()` invocation. Mother doesn't need
  to know about GitHub's rate limits. Keep fixed delay initially,
  optimize with `X-RateLimit-Remaining` headers later.

- **ForgeWriter: moves to plugin.** The forge plugin handles reads AND
  writes to its source. Uses the same `host/http` + credentials. But
  ForgeWriter is NOT an exit criterion — data ingestion is the priority.

- **Staging files: eliminated.** With `host_emit`, the plugin emits
  facts directly to the eventlog. No `.forge-issue`/`.forge-pr` staging
  files, no grammar-forge pipeline plugin needed. Direct path:
  `GitHub API → host/http → plugin parses → host/emit → events.db`.

## Open Questions

- **Pagination without `gh`.** Plugin must follow GitHub's `Link`
  headers instead of relying on `gh --limit`. Standard pattern but more
  code. Not a design risk — implementation detail.

- **Schema installation mechanism.** When plugin ships its schema, does
  `patina plugin install` copy schema to `.patina/schemas/`? Or does
  the host resolve schemas from plugin directories? Needs design in
  [[spec-host-emit-wit]] validation requirements.

- **Projection table ownership.** Who creates `forge_issues`/`forge_prs`
  after extraction? Lean toward: schema.toml declares projections,
  scrape materializes them generically (extensible to any schema).

## Non-Goals

- **Gitea/Codeberg support.** Current code has placeholder for Gitea.
  Plugin can add this later, but it's not an EC.
- **Building the lake architecture.** Forge currently pipes directly
  to project eventlog (simple path). Lake intermediary is future work
  in [[data-architecture-v3]].
- **Scrape dispatch changes.** That's [[scrape-simplification]]. This
  spec extracts forge. Scrape changes are separate.
