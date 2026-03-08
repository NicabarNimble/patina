# Design: Pipe Architecture — Protocol + Broker Model

## What Pipe Architecture Solves

Patina is a knowledge system. Five verbs define it: capture, index,
search, believe, evolve. Today, "capture" means running `patina scrape`
— it reads your local git history, code structure, sessions, and layer
files. Everything Patina knows comes from your local machine.

But knowledge doesn't only live locally. Your GitHub issues contain
decisions. Your Slack threads contain context. Your CI logs contain
evidence. To feed the belief loop from these sources, Patina needs a
way to ingest external data — securely, without compiling every possible
data source into the binary.

That's what the pipe architecture delivers: a protocol for external
data to enter Patina, a broker to route it, and a security model that
keeps children sandboxed.

### What Changes for Users

Before pipe architecture, connecting GitHub to Patina required:
generate a PAT on GitHub's web UI, run `patina secrets add`, hand-edit
a TOML grants file, configure the forge plugin, then run
`patina scrape forge`. Five steps requiring knowledge of vault mechanics
and plugin configuration.

After:

```
$ patina connect github
  Opening browser for GitHub authorization...
  Enter code: ABCD-1234
  Waiting for approval... approved!
  Connection configured.

$ patina mother run github
  github: 47 facts written, 3 dedup, cursor: 2026-03-07T...
```

Two commands. The first creates the credential and connector config.
The second fetches data and routes it into your project's knowledge
base. Run it again tomorrow and it picks up where it left off — the
cursor is tracked per-source, so each run fetches only what's new.
Duplicate facts from overlapping fetches are caught by content-hash
dedup. Idempotent by design.

From there, `patina scry "what are the open issues about auth?"`
searches GitHub issues alongside your code, sessions, and beliefs.

For users who prefer manual PATs or run headless (CI, servers), the
existing `patina secrets add` workflow continues to work. OAuth is a
convenience, not a requirement.

### How It Works: Protocol, Children, Broker

Three concepts, each with one job (detailed in §1, §2, §4 below):

**Pipe protocol** (§1) is how components exchange data. JSON-RPC 2.0
with five request methods (`pipe/initialize`, `pipe/fetch`,
`pipe/ingest`, `pipe/health`, `pipe/shutdown`) and a streaming
notification (`pipe/fact` — facts delivered one at a time during fetch,
no accumulation). The protocol doesn't care about transport — the same
messages travel over stdio (native children), WASM host calls (existing
plugins), or HTTP (future remote children). A Fact is a Fact regardless
of how it arrived.

**Children** (§2) are managed services that speak the protocol. A
GitHub connector fetches issues. A Slack connector fetches messages. An
RSS reader fetches feeds. Each child is a normal Rust binary —
`cargo run`, `cargo test`, `dbg!()`, the full ecosystem. Children run
inside OS sandboxes (macOS `sandbox_init()`, Linux Landlock) that
restrict filesystem access and outbound network to port 443 + DNS.
Credentials arrive via stdin, never through environment variables or
files.

**Mother** (§4) is the broker. She reads your project's `sources.toml`
to learn what external data you want. She resolves credentials from the
vault. She spawns the right child, sends it the fetch request, validates
every fact against declared schemas, deduplicates via content hashing,
and writes valid facts to your project's event store in a single
transaction. Mother routes — she never transforms. Schema validation and
content-hash dedup happen in Mother regardless of whether the child is
WASM or native — the broker is the single enforcement point.

### The Five Specs

The pipe architecture decomposes into five focused specs, each with
one job:

1. **pipe-protocol-types** ([[spec-pipe-protocol-types]]) — The shared
   vocabulary. A `patina-pipe-types` crate containing the Fact struct,
   PipeError enum, Capabilities, canonical JSON serialization, and
   content hashing. Both WASM and native children depend on these types.
   The protocol defined in code.

2. **pipe-native-transport** ([[spec-pipe-native-transport]]) — The
   native binding. A `patina-pipe` crate with the `Child` trait and
   `run()` entry point. Developers implement `Child`, call `run()` from
   `main()`, and get a working connector in ~50 lines. Includes OS
   sandbox enforcement on both macOS (`sandbox_init()` C API) and Linux
   (Landlock ABI v4+) — children that can't be sandboxed refuse to
   start unless explicitly opted out with `--no-sandbox`. No silent
   security degradation.

3. **github-connector** ([[spec-github-connector]]) — The first native
   child. Proves the pattern end-to-end with a real API. Migrates the
   GitHub REST client from the existing WASM forge plugin to a
   standalone binary. Emits `github.*` facts (not `forge.*` — connectors
   own their schema namespace). After parity verification, 2,200+ lines
   of GitHub-specific code leave the core binary.

4. **patina-connect** ([[spec-patina-connect]]) — The connection model.
   `patina connect github` replaces four manual steps with one OAuth
   device flow. Stores the token in the existing age-encrypted vault,
   creates the connection config. Works alongside `patina secrets` for
   manual PAT users. No pipe type dependencies — it's just vault +
   config.

5. **mother-broker** ([[spec-mother-broker]]) — The routing engine.
   Mother reads `sources.toml`, spawns children (WASM or native),
   validates facts, and writes them transactionally with cursors. Adds
   `patina mother run` and `patina mother sources` commands. Before
   closing, the WASM fact routing must either be unified through the
   broker or explicitly declared legacy — no silent drift.

### Build Order

(See parent SPEC.md §Children for dependency graph and blocked_by.)

```
pipe-protocol-types ─── pipe-native-transport ─┬── github-connector
                                               └── mother-broker

patina-connect (independent, can start immediately)
```

Protocol types are the foundation — everything depends on the shared
vocabulary. Native transport needs types to define the Child trait, and
includes real OS sandbox enforcement on both platforms (macOS
`sandbox_init()`, Linux Landlock v4 — both fail hard if enforcement
is unavailable). GitHub connector needs native transport to run as a
process. Mother broker needs native transport to spawn children and
protocol types for validation.

