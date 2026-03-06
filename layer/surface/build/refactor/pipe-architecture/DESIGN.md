# Design: Pipe Architecture — Data Flow Primitive

## The Core Insight

Session 3 audit of forge-plugin-extraction revealed a category error.
We built a GitHub data driver and packaged it as a WASM plugin. But
plugins are for *opinions and behavior* — the spec lifecycle, grammars,
CLI extensions. Data drivers are *infrastructure* — they move data
between sources and destinations. Same relationship as git
(infrastructure, always present) vs the spec system (domain logic,
optionally installed).

A pipe connects a source to a destination, secured by a secret,
described by a WIT contract. The source can be external (GitHub API)
or internal (a Patina lake). The destination can be a lake, a block,
an app, or a project. Same protocol everywhere — MCP-like design
with WIT contracts.

The forge WASM plugin proved the plumbing works:
- host_emit → events.db with provenance and schema validation
- host_http → domain-allowlisted, credential-injected HTTP
- CQRS projection → events.db → patina.db materialized views
- FTS5 indexing from projection tables

What it got wrong:
- Packaged as per-project WASM plugin (should be user-level driver)
- Manual PAT creation (should be OAuth device flow)
- Per-plugin secret grants (should be shared credential store)
- No concept of multiple destinations sharing one pipe
- Plugin sandbox overhead on pure I/O (every HTTP call crosses boundary)

## Three Layers

### Layer 1: Secrets (`patina auth`)

**What exists today:**
- Age-encrypted vault (`~/.patina/vault.age`) with Keychain + Touch ID
- Global + project-scoped secrets
- Session caching via Mother child (10min TTL, avoids Touch ID spam)
- Secret scanning (pre-commit check for leaked secrets)
- CLI: `patina secrets add`, `remove`, `run`, `check`, `audit`
- Claude OAuth precedent: `patina secrets setup-claude` stores OAuth
  token, injects as env var on adapter launch

**What's missing:**
- `patina auth login <provider>` — OAuth device flow per source
- `patina auth status` — show all credentials, expiry, which pipes use them
- `patina auth refresh <provider>` — re-auth on expiry
- Credential metadata: scopes, created_at, expires_at, last_used_at
- Provider-specific OAuth registration (GitHub app, Slack app, etc.)

**Design approach:**
Build on existing secrets infrastructure. `patina auth` is a UX layer
on top of `patina secrets`. The vault stays the same. The addition is:
an OAuth device flow that acquires the token automatically (browser
popup, user approves, token stored) instead of manual PAT creation.

The `setup-claude` pattern is the precedent — it already stores an
OAuth token in the global vault and injects it on launch. Generalize
this to multiple providers:

```
patina auth login github
→ Opens browser to GitHub device authorization
→ User approves
→ Token stored: patina secrets add --global github:user
→ Metadata stored: scopes, expiry, provider type

patina auth login slack
→ Opens browser to Slack OAuth
→ Same flow, different provider
```

**Credential naming convention:**
`<provider>:<identity>` — e.g., `github:user`, `slack:workspace`,
`google:account@email.com`. Pipes reference these by name.

### Layer 2: Pipes (Mother-managed native processes)

**What a pipe is:**
A pipe is a native binary that connects a source to a destination.
Sources can be external (GitHub API, Slack, RSS) or internal (a Patina
lake, a data block). A pipe knows:
- How to authenticate with its source (OAuth, API key, internal token)
- How to paginate/stream from the source
- How to rate limit
- What data types it can provide (issues, PRs, embeddings, curated sets)
- How to filter and fetch on demand

A pipe does NOT know:
- Which destination wants what data
- Where to store the data
- How to schedule itself
- Anything about other pipes or the broader data flow

**Execution model (MCP server):**

Pipes are MCP servers. Mother is an MCP client. The protocol is
JSON-RPC 2.0 — not MCP-like, the actual protocol. This gives us
bidirectional communication, multiple calls per session, progress
notifications, and the full transport stack (stdio, HTTP+SSE,
Streamable HTTP) without inventing anything.

