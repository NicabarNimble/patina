---
type: feat
id: github-connector
status: draft
created: 2026-03-06
blocked_by:
- pipe-native-transport
sessions:
  origin: 20260306-171859
related:
- pipe-architecture
- forge-plugin-extraction
beliefs:
- patina-is-domain-agnostic-knowledge-system
exit_criteria:
- id: connector-speaks-pipe
  text: github-connector binary speaks pipe protocol over stdio — responds to pipe/initialize, pipe/fetch, pipe/health, pipe/shutdown
  checked: false
- id: emits-github-facts
  text: Emits github.issue and github.pr facts with its own schema — same data shape as existing forge facts, new schema namespace
  checked: false
- id: src-forge-deleted
  text: src/forge/ deleted from core — completes forge-plugin-extraction EC4 (original gh CLI wrapper removed)
  checked: false
- id: plugins-forge-coexists
  text: plugins/forge/ continues to run as WASM child (forge.* schema) — coexists with native github-connector (github.* schema) to prove both runtimes
  checked: false
- id: mother-run-works
  text: '`patina mother run github` triggers the connector via pipe protocol'
  checked: false
---
# feat: GitHub Connector — First Native Child on Pipe Architecture

> First native child on pipe architecture. Proves the model end-to-end:
> native binary, pipe protocol over stdio, OS sandbox, real external
> API. Replaces the old gh CLI wrapper. Coexists with the WASM forge
> plugin to prove both runtimes under mother-broker.

## Problem

GitHub data ingestion exists in two forms, both problematic:

1. **src/forge/** (533+442+708+604 = 2,287 LOC) — gh CLI wrapper.
   Requires gh installed, uses subprocess spawning, complex staging
   pipeline. This is the original code that should have been
   extracted already.

2. **plugins/forge/** — WASM plugin via patina-sdk. Proved the
   host_emit infrastructure works (EC1/EC2 of forge-plugin-extraction).
   But WASM transport adds complexity (host proxied I/O, domain
   allowlist, credential injection through host functions).

Neither speaks pipe protocol. Neither uses the connection model.
Neither is managed by Mother as a broker.

## Solution

Build `github-connector` as a native Rust binary that:
- Speaks pipe protocol over stdio (using patina-pipe crate)
- Makes direct `reqwest` HTTP calls to api.github.com
- Runs in OS sandbox (macOS sandbox-exec)
- Emits github.issue and github.pr facts (own schema, same data shape)
- Receives credentials via pipe/initialize (not env, not files)

**Credential source is independent of this spec.** Mother reads
credentials from vault and passes them via pipe/initialize (part of
pipe-native-transport). Credentials can be stored via manual
`patina secrets add github-token` (existing workflow) or via
`patina connect github` OAuth flow ([[spec-patina-connect]]). This
spec does not depend on patina-connect — manual PAT is sufficient.

The code migrates from `plugins/forge/src/github.rs` — the GitHub
REST API client (pagination, issue/PR fetching, JSON conversion) is
proven and tested. `host_http::get` becomes `reqwest::get`.
`host_emit::emit_fact` becomes `emitter.emit()`.

```
children/github-connector/
  Cargo.toml          # depends on patina-pipe, reqwest, serde
  child.toml          # manifest: connector, native, poll
  src/
    main.rs           # Child trait impl + main()
    github.rs         # GitHub REST API client (from plugins/forge/)
```

## Steps

1. Create `children/github-connector/` binary crate
2. Write child.toml manifest (type=connector, runtime=native,
   lifecycle=poll, domains=[api.github.com])
3. Implement Child trait: capabilities (issues, prs, incremental),
   fetch (paginated GitHub API), health (rate limit check)
4. Migrate `plugins/forge/src/github.rs` API client — replace
   host_http with reqwest, host_emit with FactEmitter
5. Create `.patina/schemas/github/schema.toml` with github.issue and
   github.pr fact types (same data shape as forge, new schema namespace)
6. Wire Mother-side: `patina mother run github` spawns connector,
   sends pipe/initialize with credentials, dispatches pipe/fetch
7. **Parity verification:** run both old (`patina scrape forge`) and
   new (`patina mother run github`) against same repo, compare data
   shape. Schema names differ (forge.* vs github.*), data shape should
   match. Document any differences.
8. Delete `src/forge/` from core (2,287 LOC)
9. Delete `src/commands/scrape/forge/` subcommand handler (604 LOC)
10. plugins/forge/ stays — WASM child coexists with native child to
    prove both runtimes under mother-broker

## Key Files

**Migrate from:**
- `plugins/forge/src/github.rs` — GitHub REST API client (450 LOC)
- `plugins/forge/src/lib.rs` — ForgeChild impl pattern
- `plugins/forge/plugin.toml` — manifest, becomes child.toml

**Delete:**
- `src/forge/` — old gh CLI wrapper
- `src/commands/scrape/forge/` — old scrape subcommand

**Reference:**
- [[spec-pipe-architecture]] DESIGN.md §2.5 (connector example)
- `src/plugin/internal/host_support.rs` — emit validation to verify
  fact format compatibility

## Non-Goals

- GraphQL API support (REST is sufficient, same as existing code)
- Comment/review fetching changes (keep current behavior)
- New fact types beyond github.issue and github.pr
- OAuth device flow (that's [[spec-patina-connect]])
- Fan-out routing (that's [[spec-mother-broker]])
