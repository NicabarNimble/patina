---
type: refactor
id: pipe-architecture
status: draft
created: 2026-03-06
sessions:
  origin: 20260305-170212
related:
- forge-plugin-extraction
- lake-registry
- core-extraction
- continuous-operation
- persona-federation
beliefs:
- wit-is-contract-wasm-is-one-runtime
- patina-is-domain-agnostic-knowledge-system
- pipes-are-processes-not-wasm
- host-proxied-io-is-the-security-model
- mother-holds-connections-pipes-transform
- pipe-protocol-is-transport-agnostic
- persona-keypair-is-node-identity
- wit-defines-pipe-contract-not-runtime
exit_criteria:
- id: pipe-sdk-exists
  text: "`patina-pipe` SDK crate with Pipe trait, run() orchestrator, Fact/PipeConfig types, and `patina pipe new` scaffold command"
  checked: false
- id: pipe-interface-defined
  text: Pipe protocol defined — stdin config, stdout newline-delimited JSON facts, stderr logs — with WIT type definitions for fact shapes
  checked: false
- id: secrets-are-user-level
  text: Single secrets system manages all credentials — `patina auth` with OAuth device flow, not per-pipe manual setup
  checked: false
- id: destination-configures-pipe
  text: Projects, lakes, and blocks configure what data to pull from a pipe — the pipe doesn't decide
  checked: false
- id: github-pipe-works
  text: GitHub pipe replaces current forge WASM plugin, same data, better auth UX — built on patina-pipe SDK
  checked: false
---
# refactor: Pipe Architecture — Data Flow Primitive

> A pipe connects a source to a destination, secured by a secret,
> described by a WIT contract. Same protocol whether the source is
> GitHub's API or a Patina lake. Pipes are the universal data flow
> primitive — the connective tissue across the entire data architecture.

## Context

The forge-plugin-extraction (sessions 1-3) proved that external data can
flow through host_emit into events.db with provenance tracking. But the
audit revealed a category error: we jammed a data driver into a plugin
system designed for domain logic.

**Plugins are for opinions** — the spec lifecycle system, grammars that
parse files, CLI extensions, apps with workflow. They add behavior.

**Pipes are for plumbing** — connecting any source to any destination.
External sources (GitHub, Slack, RSS), internal sources (lakes, blocks),
transforms (app layer enrichment). They don't have opinions. They don't
add commands. They move and transform data.

The distinction matters because:
- A pipe should work across projects without per-project installation
- A pipe's credentials are user-level, not project-level
- A pipe's capabilities are fixed by the source, not by user choice
- Multiple destinations share the same pipe
- The same protocol works for external ingestion and internal routing

## Current State

**forge-plugin-extraction** built a WASM plugin that:
- Talks to GitHub REST API via host_http
- Emits forge.issue and forge.pr events via host_emit
- Requires manual PAT creation + vault + secret-grants.toml (4 steps)
- Is installed per-project as a WASM child plugin
- Has no concept of shared pipes or destination filtering

**Secrets infrastructure** (mature, but missing auth UX):
- Age-encrypted vault (`~/.patina/vault.age`) with macOS Keychain +
  Touch ID integration
- Global + project-scoped secrets with dual-storage strategy
- Session caching via Mother child (10min TTL, avoids Touch ID spam)
- Secret scanning (pre-commit check for leaked secrets)
- Claude OAuth precedent: `patina secrets setup-claude` stores token
  and injects on adapter launch — this is the pattern to generalize
- CLI: `patina secrets add/remove/run/check/audit`
- Missing: OAuth device flow, `patina auth` command, credential
  metadata (scopes, expiry, provider type)

**Plugin credential model** (works but friction):
- `CredentialMapping` struct: maps domain → secret name + injection
  location (currently Bearer only)
- `check_secret_grant()`: reads `~/.patina/plugin-config/secret-grants.toml`,
  deny-by-default
- `resolve_credential()`: grants check → vault decrypt → inject header
- `leak_check()`: scans response for leaked credential values
- No `patina plugin grant` CLI command — user hand-edits TOML file

**Host infrastructure** (solid, patterns reusable by pipes):
- `host_emit` → schema-validated event emission to events.db
- `host_http` → domain-allowlisted HTTP with credential injection
- CQRS projection → events.db → patina.db materialized views
- FTS5 indexing from projection tables
- Security logic in `host_support.rs` — domain allowlist, credential
  injection, leak check — all host-side, reusable for process-based pipes