```
Mother (MCP client)              github-pipe (MCP server)
  │                                 │
  │──── initialize ────────────────▶│
  │◀─── capabilities ──────────────│
  │                                 │
  │──── tools/call: fetch ─────────▶│
  │     {                           │
  │       "persona": "dev-nick",    │
  │       "auth": { "token": "..." },
  │       "params": { "owner": "NicabarNimble", "repo": "patina" },
  │       "types": ["issues", "prs"],
  │       "since": "2026-03-01T00:00:00Z"
  │     }                           │
  │                                 │── reqwest GET /repos/.../issues
  │◀─── notification: progress ────│   (50 issues fetched)
  │                                 │── reqwest GET /repos/.../pulls
  │◀─── notification: fact ────────│   {"schema":"forge","type":"issue",...}
  │◀─── notification: fact ────────│   {"schema":"forge","type":"pr",...}
  │◀─── result: { fetched: 73 } ───│
  │                                 │
  │──── tools/call: health ────────▶│  (Mother can call again without respawn)
  │◀─── result: { ok: true } ──────│
  │                                 │
  │──── shutdown ──────────────────▶│
  │◀─── exit(0) ───────────────────│
  │                                 │
  │── write facts to events.db      │
```

Key difference from previous design: Mother can call `fetch()` multiple
times on the same running pipe. No respawn per fetch. Connection pooling,
rate limit state, and cached auth persist across calls.

**github-pipe code sketch:**

```rust
// pipes/github-pipe/src/main.rs
use patina_pipe::{Pipe, serve, Fact, PipeConfig, Capabilities, Status};

struct GitHubPipe;

impl Pipe for GitHubPipe {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            provider: "github",
            data_types: vec!["issues", "prs", "comments", "reviews"],
            supports_incremental: true,
            supports_streaming: false,
        }
    }

    fn fetch(&self, config: &PipeConfig) -> Result<Vec<Fact>> {
        let client = GitHubClient::new(&config.auth, &config.params)?;
        let mut facts = Vec::new();

        for issue in client.fetch_issues(config.since)? {
            facts.push(Fact::new("forge", "issue", &issue)?);
        }
        for pr in client.fetch_pull_requests(config.since)? {
            facts.push(Fact::new("forge", "pr", &pr)?);
        }

        Ok(facts)
    }

    fn health(&self, config: &PipeConfig) -> Result<Status> {
        let client = GitHubClient::new(&config.auth, &config.params)?;
        client.check_rate_limit()
    }
}

fn main() -> Result<()> {
    serve(GitHubPipe)  // MCP server — handles everything
}
```

Normal Rust development: `cargo run`, `cargo test`, `dbg!()`. No WASM
build step, no cross-compilation, no host function stubs for testing.
The `GitHubClient` code migrates from `plugins/forge/src/github.rs`
with `host_http` calls replaced by direct `reqwest` calls.

`serve()` handles: MCP protocol, JSON-RPC dispatch, transport
negotiation, fact signing (persona keypair), content addressing
(blake3 hash), persona scoping, progress notifications, graceful
shutdown. The pipe author never touches any of this.

**WIT as type contract (not calling convention):**

WIT defines the types — fact shapes, capability declarations, config
schema — but pipes are NOT WASM components. WIT serves the same role
as protobuf or JSON Schema: a language-agnostic type definition that
generates Rust structs, TypeScript types, or Python dataclasses. The
pipe.wit file defines what a valid fact looks like. The pipe binary
serializes facts as JSON matching those types. Mother validates facts
against the WIT-derived schema before writing to events.db.

**Pipe lifecycle modes:**

- **Poll**: spawn → fetch → emit facts → exit. Schedule-driven
  (cron-like: hourly, daily, on-scrape). Ephemeral process. GitHub
  issues, RSS feeds, periodic API scrapes.
- **Stream**: spawn → stay alive → emit facts continuously. Mother
  monitors health, restarts on crash. Long-lived process. For sources
  with native streaming APIs (SSE, long-poll). Pipe manages its own
  connection.
- **Manual**: one-shot on user command (`patina pipe run github`).
  For testing, backfill, debugging. Same binary, same protocol.

For real-time sources where the external connection is complex
(WebSockets, webhooks), Mother holds the connection and feeds data
to the pipe. Slack example: Mother holds the Slack WebSocket, buffers
messages, spawns slack-pipe with the batch on stdin. The pipe doesn't
know about WebSocket — it transforms JSON in, facts out.