Patina-connect has no blockers. It uses the existing vault
infrastructure and writes TOML config files. It can be built in
parallel with everything else.

Mother broker and github-connector can overlap: the broker tests
against a test-child first, then verifies against the real
github-connector once it's ready.

### What Exists Today vs What's New

(See §13 Migration Path for phased delivery.)

| Concern | Exists Today | Pipe Architecture Adds |
|---------|-------------|----------------------|
| Fact emission | `host_emit::emit_fact()` in WASM host | Formalized as pipe protocol, shared types across runtimes |
| Schema validation | `host_support::validate_emit()` | Broker-side validation with content-hash dedup |
| HTTP for children | `host_http::get/post` with domain allowlist | Native children use reqwest directly, OS sandbox enforces domains |
| Credential delivery | Manual PAT + secret-grants.toml | `patina connect` OAuth + vault, credentials via stdin |
| Child lifecycle | WASM mother-child world (spawn, heartbeat) | Unified lifecycle for WASM and native (BrokerChild trait) |
| Routing | Forge writes directly to events.db | Mother broker routes facts to destination based on declarations |
| Content addressing | None | blake3 over canonical JSON, dedup across sources |
| OS sandboxing | WASM sandbox (wasmtime) | macOS sandbox_init() + Linux Landlock for native children |

The infrastructure that exists today — the vault, the event store, the
WASM plugin engine, the MCP server's JSON-RPC pattern — all survive and
are reused. Pipe architecture names what already works, fills the gaps,
and extends it to native processes.

## Design Principles

This design follows the reframing from sessions 7-8:

- **Pipe = protocol**, not a process type. JSON-RPC 2.0 + WIT types.
- **Children = managed services** that speak pipe protocol. WASM or
  native. Mother manages lifecycle.
- **Mother = broker**. Routes facts from sources to destinations
  based on pub/sub declarations. Never transforms data.
- **Connection = pipe protocol + auth**. One command links them.
- **Beliefs are the exit layer**. Everything below is plumbing.

This document addresses all 10 issues from the five-lens audit
(sessions 7-8): streaming delivery, canonical serialization, typed
errors, delivery guarantees, persona enforcement, failure modes,
encryption gap, async position, child framework, and future scope.

## 1. Pipe Protocol Specification

### 1.1 Foundation

JSON-RPC 2.0 (RFC 7049). Self-owned `pipe/*` method namespace.
MCP-compatible (JSON-RPC is JSON-RPC) but MCP-independent (we own
our methods, types, and evolution).

### 1.2 Methods

**Initial scope (poll mode):**

```
Mother → child methods:

pipe/initialize     →  Capability exchange. Mother sends config,
                       child responds with capabilities.

pipe/fetch          →  Request data. Mother sends params (types,
                       since, limit). Child streams facts back as
                       pipe/fact notifications, then sends result
                       summary. Direction: Mother requests, child
                       streams facts + returns result.

pipe/ingest         →  Deliver records. Mother sends a bounded batch
                       of records to a storage child (e.g., lakehouse).
                       Child writes, dedup-checks, and returns result.
                       Hard spec in [[raw-lake-ingestion]] DESIGN.md.

pipe/health         →  Connectivity check. Returns ok/degraded/down
                       with latency and message.

pipe/shutdown       →  Graceful shutdown request. Child flushes
                       pending work, then exits.

Child → Mother notifications (during pipe/fetch):

pipe/fact           →  Single fact delivery. Streaming notification,
                       not collected into Vec. O(1) memory per fact.
```

**Future (stream mode, not in initial implementation):**

```
pipe/emit           →  Push a fact. Child → Mother. Used in stream
                       mode for continuous emission.

pipe/capabilities   →  Re-query capabilities (for long-lived children
                       whose capabilities may change at runtime).
```

### 1.3 Streaming Fact Delivery

**Audit fix: Vec<Fact> OOM risk.**

Facts are delivered as individual JSON-RPC notifications, never
collected into a Vec. A `pipe/fetch` call returns a summary; the
actual facts arrive as `pipe/fact` notifications during execution.

```
Mother                              Child
  |                                   |
  |---- pipe/fetch {since, types} -->|
  |                                   |-- fetch page 1
  |<--- pipe/fact {fact_1} ----------|
  |<--- pipe/fact {fact_2} ----------|
  |<--- pipe/progress {50 fetched} --|
  |                                   |-- fetch page 2
  |<--- pipe/fact {fact_3} ----------|
  |<--- pipe/fact {fact_4} ----------|
  |<--- result {fetched: 4} ---------|
  |                                   |
```

Mother processes each fact as it arrives — validate schema, compute
hash, stage for write. A child emitting 100K facts uses O(1) memory
on the transport layer (per-notification). The broker layer
accumulates facts into a bounded batch (max `DEFAULT_MAX_BATCH_SIZE`,
currently 10,000) before the transactional write to events.db or
the pipe/ingest call to a lakehouse child. The batch bound is the
memory ceiling — not unbounded accumulation.

The `pipe/fact` notification:
```json
{
  "jsonrpc": "2.0",
  "method": "pipe/fact",
  "params": {
    "schema": "github",
    "fact_type": "issue",
    "data": { "number": 42, "title": "...", ... },
    "content_hash": "blake3:abc123...",
    "signature": "ed25519:xyz789..."
  }
}
```

### 1.4 Canonical Serialization

**Audit fix: broken content addressing.**

blake3 hashing requires deterministic serialization. `serde_json`
does NOT guarantee key ordering — the same data can produce different
hashes on different runs.

Solution: canonical JSON serialization for content addressing.

Rules:
1. Object keys sorted lexicographically (Unicode code point order)
2. No whitespace between tokens
3. Numbers: no leading zeros, no trailing zeros after decimal
4. Strings: minimal escaping (only required characters)
5. No duplicate keys (last wins if encountered)