## Target State

### Data architecture

Pipes connect every layer. Secrets secure every connection.

```
External Sources (GitHub, Slack, RSS, APIs...)
  │ pipes (ingest)
  ▼
Data Lakes (Parquet, lakehouse-managed, raw/complete)
  │ pipes (transform/filter)
  ▼
App Layer (transforms, enrichment, embeddings)
  │ pipes (structure/reduce)
  ▼
Data Blocks (embeddings, curated datasets)
  │ pipes (serve/query)
  ▼
Apps / Projects (action, workflows, UI)
```

Every arrow is a pipe. Every connection has a secret. Every contract
is WIT-defined. The pipe protocol doesn't change — config in, facts
out — only the source type, destination type, and transform logic
differ.

### Secrets layer

```
┌─────────────────────────────────────────────┐
│  Secrets (user-level)                        │
│  patina auth login github → OAuth device flow│
│  patina auth login slack  → OAuth device flow│
│  One credential store, all pipes share it    │
└─────────────────────────────────────────────┘
```

### Pipe interface (stdio protocol)

A pipe is a native binary that communicates over stdio:

- **stdin**: JSON config object (credentials, params, fetch request)
- **stdout**: newline-delimited JSON facts (one fact per line)
- **stderr**: structured logs (human-readable, not parsed by Mother)

WIT defines the type contract — fact shapes, capability declarations,
config schema — but pipes are NOT WASM components. They're regular
processes spawned by Mother. Any language that reads stdin and writes
stdout can be a pipe.

### Pipe SDK (`patina-pipe` crate)

The SDK is a blocker for building any pipe. It provides:

- `Pipe` trait: `capabilities()`, `fetch()`, `health()`
- `run()` orchestrator: handles stdin/stdout/stderr protocol
- Types: `Fact`, `PipeConfig`, `Capabilities`, `Status`
- WIT-generated type definitions for fact shapes
- Signal handling for stream mode (graceful shutdown)
- `patina pipe new <name>` scaffold command

Pipe authors implement the trait. The SDK handles everything else.
Same pattern as MCP SDKs — you don't hand-parse JSON-RPC, you
implement a handler.

### Pipe lifecycle modes

- **Poll**: spawn → fetch → emit facts → exit. Schedule-driven
  (cron-like: hourly, daily, on-scrape). Ephemeral process.
- **Stream**: spawn → stay alive → emit facts continuously. Mother
  monitors health, restarts on crash. Long-lived process.
- **Manual**: one-shot on user command (`patina pipe run github`).
  For testing, backfill, debugging. Same binary, same protocol.

### Destination configuration

A project, lake, or block specifies what it wants:

```toml
[sources.github]
pipe = "github"
auth = "github:user"          # references patina auth credential
repo = "NicabarNimble/patina"
types = ["issues", "prs"]
schedule = "on-scrape"        # or "hourly", "daily", "manual"

[sources.slack]
pipe = "slack"
auth = "slack:workspace"
channels = ["#dev", "#incidents"]
schedule = "hourly"
```

### Secrets as core infrastructure

```
patina auth login github   → browser OAuth → token in vault
patina auth login slack    → browser OAuth → token in vault
patina auth status         → show all credentials, expiry, which pipes use them
patina auth refresh github → re-auth if expired
```

No manual PAT creation. No editing TOML files. No per-plugin grants —
pipes are trusted infrastructure, not untrusted third-party code.

## Key Distinctions

| | Pipes | Plugins |
|---|---|---|
| Purpose | Connect source → destination (data flow) | Add behavior (domain logic) |
| Runtime | Native process over stdio | WASM component in wasmtime |
| Trust | Infrastructure (like git) | Third-party (like Obsidian plugins) |
| Sandbox | OS sandbox (sandbox-exec/Landlock) — all pipes, ~0ns runtime cost | WASM sandboxed |
| Install scope | User-level (Mother) | Project-level |
| Auth | `patina auth` (OAuth, shared) | `secret-grants.toml` (manual, per-plugin) |
| Examples | GitHub, Slack, RSS, Google | Spec system, grammars, doctor, apps |
| Configuration | Destination says what it wants | Plugin decides what it does |

## What Happens to forge-plugin-extraction