**Fan-out:**

Mother spawns N instances of the same pipe binary with different
configs. One github-pipe binary serves multiple destinations:

```
Mother
  ├── github-pipe (config: NicabarNimble/patina, types: [issues, prs])
  │     → project events.db
  ├── github-pipe (config: NicabarNimble/*, types: [issues, prs, releases])
  │     → org-lake events.db
  └── github-pipe (config: NicabarNimble/patina, types: [security-advisories])
        → security-block events.db
```

**All pipes are OS-sandboxed:**

Every pipe runs in an OS sandbox — no trusted/untrusted tiers.

- **macOS**: `sandbox-exec` with a profile that denies filesystem
  access and process spawning. Network allowed for declared domains
  (from pipe.toml manifest). ~2ms startup overhead, ~0ns runtime.
- **Linux**: Landlock LSM restricts filesystem access and process
  creation. Same model, same overhead.

This is the Chrome renderer process pattern: the sandbox doesn't
provide the security (the protocol does), it prevents bypass. A
compromised pipe binary can't read arbitrary files, access secrets
on disk, or spawn subprocesses. It makes HTTP calls to its declared
domains and communicates with Mother over inherited stdio.

Performance comparison:
- OS sandbox startup: ~2ms (one-time per process)
- OS sandbox runtime: ~0ns (kernel-enforced, no per-call overhead)
- WASM boundary crossing: ~1ms per host function call
- For a GitHub sync with 50 API calls: 0ms (OS) vs 50ms (WASM)

**Three-layer security model:**

1. **Protocol enforcement** (always): Mother validates that facts
   match declared schemas. Pipe can only emit what its manifest allows.
   Credentials are passed via stdin config, not environment or files.

2. **Capability manifest** (always): pipe.toml declares what domains
   the pipe needs, what schemas it emits, what auth it requires. Mother
   refuses to run a pipe that requests undeclared resources.

3. **OS sandbox** (all pipes): kernel-enforced process isolation.
   Pipe can't make network calls, read filesystem, or spawn processes.
   All I/O goes through inherited stdio file descriptors.

Future: UCAN capability tokens for scoped credential delegation —
persona keypair signs a token granting specific API scopes to a
specific pipe for a specific duration.

**Pipe packaging:**

A directory with `pipe.toml` manifest and a native binary. Installed
at user level (`~/.patina/pipes/`) not project level.

```
~/.patina/pipes/github-pipe/
├── pipe.toml        # manifest: provider, domains, schemas, lifecycle
└── github-pipe      # native binary (or symlink to cargo build)
```

### Pipe SDK (`patina-pipe` crate)

The SDK is the foundation — every pipe depends on it. No pipe is built
without the SDK. The pipe is an MCP server. The SDK handles MCP protocol,
transport, signing, hashing, persona isolation. The pipe author implements
`fetch()` and nothing else.

**Core trait:**

```rust
// patina-pipe/src/lib.rs

/// The contract every pipe implements. This never changes.
pub trait Pipe {
    /// What this pipe can provide.
    fn capabilities(&self) -> Capabilities;

    /// Fetch data matching the config. Called by serve() on each
    /// MCP tools/call request. May be called multiple times per session.
    fn fetch(&self, config: &PipeConfig) -> Result<Vec<Fact>>;

    /// Test connectivity with current credentials.
    fn health(&self, config: &PipeConfig) -> Result<Status>;
}

/// MCP server entry point. Pipe authors call this from main().
/// Handles everything: protocol, transport, signing, persona.
pub fn serve<P: Pipe>(pipe: P) -> Result<()> {
    // 1. Negotiate transport (stdio, HTTP+SSE, Streamable HTTP)
    // 2. MCP initialize handshake
    // 3. Listen for JSON-RPC requests
    // 4. Dispatch to pipe.fetch() / pipe.health() / pipe.capabilities()
    // 5. Sign each fact with persona keypair (automatic)
    // 6. Hash each fact with blake3 (content addressing)
    // 7. Send facts as MCP notifications
    // 8. Handle SIGTERM gracefully (stream mode)
}
```

**SDK-provided types:**

