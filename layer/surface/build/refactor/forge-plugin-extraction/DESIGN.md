# Design: Extract Forge Connector to WASM Plugin

## Why Forge Is First

Forge extraction is the proof-of-concept for the entire plugin
infrastructure. Every design decision in [[spec-plugin-infrastructure]]
— host_emit, plugin roles, connector architecture — gets validated
end-to-end when forge runs as a plugin instead of compiled-in code.

If forge works as a connector plugin, the pattern is proven. Every
subsequent extraction (code grammars, spec subsystem) follows the same
steps. The risk is front-loaded here. See [[spec-core-extraction]]
DESIGN.md, "Forge First — Proving the Pattern."

## What Forge Does Today

Forge is a `gh` CLI wrapper with a sync engine. It does NOT call
GitHub's REST API directly — it shells out to `gh` (8 call sites in
`github/internal.rs`), parses the JSON output, and converts it to
Patina domain types.

**The read path** (`ForgeReader` trait, `src/forge/mod.rs:41`):
- `list_issues()`, `list_pull_requests()` — paginated list fetches
- `get_issue()`, `get_pull_request()` — single item fetch with details
- `get_issue_count()`, `get_pr_count()` — search API for totals
- Platform detection via git remote URL parsing

**The sync engine** (`src/forge/sync/`, 708 LOC):
- Discovers `#N` references from git commit messages
- Resolves them via API with 750ms rate limiting between calls
- Writes resolved items to `.forge-issue`/`.forge-pr` staging files
- Staging files flow through `grammar-forge` pipeline plugin on next scrape
- Background sync via `libc::fork()` + PID file (Unix-only)
- Safe to interrupt — progress saved after each item

**The scrape dispatcher** (`src/commands/scrape/forge/mod.rs`, 604 LOC):
- Detects forge from git remote
- Full sync path: calls GitHub API, writes events to events.db
- Creates materialized views (`forge_issues`, `forge_prs`) in patina.db
- Dedup via `json_extract()` queries on events.db

**The write path** (`src/forge/writer.rs`, 231 LOC):
- `ForgeWriter` trait — fork repo, create repo via `gh` CLI
- Separate concern from data ingestion — but lives in same module

## The gh CLI Problem

The current code shells out to `gh` for everything. As a WASM plugin,
this can't work — WASM plugins can't spawn processes. The plugin must
use `host/http` to call GitHub's REST API directly.

**What changes:**
- Replace `Command::new("gh")` calls with `host_http::http-get()` /
  `host_http::http-post()` calls to `api.github.com`
- Authentication moves from `gh auth` to credential injection via the
  vault (`host_secrets` in manifest)
- JSON parsing stays the same — `gh` CLI returns GitHub API JSON anyway,
  just with `--json` field selection. The plugin parses the raw API response.
- Pagination changes from `gh` `--limit` flag to following `Link` headers

**What this means for manifest:**
```toml
[plugin]
name = "forge-github"
world = "mother-child"
role = "connector"

[capabilities]
host_log = true
host_http = ["api.github.com"]
host_emit = true

[capabilities.host_secrets]
"api.github.com" = { secret = "github-token", location = "bearer" }

[schemas.forge]
package = "patina:schema/forge@1.0.0"
```

## Staging Files vs host_emit

Today forge uses a two-stage pipeline:
1. Sync engine writes `.forge-issue`/`.forge-pr` staging files
2. `grammar-forge` pipeline plugin parses staging files into events

With `host_emit`, the plugin emits facts directly to the eventlog.
Staging files become unnecessary. The `grammar-forge` plugin may also
become unnecessary — it exists to parse staging files that won't exist.

**The direct emit path:**
```
GitHub API → host/http → plugin parses JSON → host/emit → events.db
```

**Materialized views** (`forge_issues`, `forge_prs` in patina.db) are
projections of events. They're rebuilt from events.db by scrape. The
plugin doesn't touch patina.db — it emits facts, and the projection
system materializes them. This matches the CQRS principle from
[[spec-data-architecture-v2]]: events.db is the write model, patina.db
is the read model.

## Sync Engine Migration

The sync engine has two modes that map to different plugin patterns:

**Full sync** (scrape/forge/ dispatcher):
- Fetches all issues/PRs up to a limit
- Used for initial population and periodic refresh
- Maps to `handle("sync", ...)` in the plugin