The forge WASM plugin proved:
- host_emit works (events.db, provenance, schema validation)
- The security model works (domain allowlist, credential injection, leak check)
- The projection pipeline works (CQRS, FTS5)

These infrastructure pieces survive. The pipe architecture reframes
*how* the connector is packaged and configured, not what it does
internally. The GitHub pipe will still emit forge.issue and forge.pr
events. It becomes a native Rust binary that reads config from stdin,
calls the GitHub API directly via reqwest, and writes newline-delimited
JSON facts to stdout. Mother reads stdout and writes to events.db.
Normal Rust development: `cargo run`, `cargo test`, `dbg!()` — no
WASM build step, no cross-compilation.

## Resolved Questions (from session 20260305-224446)

1. **WASM for community pipes?** → No. All pipes are OS-sandboxed native
   processes. macOS sandbox-exec and Linux Landlock provide kernel-enforced
   isolation at ~2ms startup, ~0ns runtime — same pattern as Chrome
   renderer processes. No trusted/untrusted tiers. One model for all pipes.
   [[pipes-are-processes-not-wasm]], [[host-proxied-io-is-the-security-model]]

2. **Streaming sources?** → Mother holds external connections (WebSockets,
   webhooks, polling). Pipes are stateless transforms — data in via stdin,
   facts out via stdout. For Slack real-time: Mother holds the WebSocket,
   buffers messages, feeds batches to slack-pipe over stdin. Pipe doesn't
   know about WebSocket. Same interface for all source transports.
   [[mother-holds-connections-pipes-transform]]

3. **Filter language?** → Pipe config params handle source-side filtering
   (which repos, which channels). Destination-side filtering is projection
   — views over the lake's events.db. No custom filter language needed.

4. **Pipe vs lake-registry overlap?** → Lake is a destination type. A pipe
   feeds data into a lake's events.db. Lake-registry manages lake metadata
   (what sources, what schemas, sync state). They're complementary layers,
   not competing specs.

5. **Transport?** → stdio for local pipes (default). HTTP+SSE for remote
   pipes on a VPS. Streamable HTTP for shared pipes serving multiple
   Mother instances. Same message format across all transports — protocol
   is transport-agnostic. Build stdio now, don't hardcode assumptions.
   [[pipe-protocol-is-transport-agnostic]]

6. **Fan-out?** → Mother spawns N instances of the same pipe binary with
   different configs. One github-pipe binary serves multiple destinations
   (project, lake, block). Lake-as-source pattern: fetch everything once
   into lake, project many views — no re-fetching.

## Open Questions

1. **Pipe discovery.** How do users find and install community pipes?
   Registry like crates.io? GitHub releases? Manual download?

2. **Schema ownership.** Today the forge schema ships with the plugin.
   With pipes, does the schema ship with the pipe or is it installed
   independently? Lean toward: ships with the pipe (same as today).

3. **Pipe versioning.** When a pipe's output format changes, how do
   projection tables migrate? Schema version in event metadata?

4. **Multi-provider pipes.** Is "github-pipe" one pipe for all GitHub
   instances, or is "github-enterprise" a separate pipe? Lean toward:
   one pipe, configured with base_url per destination.

## Steps

1. **Build `patina-pipe` SDK crate** (blocker for all pipes):
   - `Pipe` trait, `run()` orchestrator, types (`Fact`, `PipeConfig`,
     `Capabilities`, `Status`)
   - stdin/stdout/stderr protocol handling, NDJSON serialization
   - Signal handling, logging, health check protocol
   - WIT type definitions for fact shapes
2. Build `patina pipe new <name>` scaffold command — generates
   Cargo.toml, main.rs, pipe.toml from template
3. Define pipe.toml manifest format (provider, data-types, domains,
   schema package, lifecycle mode)
4. Define sources.toml destination config format (pipe, auth, params,
   types, schedule)
5. Build Mother pipe manager — spawn process, read stdout, write to
   events.db, track sync state, handle lifecycle modes
6. Build github-pipe binary — first pipe, built on SDK. Migrate from
   forge plugin code, direct reqwest, reads stdin, writes facts to stdout
7. Build `patina pipe run/health/list` CLI commands
8. Build `patina auth login` — OAuth device flow, credential store,
   `patina auth status/refresh/revoke`
9. Wire scheduling — poll mode (cron-like intervals), stream mode
   (always-on with health monitoring), manual mode (one-shot)