```rust
/// What Mother sends in each MCP tools/call request.
pub struct PipeConfig {
    pub persona: String,            // "dev-nick" — scoped, can't cross
    pub auth: Option<AuthConfig>,   // credentials from vault
    pub params: serde_json::Value,  // pipe-specific params
    pub types: Vec<String>,         // which data types to fetch
    pub since: Option<DateTime>,    // incremental: only after this
    pub limit: Option<u32>,         // max items
}

pub struct AuthConfig {
    pub token: String,              // resolved from vault
    pub provider: String,           // "github", "slack", etc.
}

/// What the pipe returns. SDK signs and hashes before sending.
pub struct Fact {
    pub schema: String,             // "forge", "slack", etc.
    pub fact_type: String,          // "issue", "pr", "message"
    pub data: serde_json::Value,    // the payload
    // SDK adds before sending to Mother:
    //   content_hash: blake3 of data (dedup across pipes/nodes)
    //   signature: persona keypair signs fact (provenance)
    //   persona: which persona produced this fact
}

/// Health check result.
pub struct Status {
    pub ok: bool,
    pub message: String,
    pub latency_ms: Option<u64>,
}

/// What this pipe can do.
pub struct Capabilities {
    pub provider: String,
    pub data_types: Vec<String>,
    pub supports_incremental: bool,
    pub supports_streaming: bool,
}
```