**Incremental sync** (sync/ engine):
- Discovers `#N` refs from commits, resolves individually
- 750ms rate limiting, background process, PID management
- Maps to `tick()` → discover refs, `handle("resolve", ...)` → fetch one

**Background sync** currently uses `libc::fork()`. As a mother-child
plugin, Mother's daemon IS the background process. The plugin doesn't
need its own background management:
- Mother calls `tick()` on schedule (configurable per connector)
- Plugin discovers pending refs, returns toy requests or handles inline
- Mother manages lifecycle, health checks, restart on failure

Rate limiting stays plugin-internal — the plugin sleeps 750ms between
API calls within a single `handle()` invocation. Mother doesn't need
to know about GitHub's rate limits.

## What Gets Removed from Core

After extraction, these paths are deleted:
- `src/forge/` (1,683 LOC) — entire module
- `src/commands/scrape/forge/` (604 LOC) — scrape dispatcher
- `src/generated/schemas/forge.rs` (58 LOC) — generated schema types

What STAYS:
- `.patina/schemas/forge/` — schema definition (schema.toml + forge.wit).
  Ships with the plugin but also installable independently.
- `forge_issues`/`forge_prs` materialized views in patina.db — these are
  projections created by scrape from events.db. The projection logic is
  protocol (materializing events into queryable tables), not domain.
- Event types `forge.issue` and `forge.pr` in events.db — the data stays,
  only the ingestion path changes.

## ForgeWriter Decision

`ForgeWriter` (fork repo, create repo) is a write-to-GitHub capability,
not data ingestion. Options:

1. **Move to plugin** — the forge plugin handles reads AND writes to
   its source. Plugin becomes bidirectional.
2. **Separate extension** — ForgeWriter becomes its own extension plugin
   for GitHub operations.
3. **Drop it** — ForgeWriter is rarely used. Remove and add back if needed.

**Lean toward option 1.** The forge plugin is the GitHub specialist.
If it can read issues, it should be able to fork repos too. Uses the
same `host/http` + credentials. But ForgeWriter is NOT an exit criterion
— data ingestion is the priority.

## Key Files

**Current (to be extracted):**
- `src/forge/mod.rs` — `ForgeReader` trait, platform detection
- `src/forge/types.rs` — Issue, PullRequest, Comment types
- `src/forge/github/internal.rs` — `gh` CLI calls, JSON parsing
- `src/forge/sync/internal.rs` — sync engine, rate limiting, background
- `src/forge/writer.rs` — `ForgeWriter`, `gh` CLI write operations
- `src/commands/scrape/forge/mod.rs` — scrape integration, materialized views
- `.patina/schemas/forge/schema.toml` — fact definitions
- `.patina/schemas/forge/forge.wit` — WIT record types

**Plugin infrastructure (must exist first):**
- `wit/deps/patina-host/host.wit` — needs `emit` interface added
- `src/plugin/internal/host_support.rs` — host_emit implementation
- `src/plugin/internal/mod.rs` — needs `role` field in PluginManifest

## Open Questions

1. **Pagination without `gh`.** `gh` handles pagination internally
   (`--limit 100` fetches multiple pages). With raw HTTP, the plugin
   must follow GitHub's `Link` headers for pagination. Standard pattern
   but more code than the `gh` wrapper.

2. **Rate limit handling.** GitHub returns `X-RateLimit-Remaining`
   headers. Should the plugin read these and adapt, or keep the fixed
   750ms delay? Adaptive is better but more complex. **Lean toward:
   keep fixed delay initially, optimize later.**

3. **Schema migration.** Schema currently lives in `.patina/schemas/forge/`.
   When the plugin ships its schema, does `patina plugin install` copy
   the schema to `.patina/schemas/`? Or does the host resolve schemas
   from plugin directories? The schema installation mechanism needs
   design (see [[spec-host-emit-wit]] for validation requirements).

4. **Projection table ownership.** Who creates `forge_issues`/`forge_prs`
   materialized views after extraction? Options: (a) scrape creates them
   when it sees `forge.*` events, (b) schema.toml declares projections
   and scrape materializes them generically. Option (b) is more
   extensible — any schema could declare projections.
