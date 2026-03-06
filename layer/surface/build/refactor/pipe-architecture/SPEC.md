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
beliefs:
- wit-is-contract-wasm-is-one-runtime
- patina-is-domain-agnostic-knowledge-system
exit_criteria:
- id: pipe-interface-defined
  text: WIT-defined pipe interface (connect, fetch, health) independent of WASM/native runtime
  checked: false
- id: secrets-are-user-level
  text: Single secrets system manages all credentials — `patina auth` with OAuth device flow, not per-pipe manual setup
  checked: false
- id: destination-configures-pipe
  text: Projects, lakes, and blocks configure what data to pull from a pipe — the pipe doesn't decide
  checked: false
- id: github-pipe-works
  text: GitHub pipe replaces current forge WASM plugin, same data, better auth UX
  checked: false
---
# refactor: Pipe Architecture — Data Drivers, Not Plugins

> Connectors are reusable data drivers (pipes), not plugins. Pipes know
> how to talk to a source. Destinations (project, lake, block) configure
> what data flows through. Secrets are user-level infrastructure.

## Context

The forge-plugin-extraction (sessions 1-3) proved that external data can
flow through host_emit into events.db with provenance tracking. But the
audit revealed a category error: we jammed a data driver into a plugin
system designed for domain logic.

**Plugins are for opinions** — the spec lifecycle system, grammars that
parse files, CLI extensions, apps with workflow. They add behavior.

**Pipes are for plumbing** — GitHub, Slack, RSS, Google Workspace. They
move data from external sources into Patina's event system. They don't
have opinions. They don't add commands. They're drivers.

The distinction matters because:
- A pipe should work across projects without per-project installation
- A pipe's credentials are user-level, not project-level
- A pipe's capabilities are fixed by the source API, not by user choice
- Multiple destinations (project, lake, block) share the same pipe

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

**Host infrastructure** (solid, reusable by pipes):
- `host_emit` → schema-validated event emission to events.db
- `host_http` → domain-allowlisted HTTP with credential injection
- CQRS projection → events.db → patina.db materialized views
- FTS5 indexing from projection tables
- WASM isolation via wasmtime with empty WASI context

## Target State

### Three layers

```
┌─────────────────────────────────────────────┐
│  Secrets (user-level)                        │
│  patina auth login github → OAuth device flow│
│  patina auth login slack  → OAuth device flow│
│  One credential store, all pipes share it    │
└──────────┬──────────────────────────────────┘
           │ credentials by source name
┌──────────▼──────────────────────────────────┐
│  Pipes (Mother-level, one per source)        │
│  github-pipe: GitHub REST/GraphQL            │
│  slack-pipe: Slack API                       │
│  rss-pipe: RSS/Atom feeds                    │
│  Installed once, available to all projects   │
└──────────┬──────────────────────────────────┘
           │ data (filtered by destination config)
     ┌─────┼─────────┐
     ▼     ▼         ▼
  Project  Data Lake  Data Block
  config   config     config
  says     says       says
  what     what       what
  it       it         it
  wants    wants      wants
```

### Pipe interface (WIT-defined)

A pipe implements a WIT interface regardless of runtime:

- `connect(config) → result` — validate config, test connectivity
- `fetch(filter) → stream<fact>` — pull data matching filter
- `health() → status` — is the source reachable?
- `capabilities() → list<fact-type>` — what data types can this pipe provide?

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
| Purpose | Move data in | Add behavior |
| Trust | Infrastructure (like git) | Third-party (like Obsidian plugins) |
| Sandbox | Credential-scoped, not process-sandboxed | WASM sandboxed |
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
events. It just won't be a WASM plugin — it'll be a pipe with better
auth and shared across destinations.

## Open Questions

- **WASM for community pipes?** If someone publishes a pipe for an
  obscure API, should it run in WASM for safety? Maybe pipes have two
  tiers: core/trusted (native) and community (WASM sandboxed).
- **Streaming sources.** WebSockets, SSE, real-time feeds. Pipes need
  to handle both poll (GitHub) and push (Slack real-time) models. Mother
  manages the connection lifecycle.
- **Filter language.** How expressive does destination filtering need to
  be? Simple field matching? JSONPath? SQL-like predicates?
- **Pipe vs lake-registry overlap.** The lake-registry spec was about
  lake metadata in graph.db. This spec is about the pipe layer underneath.
  They may merge or one may subsume the other.

## Steps

1. Design WIT pipe interface (connect, fetch, health, capabilities)
2. Design `patina auth` — OAuth device flow, credential store, refresh
3. Implement GitHub pipe using existing forge code as starting point
4. Design destination configuration format
5. Wire Mother to manage pipe scheduling per destination
6. Migrate forge-plugin-extraction to pipe architecture
7. Delete old WASM forge plugin (or keep as community example)