Implementation: `patina-pipe-types` provides `canonical_json(value)`
that serializes a `serde_json::Value` with sorted keys. Used by the
SDK before blake3 hashing. The hash is computed over canonical JSON
bytes, not over the wire representation.

```rust
// patina-pipe-types/src/canonical.rs
pub fn canonical_json(value: &serde_json::Value) -> Vec<u8> {
    let mut buf = Vec::new();
    write_canonical(&mut buf, value);
    buf
}

pub fn content_hash(value: &serde_json::Value) -> String {
    let canonical = canonical_json(value);
    let hash = blake3::hash(&canonical);
    format!("blake3:{}", hash.to_hex())
}
```

### 1.5 Typed Errors

**Audit fix: undefined error types.**

Pipe protocol errors are Rust enum variants. Callers match on the
variant, not numeric codes. JSON-RPC error codes are transport detail
— they live in the transport serialization layer (`patina-pipe`'s
`send_error()`, `patina-sdk`'s host error mapping), not in the enum.

**Design constraint** (from five-lens audit, session 12): if callers
match on code numbers, adding new variants breaks everyone. If they
match on variants, new codes are a transport-layer detail.

```rust
// patina-pipe-types/src/error.rs

pub enum PipeError {
    Transient { message: String, retry_after_ms: Option<u64> },
    Fatal { message: String },
    RateLimited { message: String, retry_after_ms: u64 },
    Partial { message: String, emitted: u64 },
}

// Transport-only (not part of child author API):
impl PipeError {
    pub fn jsonrpc_code(&self) -> i32 { /* -32001..-32004 */ }
    pub fn from_jsonrpc(code: i32, msg: String, data: Option<Value>) -> Self { /* ... */ }
}
```

JSON-RPC error code mapping (used by transport layers only):
- `-32001`: Transient (network timeout, service unavailable)
- `-32002`: Fatal (bad auth, schema not found, invalid config)
- `-32003`: Rate limited (429 from source API)
- `-32004`: Partial success (some facts emitted before failure)
- `-32600` to `-32700`: Standard JSON-RPC errors (parse, method, etc.)

### 1.6 Content Addressing and Signing

Every fact emitted through pipe protocol is:

1. **Canonicalized**: `data` field serialized via canonical JSON
2. **Hashed**: blake3 over canonical bytes → `content_hash`
3. **Signed** (future): ed25519 signature over `content_hash` using
   persona keypair → `signature`

Content addressing (steps 1-2) ships with the initial implementation.
Child authors never touch hashing — the transport layer computes it
automatically. Mother checks `content_hash` for dedup before writing
to events.db.

```
Child calls emit("github", "issue", data)
  → Transport: canonical_json(data) → bytes
  → Transport: blake3(bytes) → content_hash
  → Transport: send pipe/fact notification
  → Mother: verify content_hash, check for dedup
  → Mother: write to events.db (or skip if duplicate)
```

**Signing is stubbed until [[spec-persona-federation]] ships keypair
infrastructure.** The `signature` field exists in the Fact struct
(empty string) so the wire format is stable. When persona-federation
ships, signing is added to the transport layer — child code doesn't
change. Content-hash dedup works without signatures.

### 1.7 Delivery Guarantees

**Audit fix: no delivery guarantees stated.**

**At-least-once with content-addressed dedup.**

- Children may emit the same fact multiple times (crash, retry,
  overlapping fetches). Mother deduplicates via `content_hash`.
- Mother acknowledges receipt implicitly (the JSON-RPC result to
  pipe/fetch). If the child crashes before receiving the result,
  it re-emits on restart — Mother deduplicates.
- Ordering: facts arrive in the order the child emits them. Mother
  writes in arrival order. No global ordering across children.
- Completeness: not guaranteed. A child may fail mid-fetch. The
  `since` cursor and Partial error enable resumption.

This is the right model for Patina: facts are evidence feeding a
belief loop. Duplicate evidence is harmless (dedup). Missing evidence
is detectable (incremental cursor shows gap). Ordering within a
source is preserved; cross-source ordering is meaningless (beliefs
don't depend on insertion order).

## 2. Child Framework

### 2.1 Child Types

All children are managed services of Mother. All speak pipe protocol.
The type determines what the child does, not how it communicates.

| Type | Does | Examples |
|---|---|---|
| Connector | Bridges external sources to Patina | github, slack, rss |
| Transport | Holds complex external connections | websocket, webhook |
| Lakehouse | Manages data storage layer | parquet, s3, sqlite |
| Transform | Curates/enriches data between layers | filter, embed, aggregate |

### 2.2 Child Runtimes

Children can run as WASM components or native processes. The pipe
protocol is the same for both — Mother doesn't care.

**WASM children** (current model):
- Run in wasmtime via mother-child world
- Communicate via host function calls (host_emit → emit, etc.)
- Sandboxed by WASM — all I/O proxied through host
- Installed per-project in `.patina/plugins/`
- Proven by forge connector

**Native children** (new model):
- Run as OS processes communicating over stdio
- JSON-RPC 2.0 over stdin/stdout
- Sandboxed by OS (macOS sandbox_init(), Linux Landlock)
- Installed user-level in `~/.patina/children/`
- Normal Rust development: `cargo run`, `cargo test`, `dbg!()`

The patina-sdk crate serves WASM children. The patina-pipe crate
serves native children. Both implement the same pipe protocol.

### 2.3 Child Lifecycle

Mother manages all children through a uniform lifecycle:

```
[Configured]  →  Mother reads sources.toml
     |
[Spawning]    →  Mother spawns child (WASM: instantiate,
     |            native: fork+exec in sandbox)
     |
[Initializing] → pipe/initialize handshake
     |            (capability exchange, config delivery)
     |
[Running]     →  Child processes pipe/fetch or pipe/emit calls
     |            Mother monitors via pipe/health
     |
[Draining]    →  pipe/shutdown sent, child flushes work
     |
[Stopped]     →  Child exited. Mother records exit status.
     |            Poll: done. Stream: restart after backoff.
```

Health monitoring (stream mode):
- Mother calls `pipe/health` every N seconds (configurable)
- Three states: `ok`, `degraded`, `down`
- `degraded` → log warning, continue
- `down` → restart with exponential backoff (1s, 2s, 4s, max 5min)
- 3 consecutive `down` → stop, alert user

### 2.4 Child Manifest (child.toml)

```toml
[child]
name = "github-connector"
version = "0.1.0"
type = "connector"          # connector | transport | lakehouse | transform
runtime = "native"          # native | wasm
lifecycle = "poll"          # poll | stream | manual

[capabilities]
data_types = ["issues", "prs", "comments", "reviews"]
supports_incremental = true

[domains]
allowed = ["api.github.com"]

[auth]
required = true
provider = "github"

[schemas.github]
package = "patina:schema/github@1.0.0"
```

For WASM children, this extends the current plugin.toml format.
For native children, this is the equivalent manifest.

### 2.5 Connector Child Example (GitHub)

The GitHub connector replaces the current forge WASM plugin. It
speaks pipe protocol over either WASM (patina-sdk) or native
(patina-pipe) transport.

**Native version (patina-pipe):**

```rust
// children/github-connector/src/main.rs
use patina_pipe::{Child, run, FactEmitter, FetchParams, PipeError};

struct GitHubConnector;

impl Child for GitHubConnector {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            provider: "github",
            data_types: vec!["issues", "prs"],
            supports_incremental: true,
        }
    }

    fn fetch(
        &mut self,
        params: &FetchParams,
        emitter: &mut FactEmitter,
    ) -> Result<FetchResult, PipeError> {
        let client = GitHubClient::new(&params.auth)?;

        for issue in client.fetch_issues(params.since)? {
            emitter.emit("github", "issue", &issue)?;
        }
        for pr in client.fetch_prs(params.since)? {
            emitter.emit("github", "pull-request", &pr)?;
        }

        Ok(FetchResult { emitted: emitter.count() })
    }

    fn health(&self) -> Result<HealthStatus, PipeError> {
        Ok(HealthStatus { status: Status::Ok, message: None, latency_ms: None })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(GitHubConnector)
}
```

Key differences from old design:
- `&mut self` instead of `&self` — allows mutable state (connection
  pools, rate limit tracking) across calls. (Audit fix: `&self`
  prevents mutable state)
- `FactEmitter` instead of `Vec<Fact>` return — streaming delivery,
  each fact sent immediately via pipe/fact notification. (Audit fix:
  Vec<Fact> OOM)
- `PipeError` instead of generic `Result` — typed, retryable vs
  fatal. (Audit fix: undefined error types)
- Trait named `Child` not `Pipe` — pipe is the protocol, child is
  the implementation.

**WASM version (patina-sdk, existing pattern updated):**

```rust
// plugins/forge/src/lib.rs (updated to pipe protocol)
use patina_sdk::mother_child::{emit, fetch, log};

// Same emit() call, renamed from host_emit.
// SDK handles pipe protocol over WASM host calls.
emit::emit_fact("forge", "issue", &event_data)?;
```

The WASM version continues to work. The rename (`host_emit` → `emit`)
is the only visible change. Under the hood, patina-sdk gains pipe
protocol awareness — the host function IS the transport binding.

## 3. Connection Model

### 3.1 Connection = Pipe Protocol + Auth

A connection is a named pair: credential + connector child config.
Created by one command, referenced by name in destination config.

```
patina connect github
  1. OAuth device flow → acquire token
  2. Store token in vault as "github:user"
  3. Register connector child config
  4. Done. Ready to use in sources.toml.
```

### 3.2 Connection Lifecycle

```
patina connect github
  → Prompt: "GitHub account to connect?"
  → Device authorization: browser popup + user approval
  → Token stored: vault.age (github:user)
  → Metadata stored: scopes, expiry, provider, created_at
  → Connector child registered: ~/.patina/connections/github.toml

patina connect status
  → github: connected (token valid, expires: never)
  → slack: connected (token valid, expires: 2026-04-06)

patina connect refresh github
  → Re-authorize if expired

patina connect remove github
  → Remove token from vault
  → Remove connector config
```

### 3.3 Connection Config

```toml
# ~/.patina/connections/github.toml
[connection]
name = "github"
provider = "github"
credential = "github:user"     # references vault secret
child = "github-connector"     # which child binary to use
created = "2026-03-06T00:00:00Z"
```

This is the evolution of `patina secrets` for external sources.
The vault stays the same. The addition is: one command creates
both the credential AND the connector configuration.

## 4. Mother as Broker

### 4.1 Routing Engine

Mother reads destination declarations from sources.toml files across
all registered projects, lakes, and blocks. For each source entry:

1. Find the connection (connection name → connection config)
2. Resolve credential (vault decrypt with session caching)
3. Determine child to spawn (connection config → child binary)
4. Build fetch params (merge destination params + auth + cursor)
5. Spawn child (or reuse running instance for stream mode)
6. Route emitted facts to destination's events.db
7. Update incremental cursor (last-sync timestamp)

### 4.2 No Fan-Out Optimization (Initial Scope)

One child spawn per `run_source()` call. Multiple projects referencing
the same connection get separate spawns:

```
sources.toml (project A): connection = "github", types = ["issues"]
sources.toml (project B): connection = "github", types = ["prs"]
→ Mother spawns github-connector twice with different params
```

Content-hash dedup handles any data overlap between runs. This is
correct because each source has its own cursor and writes to its own
project's events.db.

**Future optimization (deferred):** shared spawn routing facts to
multiple destinations. Not implemented until measured need exists.
Children don't know about fan-out regardless of strategy.

### 4.3 Scheduling

Mother integrates with [[spec-continuous-operation]] for scheduling:

| Mode | Trigger | Behavior |
|---|---|---|
| `on-scrape` | `patina scrape` command | Fetch before local scrape |
| `hourly` | Clock (cron-like) | Mother daemon schedules |
| `daily` | Clock (cron-like) | Mother daemon schedules |
| `stream` | Always-on | Mother keeps child running |
| `manual` | `patina mother run <name>` | One-shot, user-triggered |

Schedule is per-destination, not per-child. The same github-connector
can be `on-scrape` for project A and `hourly` for the org lake.

### 4.4 Schema Validation

Mother validates every fact before writing to events.db:

1. Check `schema` + `fact_type` against child manifest's declared
   schemas
2. Verify `content_hash` matches canonical JSON of `data`
3. Verify `signature` against persona keypair (when available)
4. Check for duplicate `content_hash` in events.db (dedup)
5. Write valid fact with provenance: `source_id = "child:<name>"`

Invalid facts are logged and dropped. Mother never writes unvalidated
data.

### 4.5 Schema Resolution

Schema ownership spans three specs. This is the unified flow:

1. **Child manifest declares intent.** `child.toml [schemas.github]`
   says "I emit `github.*` facts." The manifest lives with the child
   binary (in `children/github-connector/child.toml`). Mother reads
   it at spawn time to know what schemas to expect.

2. **Schema file defines structure.** `.patina/schemas/github/schema.toml`
   defines fact types (`github.issue`, `github.pr`), event_type
   mappings, FTS5 indexes, and embedding config. The schema file
   lives in the destination project (or globally in `~/.patina/schemas/`).

3. **Broker validates against structure.** Mother loads schema
   definitions from the destination project, cross-references with the
   child manifest's declarations, and validates each fact's `schema` +
   `fact_type` against the loaded definitions.

**Installation:** In initial scope, schema files are installed manually
(copy `schema.toml` to `.patina/schemas/<name>/`). The connector ships
its schema definition alongside its binary. Future: Mother auto-installs
schemas from child manifests on first run.

**Single source of truth:** The schema.toml file IS the source of truth
for fact structure. The manifest declaration is a reference (package
name + version) that tells Mother which schema.toml to load. If the
schema.toml is missing from the destination project, validation fails
with a clear error: "schema 'github' not installed. Copy from
children/github-connector/schema.toml."

## 5. Persona Enforcement (Future — Depends on persona-federation)

**Audit fix: hand-waved persona enforcement.**

**Note:** Full persona enforcement requires [[spec-persona-federation]]
to ship keypair infrastructure. The design below is the target state.
The initial pipe protocol implementation operates without persona
enforcement — all facts belong to a single implicit persona (`default`).

**Exception: `pipe/ingest` carries persona pre-federation.** The
`pipe/ingest` method (Mother → lakehouse child, defined in
[[raw-lake-ingestion]]) includes a `persona` field because lake
storage is keyed by persona from day one (`lake_registry` and
`lake_sync` have `persona` in their primary keys). Pre-federation,
this is always `"default"`. The field exists so the lakehouse child
can partition storage by persona without a schema migration when
persona-federation ships. This is forward-compatible keying, not
persona enforcement — there is no keypair validation, no cross-persona
denial, no namespace isolation until persona-federation.

The core pipe protocol (`pipe/initialize`, `pipe/fetch`, Fact struct)
does NOT include persona fields in initial scope. `pipe/ingest` is
the exception because it writes to persona-keyed storage.

### 5.1 Target: Persona Scoping

When persona-federation ships, every pipe protocol interaction will
be scoped to a persona:

- `pipe/initialize` will include `persona` field
- Emitted facts will carry `persona` in the notification
- Mother will route facts only to destinations owned by the same persona
- Cross-persona routing will be denied at the broker level

### 5.2 Target: Enforcement Points

| Point | What | How |
|---|---|---|
| Config delivery | Child receives persona ID | pipe/initialize params |
| Fact validation | Fact's persona matches session | Mother checks on receipt |
| Storage routing | Facts go to persona's namespace | Mother routes by persona |
| Cross-persona | Denied | Mother refuses to route |
| Multi-persona | Isolated namespaces | Mother runs separate sessions |

### 5.3 Target: Multi-Persona on One Node

Mother will support multiple personas on the same machine (e.g., work
persona and personal persona). Each persona:

- Has its own connections, destinations, credentials
- Has its own events.db namespace
- Has its own belief set
- Cannot see facts from other personas

The persona will be set at the destination level in sources.toml:
```toml
[sources.github]
persona = "work"         # optional, defaults to active persona
connection = "github"
...
```

## 6. Failure Mode Catalog

**Audit fix: no failure modes specified.**

### 6.1 Child Failures

| Failure | Detection | Recovery |
|---|---|---|
| Child won't start | Spawn fails | Log error, alert user, skip schedule |
| Child crashes mid-fetch | Process exit, broken pipe | PipeError::Partial, retry with cursor |
| Child hangs | Health check timeout | Kill process, restart with backoff |
| Bad credentials | PipeError::Fatal(auth) | Alert user: `patina connect refresh` |
| Rate limited | PipeError::RateLimited | Backoff per retry_after_ms |
| Schema mismatch | Fact validation fails | Drop fact, log warning, continue |
| Content hash mismatch | Hash verification fails | Drop fact, log error (corruption) |

### 6.2 Mother Failures

| Failure | Detection | Recovery |
|---|---|---|
| events.db locked | SQLite error | Retry with backoff (up to 30s) |
| Vault locked | Keychain timeout | Session cache miss → prompt user |
| Disk full | Write error | Stop all children, alert user |
| Mother crash | Process exit | systemd/launchd restarts Mother |
| Config parse error | TOML error on startup | Log specific error, skip bad entry |

### 6.3 Network Failures

| Failure | Detection | Recovery |
|---|---|---|
| Source API down | Child returns Transient error | Retry with backoff |
| DNS failure | Child returns Transient error | Retry with backoff |
| TLS failure | Child returns Fatal error | Alert user (cert issue) |
| Partial response | Child returns Partial error | Keep emitted facts, retry remainder |

## 7. Transport Bindings

### 7.1 WASM Transport (patina-sdk)

The existing host function interface IS the WASM transport binding
for pipe protocol. Current names map to protocol methods:

| Current (host_*) | Renamed | Protocol Method |
|---|---|---|
| `host_emit::emit_fact` | `emit::emit_fact` | `pipe/fact` notification |
| `host_http::get/post` | `fetch::get/post` | Child-internal (not protocol) |
| `host_log::log` | `log::log` | Child-internal (not protocol) |

The renaming is cosmetic — the protocol binding already exists.
patina-sdk gains pipe protocol types (PipeError, Capabilities, etc.)
that are shared with patina-pipe via patina-pipe-types.

### 7.2 Native Transport (patina-pipe)

New. JSON-RPC 2.0 over stdio (stdin/stdout):

- stdin: Mother → child (requests)
- stdout: child → Mother (responses + notifications)
- stderr: child logging (structured, not protocol)

One JSON-RPC message per line (newline-delimited). This is the same
transport as the MCP stdio server (`src/mcp/server/mod.rs`).

The `run()` function in patina-pipe:

```rust
pub fn run<C: Child>(mut child: C) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        let request: Request = serde_json::from_str(&line?)?;
        match request.method.as_str() {
            "pipe/initialize" => {
                let params = parse_init_params(&request)?;
                child.initialize(&params)?;
                let caps = child.capabilities();
                send_response(&mut stdout, &request, &caps)?;
            }
            "pipe/fetch" => {
                let params = parse_fetch_params(&request)?;
                let mut emitter = FactEmitter::new(&mut stdout);
                match child.fetch(&params, &mut emitter) {
                    Ok(result) => send_response(&mut stdout, &request, &result)?,
                    Err(e) => send_error(&mut stdout, &request, &e)?,
                }
            }
            "pipe/health" => {
                match child.health() {
                    Ok(status) => send_response(&mut stdout, &request, &status)?,
                    Err(e) => send_error(&mut stdout, &request, &e)?,
                }
            }
            "pipe/shutdown" => {
                send_response(&mut stdout, &request, &json!({}))?;
                break;
            }
            _ => send_error(&mut stdout, &request,
                &PipeError::Fatal { message: "Method not found".into() })?,
        }
    }
    Ok(())
}
```

### 7.3 Future Transports

HTTP+SSE and Streamable HTTP follow the same pattern as MCP
transports — same JSON-RPC messages, different wire. Not implemented
until there's a concrete use case (remote child on VPS, shared child
serving multiple Mothers).

## 8. Security Model

### 8.1 Three Layers (All Children)

1. **Protocol enforcement**: Mother validates facts against declared
   schemas. Child can only emit what its manifest allows. Undeclared
   schemas → fact dropped.

2. **Capability manifest**: child.toml declares domains, schemas,
   auth requirements. Mother refuses to spawn a child requesting
   undeclared resources. Validated at load time AND call time.

3. **Runtime sandbox**:
   - WASM: wasmtime sandbox. All I/O proxied through host functions.
   - Native: OS sandbox (macOS sandbox_init(), Linux Landlock).
     No filesystem access, no process spawning. Network currently
     restricted to port 443 + DNS (port-level). [[spec-pipe-mother-io]]
     tightens to deny all outbound sockets — children use Mother's
     pipe/http proxy for domain-enforced networking.

### 8.2 Credential Security

- Credentials stored in age-encrypted vault (Keychain + Touch ID)
- Mother resolves credentials and passes via pipe/initialize (stdin)
- Credentials never in environment variables or files for children
- OS sandbox prevents native children from accessing vault directly
- WASM sandbox prevents WASM children from accessing filesystem
- Credential leak detection: Mother scans emitted fact data for
  credential values (pattern from host_support.rs:leak_check)

### 8.3 Native Child Sandbox Detail

Both platforms use in-process kernel APIs, not external CLI tools:

- **macOS**: `sandbox_init()` C API (not the deprecated `sandbox-exec`
  CLI). Applied after fork, before exec. Same `.sb` Scheme profile
  format. Kernel mechanism is not deprecated.
- **Linux**: Landlock ABI v4+ (kernel 6.7+). Applied via `landlock`
  crate after fork, before exec. Equivalent filesystem/network
  restrictions.

**Sandbox profiles are parameterized by child type.** Different
children have different security needs:

```
Connector profile (deny-all filesystem):
  - deny all filesystem access
  - allow stdin/stdout/stderr
  - allow outbound network on port 443 (HTTPS) and port 53 (DNS)
  - port-level only — OS sandboxes cannot filter by hostname
  - domain-level filtering enforced inside Mother via pipe/http
    ([[spec-pipe-mother-io]]) — which also tightens sandbox to
    deny all outbound sockets

Storage child profile (scoped filesystem):
  - deny all filesystem access EXCEPT a Mother-provided path
  - allow read/write to a specific directory (e.g., lake root)
  - allow stdin/stdout/stderr
  - deny all outbound network (storage children don't call APIs)
  - Mother provides the scoped path at spawn time from child.toml
    type + destination config; the OS enforces the boundary
```

Mother determines the sandbox profile at spawn time:
1. Read `child.toml` type (connector, lakehouse, transform, etc.)
2. For connectors: deny-all filesystem, allow network
3. For storage children (lakehouse): scoped filesystem access to
   the destination path, deny network
4. Pass the allowed path to sandbox generator

**Implementation:** macOS gets `(allow file-read* file-write*
(subpath "<lake_path>"))` added to the SBPL profile. Linux gets
a Landlock `PathBeneath` rule for the allowed directory. The
sandbox APIs already accept parameters — `generate_macos_profile`
and `apply_landlock` gain a `allowed_paths: &[&Path]` parameter.

**Role alignment test:**
1. Can connector be replaced without touching storage? Yes — different sandbox profiles.
2. Can lakehouse be replaced without touching connector? Yes — different children.
3. Mother governing, not executing? Yes — Mother configures sandbox at spawn time (policy), OS enforces (execution).

**Fail behavior**: If the OS cannot enforce sandboxing (unsupported
kernel, API error), Mother refuses to spawn the child unless
`--no-sandbox` is explicitly passed. No silent degradation.
`--no-sandbox` is [[spec-mother-broker]] scope.

Cost: ~2ms startup, ~0ns runtime (kernel-enforced, no per-call
overhead). Chrome renderer process pattern.

### 8.4 Future: UCAN Capability Tokens

When persona-federation ships keypair infrastructure, credentials
can be delegated as UCAN tokens:

```
persona keypair → sign UCAN token
  → scope: api.github.com, read:issues, read:prs
  → audience: github-connector child
  → duration: 1 hour
  → child presents token to source API
```

This replaces raw credential passing with cryptographically scoped
delegation. The child proves it has permission without holding the
raw secret. Future scope — not in initial implementation.

## 9. Crate Structure

### 9.1 patina-pipe-types

Shared types used by both WASM and native children:

```
patina-pipe-types/
  src/
    lib.rs          # re-exports
    fact.rs         # Fact, FetchResult, content_hash, signature
    error.rs        # PipeError (Transient, Fatal, RateLimited, Partial)
    capabilities.rs # Capabilities, Status
    config.rs       # FetchParams, AuthConfig
    canonical.rs    # canonical_json(), content_hash()
```

Zero dependencies beyond serde, serde_json, blake3. This crate is
the protocol definition in code.

### 9.2 patina-sdk (updated)

Existing WASM SDK gains pipe protocol awareness:

```
patina-sdk/
  src/
    mother_child/
      emit.rs       # emit::emit_fact (renamed from host_emit)
      fetch.rs      # fetch::get/post (renamed from host_http)
      log.rs        # log::log (renamed from host_log)
      query.rs      # query::query (renamed from host_query)
    pipe_types.rs   # re-export patina-pipe-types
```

Breaking rename: `host_emit` → `emit`, `host_http` → `fetch`,
`host_log` → `log`. Existing plugins update their imports. The
functionality is identical — names change for clarity.

### 9.3 patina-pipe (new)

Native transport binding for native children:

```
patina-pipe/
  src/
    lib.rs          # Child trait, run() entry point
    transport.rs    # stdio JSON-RPC (future: HTTP+SSE)
    emitter.rs      # FactEmitter (streaming fact delivery)
    signing.rs      # persona keypair signing (stub until federation)
```

Depends on patina-pipe-types. Provides the `Child` trait and `run()`
orchestrator that native children call from `main()`.

## 10. Async Position

**Audit fix: async story absent.**

**Sync-first, async-optional.**

- `run()` / `serve()` are sync (blocking I/O on stdio). This matches
  the current codebase (no tokio, no async runtime).
- `Child::fetch()` is sync. Children that need async internally
  (e.g., concurrent API pagination) can use a local runtime
  (`tokio::runtime::Runtime::new()`) inside their implementation.
- Mother spawns children as processes — process-level parallelism,
  not async parallelism. Multiple children run concurrently via OS
  process scheduling.
- Future: if HTTP transport is added, the transport layer may need
  async. This is isolated to the transport binding, not the Child
  trait.

This is deliberate: sync interfaces are simpler to implement, test,
and debug. Process-level parallelism (Mother spawns N children) gives
concurrency without async complexity.

## 11. Deployment Contexts

| Context | Child Runtime | Transport | Sandbox | Identity |
|---|---|---|---|---|
| Local | Native process | stdio | OS sandbox | Persona keypair |
| Local (WASM) | wasmtime | host calls | WASM sandbox | Persona keypair |
| Remote (VPS) | Native process | HTTP+SSE | OS sandbox | Same keypair |
| Edge (CF Workers) | WASM | Streamable HTTP | WASM sandbox | Same keypair |
| P2P (other nodes) | Native process | Iroh/HTTP | OS sandbox | Node keypair |

Child code doesn't change. Transport binding and sandbox adapt.
Persona keypair provides identity everywhere.

## 12. Encryption (Acknowledged Gap)

**Audit fix: encryption entirely absent.**

Signing (persona keypair) proves WHO produced a fact.
Hashing (blake3) proves INTEGRITY of a fact.
Neither prevents READING — events.db stores plaintext JSON.

Mother is the correct encryption point — she writes to events.db,
she owns the persona keypair, she controls access.

Encryption model (future, when persona-federation ships):
1. Child emits plaintext fact (child doesn't know about encryption)
2. Mother validates, hashes, signs (over plaintext, for dedup)
3. Mother encrypts `data` field with persona key before storage
4. Storage contains: `content_hash` (cleartext for dedup),
   `signature` (cleartext for verification), `data` (encrypted)
5. Consumers decrypt with persona key on read

This ensures: dedup works across nodes (content_hash is cleartext),
verification works without decryption (signature over hash), but
data at rest is encrypted. Not in scope for initial pipe protocol.

## 13. Migration Path

### 13.1 Phase 1: Types (no runtime changes)

Ship `patina-pipe-types` with Fact, PipeError, Capabilities,
canonical_json, content_hash. Both patina-sdk and patina-pipe
depend on it. No runtime changes.

### 13.2 Phase 2: SDK Rename

Rename host_* to semantic names in patina-sdk. Update forge plugin
imports. Functionally identical — pipe protocol awareness is naming.

### 13.3 Phase 3: Native Transport

Ship `patina-pipe` with Child trait, `run()`, FactEmitter. Build
github-connector as native child. Prove native transport works.

### 13.4 Phase 4: Broker

Build Mother's routing engine. Read sources.toml, spawn children
(WASM or native), route facts to destinations. Wire into
continuous-operation scheduling.

### 13.5 Phase 5: Connections

Build `patina connect` command. OAuth device flow, vault storage,
connection config. Replace manual PAT + secret-grants workflow.

## 14. Integration Test Matrix

Verification matrix for marking child specs complete. Rows are test
scenarios; columns are OS/runtime combinations. Each cell notes expected
behavior and whether verification is automated or manual.

| Test | macOS (sandbox_init) | Linux 6.7+ (Landlock v4) | Linux <6.7 (no Landlock v4) |
|------|---------------------|-------------------------|---------------------------|
| **Native child + sandbox** | sandbox_init() applied, fetch succeeds for declared domains | Landlock v4 applied, equivalent restrictions | Spawn refused with explicit error (unless --no-sandbox) |
| **Native child + --no-sandbox** | Runs unsandboxed, warning logged | Runs unsandboxed, warning logged | Runs unsandboxed, warning logged |
| **WASM child** | wasmtime sandbox (existing) | wasmtime sandbox (existing) | wasmtime sandbox (existing) |
| **pipe/initialize handshake** | Verify capability exchange + auth delivery | Same | Same |
| **pipe/fetch streaming** | Verify O(1) memory — 1000+ facts, no accumulation | Same | Same |
| **Content-hash dedup** | Emit same fact twice, verify 1 row in events.db | Same | Same |
| **Sandbox port enforcement** | Non-443 port → EACCES on connect() | Non-443 port → EACCES | N/A (spawn refused) |
| **Schema validation** | Undeclared schema → fact dropped, logged | Same | Same |
| **Cursor transactionality** | Kill Mother mid-write, verify no partial state | Same | Same |

**Automated** (`cargo test`): pipe/initialize handshake, pipe/fetch
streaming, content-hash dedup, schema validation (using test-child).

**Manual** (requires real environment): sandbox domain enforcement
(requires OS sandbox), cursor transactionality (requires crash
simulation), native child + sandbox on each OS (platform-specific).

**Requires real API**: GitHub connector end-to-end (needs GitHub PAT or
OAuth token, hits api.github.com). Use a test repo with known issue/PR
counts for deterministic verification.

## 15. Risk Log

External assumptions that could break during implementation. Consolidated
from open questions across child DESIGN.md files.

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|-----------|
| **GitHub OAuth App registration** (patina-connect OQ#1) | Blocks OAuth testing — code works with placeholder client_id but can't verify real device flow | Medium — requires account decision (personal vs org) | Register early. Code against placeholder, swap client_id when registered. <5 min task. |
| **Landlock ABI v4 availability** (pipe-native-transport) | Linux users on older kernels can't run native children without --no-sandbox | Low for target distros — Ubuntu 24.04 ships 6.8, Fedora 39+ ships 6.5+ | Document minimum kernel version. Fail-hard with clear error. --no-sandbox as escape hatch. Ubuntu 22.04 LTS (kernel 5.15) is explicitly unsupported for sandboxed children. |
| **sandbox_init() longevity** (pipe-native-transport RQ#1) | Apple could change behavior of the C API in future macOS | Low — kernel mechanism not deprecated, only CLI wrapper was | Monitor macOS release notes. The FFI layer is ~30-50 lines, cheap to adapt. Wrapper abstraction isolates the call site. |
| **blake3 dependency** (pipe-protocol-types OQ#1) | New dependency not currently in tree | Low — blake3 is no-std compatible, pure Rust fallback, widely used | Confirm acceptable before adding. Alternative: sha2 is already in tree via age, but blake3 is faster and already specified in architecture DESIGN.md. |
| **chrono dependency** (github-connector OQ#1) | New dependency for cursor timestamps | Low — chrono is common, but adds compilation time | Alternative: use `updated_at` from fetched items as cursor (no new dep, more precise). Decision deferred to implementation session. |
| **patina-sdk crate structure** (pipe-protocol-types pre-impl note) | SDK may use wit-bindgen macros instead of explicit module files — rename plan may need adjustment | Medium — haven't inspected SDK internals yet | Read `plugins/sdk/src/` before implementing rename. The module layout determines whether it's a simple rename or requires macro changes. |

## 16. Future Scope (Explicitly Marked)

These are NOT in the initial implementation. They are noted here
so the protocol design doesn't preclude them:

- **UCAN capability tokens**: scoped credential delegation
- **MessagePack-RPC**: binary drop-in for JSON-RPC (performance)
- **QUIC transport**: multi-stream, p2p, no head-of-line blocking
- **Encryption at rest**: Mother-side, persona-key encrypted storage
- **Cross-node routing**: Mother-to-Mother fact sync via Iroh
- **Community child registry**: discovery and installation
- **WIT code generation**: generate Rust/TS/Python types from .wit

## Key Files (Implementation Reference)

**Existing code that embodies pipe protocol today:**
- `src/plugin/internal/host_support.rs` — emit_fact IS pipe/fact
  over WASM transport. Security patterns (validate, hash, check)
  apply to all transports.
- `src/mcp/server/mod.rs` — stdio JSON-RPC 2.0 server. The native
  transport binding follows this exact pattern.
- `src/secrets/mod.rs` — vault, identity, session caching. Reused
  by connection model.
- `plugins/forge/src/github.rs` — first child implementation.
  Migrates to native with `host_http` → `reqwest`.
- `plugins/forge/plugin.toml` — child manifest prototype.

**New code to build:**
- `crates/patina-pipe-types/` — shared types (Fact, Error, etc.)
- `crates/patina-pipe/` — native transport (Child trait, run())
- `children/github-connector/` — first native child
- `src/broker/` — Mother routing engine
- `src/connect/` — connection management (`patina connect`)
- `src/commands/connect.rs` — CLI commands
- `src/commands/mother.rs` — `patina mother run/status/health/logs`