**What the SDK handles (pipe authors don't touch):**

- MCP JSON-RPC 2.0 protocol (initialize, tools/call, notifications)
- Transport negotiation (stdio / HTTP+SSE / Streamable HTTP)
- Fact signing (persona keypair — automatic provenance)
- Content addressing (blake3 hash — automatic dedup)
- Persona scoping (pipe receives persona context, can't cross)
- Progress notifications (SDK sends progress to Mother)
- stderr logging (structured, timestamped)
- Signal handling (SIGTERM → graceful shutdown for stream mode)
- Error formatting (pipe errors → MCP error response)

**What pipe authors implement:**

- `fetch()` — the actual data fetching logic (reqwest calls, pagination,
  rate limiting, data mapping to Fact structs)
- `health()` — test connectivity (e.g., GET /rate_limit for GitHub)
- `capabilities()` — declare what the pipe can do (static)

**pipe.toml manifest:**

```toml
[pipe]
name = "github-pipe"
version = "0.1.0"
provider = "github"
lifecycle = "poll"           # poll | stream | manual

[capabilities]
data_types = ["issues", "prs", "comments", "reviews"]
supports_incremental = true

[domains]
allowed = ["api.github.com"]

[auth]
required = true
provider = "github"

[schemas.forge]
package = "patina:schema/forge@1.0.0"
```

**`patina pipe new <name>` scaffold:**

```
$ patina pipe new github

Created pipes/github-pipe/
├── Cargo.toml         # patina-pipe dependency, binary target
├── src/
│   └── main.rs        # Pipe trait stub, serve() call
└── pipe.toml          # manifest template (fill in provider, domains)

Next: edit src/main.rs, implement fetch(), then:
  cargo run              # test locally (stdio transport)
  patina pipe run github # run via Mother
```

**Deployment contexts:**

The same `impl Pipe` runs everywhere. The SDK adapts runtime and
transport. The pipe author writes `fetch()` once.

| Context | Runtime | Transport | Identity |
|---|---|---|---|
| Local (your machine) | Native process | stdio | Persona keypair |
| Remote (VPS) | Native process | HTTP+SSE | Same keypair |
| Edge (Cloudflare) | WASM Worker | Streamable HTTP | Same keypair |
| P2P (other nodes) | Native process | Iroh/HTTP | Node keypair |
| Shared (community) | Native or WASM | Streamable HTTP | Their persona |

WIT compiles to native (local) AND WASM (edge). Same contract, same
types, different binary target. `cargo build` for local, `cargo build
--target wasm32-wasi` for edge. Pipe code unchanged.

**Separation of concerns:**

```
            knows          knows           knows           knows
            source data    transport       routing         purpose
               │              │               │               │
             Pipe           Child           Mother         Consumer
               │              │               │               │
does:       transform      hold conn      orchestrate     project+act
            (fetch)        (WebSocket)    (spawn,route)   (query,index)
               │              │               │               │
doesn't:    store          parse data     transform       fetch
            connect        route          hold conns      know source
            sign facts     know schema    know format     know transport
            know persona   know persona   OWNS persona    scoped by persona
```

### Layer 3: Destinations (project, lake, block configuration)

**What a destination is:**
A destination is a consumer of pipe data. It specifies:
- Which pipe(s) to use
- Which credential to authenticate with
- What data to fetch (types, filters, repos/channels/feeds)
- When to fetch (schedule or trigger)

**Destination types:**

| Type | Scope | Example |
|---|---|---|
| Project | `.patina/` in a git repo | "issues + PRs from this repo" |
| Data lake | `~/.patina/lakes/<name>/` | "all repos from org X + org Y" |
| Data block | `~/.patina/blocks/<name>/` | "security-labeled PRs from repo Z" |

**Configuration format (project-level example):**

```toml
# .patina/sources.toml (or in patina.toml)

[sources.github]
pipe = "github"
auth = "github:user"
params = { owner = "NicabarNimble", repo = "patina" }
types = ["issues", "prs"]
schedule = "on-scrape"

[sources.slack]
pipe = "slack"
auth = "slack:myworkspace"
params = { channels = ["#dev", "#incidents"] }
types = ["messages"]
schedule = "hourly"
```

**Mother's role:**
Mother manages pipe lifecycle. For each destination:
1. Read source configuration (sources.toml)
2. Resolve credential from `patina auth` (vault decrypt, session cache)
3. Build stdin config JSON (credential + params + fetch request)
4. Spawn pipe binary in OS sandbox
5. Write config to pipe's stdin, close stdin
6. Read newline-delimited JSON facts from pipe's stdout
7. Validate facts against declared schema
8. Write valid facts to destination's events.db
9. Track last-sync timestamp for incremental fetches
10. Handle lifecycle: exit code for poll, health monitoring for stream

This replaces the current `patina plugin run patina-forge -- sync`
manual invocation.

## Migration Path from forge-plugin-extraction

The forge WASM plugin code is the starting point for the GitHub pipe.
What changes:

| Aspect | Current (WASM plugin) | Target (pipe) |
|---|---|---|
| Runtime | WASM in wasmtime | Native binary over stdio |
| Interface | `handle("sync", payload)` via host calls | stdin JSON config → stdout NDJSON facts |
| HTTP | `host_http` (crosses WASM boundary) | Direct `reqwest` (pipe owns HTTP) |
| Auth | Manual PAT + vault + grants TOML | `patina auth login github` |
| Scope | Per-project installation | User-level, shared |
| Config | JSON payload at runtime | TOML in destination config |
| Scheduling | Manual `plugin run` | Mother-managed per destination |
| Facts | Plugin calls host_emit directly | Pipe writes to stdout, Mother emits |
| Sandbox | WASM sandbox (wasmtime) | OS sandbox (sandbox-exec/Landlock) |
| Development | Cross-compile to WASM, stub host funcs | `cargo run`, `cargo test`, `dbg!()` |
| Schema | Declared in plugin manifest | Ships with pipe, WIT type defs |

The internal code (GitHub REST client, JSON parsing, rate limiting)
migrates almost unchanged. `host_http` calls become direct `reqwest`
calls. The wrapper changes from MotherChildPlugin to a `main()` that
reads stdin and writes stdout.

## Secrets Architecture Detail

**Credential lifecycle:**

```
patina auth login github
  → Register GitHub OAuth App (one-time, or use Patina's app ID)
  → Device authorization flow:
    1. POST https://github.com/login/device/code
    2. Display: "Go to github.com/login/device, enter code: XXXX-YYYY"
    3. Poll: POST https://github.com/login/oauth/access_token
    4. Receive token
  → Store: patina secrets add --global github:user <token>
  → Store metadata: provider=github, scopes=[repo], expires=never,
    created=2026-03-06, last_used=null
```

**Credential resolution for pipes:**

```
Mother loads source config → auth = "github:user"
  → Resolve: patina secrets get --global github:user
  → Decrypt via Keychain/Touch ID (or session cache)
  → Include token in stdin config JSON: { "auth": { "token": "..." } }
  → Pipe reads config, uses token directly via reqwest
  → OS sandbox prevents pipe from leaking token (no network, no files)
```

No secret-grants.toml. No manual PAT. The trust model shifts from
"user grants plugin access to a secret" to "user authenticates with
a provider, Mother injects credentials into pipe config via stdin."

**Security model comparison:**

| | Current (plugin grants) | Target (pipe auth) |
|---|---|---|
| Who creates credential | User (manual PAT) | `patina auth` (OAuth) |
| Who stores it | User (vault add) | `patina auth` (vault add) |
| Who grants access | User (secret-grants.toml) | Source config (auth = "github:user") |
| Enforcement | host_http call-time check | OS sandbox (no network except stdout) |
| Revocation | Delete from vault | `patina auth revoke github` |

The OS sandbox prevents credential exfiltration via filesystem or
subprocess — the pipe can't write the token to disk, send it to a
different process, or access other secrets. The credential arrives via
stdin, gets used for declared-domain HTTP only. The difference from
the plugin model is UX: one command vs four steps.

## Relationship to Other Specs

- **forge-plugin-extraction** — proved the pattern. Pipe architecture
  supersedes the "plugin" framing but keeps the infrastructure (host_emit,
  projection, schema validation).
- **lake-registry** — becomes the "data lake destination" type. Lake
  metadata in graph.db, lake sources configured as pipe destinations.
- **core-extraction** — pipes are NOT core. They're user-level drivers
  managed by Mother. Core is protocol + stores.
- **continuous-operation** — Mother daemon manages pipe scheduling.
  Pipe health checks feed into Mother's health monitoring.
- **scrape-simplification** — scrape stays local (git). Pipe data
  arrives via Mother, not via scrape dispatch. `patina scrape` triggers
  projection of pipe-emitted events, not pipe execution.
- **persona-federation** — dependency. Persona keypair serves as pipe
  signing key (fact provenance), node identity (Iroh peer discovery),
  and UCAN issuer (capability delegation). The identity primitive from
  persona-federation is what makes pipes work in a multi-node network.

## Key Files (current, to be refactored)

**Pipe code (from forge plugin):**
- `plugins/forge/src/github.rs` — GitHubClient, migrates to github-pipe
  binary. `host_http` calls become direct `reqwest` calls.
- `plugins/forge/plugin.toml` — becomes pipe.toml manifest format

**Host infrastructure (security patterns reusable):**
- `src/plugin/internal/host_support.rs` — domain allowlist, credential
  injection, leak check — all host-side security logic. Patterns reusable
  for pipe manager validation (not the code directly, but the approach).
- `src/secrets/mod.rs` — vault, identity, session caching. Pipes use
  the same vault; Mother resolves credentials before passing to pipe.

**Projection (stays, source-agnostic):**
- `src/commands/scrape/forge/mod.rs` — project_from_events, FTS5 population

**MCP server (pattern to follow for pipe protocol):**
- `src/mcp/server/mod.rs` — stdio JSON-RPC server. Same spawn-and-stdio
  pattern that pipes will use, but pipes are simpler (no JSON-RPC, just
  config in + NDJSON out).

**New code needed:**
- `crates/patina-pipe/` — SDK crate (`Pipe` trait, `run()`, types)
- `pipes/github-pipe/` — first pipe binary, depends on patina-pipe SDK
- `src/pipe/` — Mother pipe manager (spawn, sandbox, read stdout, emit)
- `src/auth/` — OAuth device flow, provider registry, `patina auth` CLI
- WIT: `wit/pipe/pipe.wit` — type definitions for fact shapes, config
- `src/commands/pipe.rs` — `patina pipe new/run/health/list` commands

## Data Flow Architecture

Pipes are the connective tissue across the entire data architecture.
Every arrow is a pipe — same SDK, same protocol, same WIT contract.

### Full data flow

```
External Sources (GitHub, Slack, RSS, APIs...)
  │ ingest pipes (github-pipe, slack-pipe, rss-pipe)
  ▼
Data Lakes (Parquet, lakehouse-managed, raw/complete)
  │ transform pipes (filter-pipe, enrich-pipe)
  ▼
App Layer (transforms, enrichment, embeddings)
  │ structure pipes (embed-pipe, curate-pipe)
  ▼
Data Blocks (embeddings, curated datasets from lakes/sources)
  │ serve pipes (query-pipe, export-pipe)
  ▼
Apps / Projects (action, workflows, UI)
```

Each layer is both a source and a destination depending on direction.
A lake is a destination for github-pipe and a source for filter-pipe.
A block is a destination for embed-pipe and a source for a project.

### Lake-as-source pattern

The default external data flow is pipe → lake → project projection:

```
github-pipe (fetch everything from org)
  └─▶ org-lake/events.db (all issues, PRs, releases)
        ├─▶ project-A/.patina/ (project issues + PRs only)
        ├─▶ project-B/.patina/ (project issues + PRs only)
        └─▶ security-block/ (security advisories only)
```

Benefits:
- **No re-fetching**: pipe fetches once, projections are local queries
- **Fan-out is config**: destinations select what they want from the lake
- **Backfill is free**: lake has all history, new projects can project
  retroactively
- **Rate limit friendly**: one API call set, many consumers

Mother decides parallelism strategy — whether to spawn one pipe and
fan-out the output, or spawn N pipes with different configs. The pipe
binary is the same either way.

### Internal pipes

Internal pipes (lake → block, block → app) use the same SDK and
protocol as external pipes. The difference is the source:

- External pipe: source is an HTTP API, auth is OAuth token
- Internal pipe: source is an events.db/Parquet file, auth may be
  a local secret or none (same machine)

```rust
// pipes/lake-reader-pipe/src/main.rs — reads from a lake
impl Pipe for LakeReaderPipe {
    fn fetch(&self, config: &PipeConfig) -> Result<Vec<Fact>> {
        let db = Connection::open(&config.params["lake_path"])?;
        let facts = query_events(&db, &config.types, &config.since)?;
        Ok(facts)
    }
}
```

Same `Pipe` trait, same `run()`, same NDJSON on stdout. Mother doesn't
care if the pipe talked to GitHub or read from a local SQLite file.

## Transport Model

The pipe protocol (config in, facts out) is transport-agnostic. Build
stdio first, design for future transports:

| Transport | Topology | Use Case |
|---|---|---|
| stdio | local, same machine | Default. Mother spawns pipe process |
| HTTP+SSE | remote, pipe on VPS | Pipe runs on server, Mother connects |
| Streamable HTTP | shared, multi-Mother | Community pipe serving multiple users |

Same message format across all transports. Same fact schema. Different
wire. Following the MCP pattern where deployment topology doesn't
dictate protocol design.

**Current scope**: stdio only. Don't hardcode stdio assumptions in the
protocol layer (message format), but don't build HTTP transport until
there's a concrete remote pipe use case.

## Network / P2P Future

Brief notes on where this architecture goes at network scale — these
are future scope, not current implementation targets:

- **Content-addressed facts**: each fact gets a blake3 hash. Dedup
  across pipes and nodes is automatic. Same fact from two sources
  resolves to one entry.
- **Iroh document sync**: facts sync between nodes as Iroh documents.
  Each node runs its own pipes, facts converge via gossip.
- **Persona = node identity**: the persona keypair from persona-
  federation serves as Iroh node identity and UCAN issuer. Facts
  carry provenance signatures.
- **Node specialization**: pipe nodes (fetch), compute nodes
  (embeddings), belief nodes (inference), leaf nodes (read-only).
  The pipe binary doesn't change at any scale.

The key architectural point: designing pipes as processes over stdio
with transport-agnostic protocol means the same pipe binary works on
a developer laptop, a VPS, a Docker container, or a p2p node. No
redesign needed as deployment topology evolves.

## Open Questions

1. **Pipe discovery.** How do users find and install community pipes?
   Registry like crates.io? GitHub releases? Manual download?

2. **Schema ownership.** Does the schema ship with the pipe or is it
   installed independently? Lean toward: ships with the pipe.

3. **Pipe versioning.** When a pipe's output format changes, how do
   projection tables migrate? Schema version in event metadata?

4. **Multi-provider pipes.** Is "github-pipe" one pipe for all GitHub
   instances, or is "github-enterprise" a separate pipe? Lean toward:
   one pipe, configured with base_url per destination.

5. **Community pipe security model.** First-party pipes make direct
   HTTP (trusted code + OS sandbox). Community pipes may need host-
   proxied I/O where Mother makes HTTP calls on the pipe's behalf —
   same pattern as current host_http. Design the two-tier model when
   community pipes become relevant.
