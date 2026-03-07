# Design: Pipe Architecture — Protocol + Broker Model

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

```
pipe/initialize     →  Capability exchange. Mother sends config,
                       child responds with capabilities.

pipe/fetch          →  Request data. Mother sends params (types,
                       since, limit). Child streams facts back as
                       notifications, then sends result summary.

pipe/emit           →  Push a fact. Child → Mother. Used in stream
                       mode for continuous emission.

pipe/health         →  Connectivity check. Returns ok/degraded/down
                       with latency and message.

pipe/capabilities   →  Re-query capabilities (for long-lived children).

pipe/shutdown       →  Graceful shutdown request. Child flushes
                       pending work, then exits.
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
hash, write to events.db. No accumulation. A child emitting 100K
facts uses O(1) memory on both sides.

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

Pipe protocol errors use JSON-RPC error codes with structured data:

```rust
// patina-pipe-types/src/error.rs

/// Error categories for pipe protocol.
pub enum PipeError {
    /// Retryable — transient failure, try again later.
    /// Mother should backoff and retry.
    Transient {
        code: i32,          // -32001
        message: String,
        retry_after_ms: Option<u64>,
    },

    /// Fatal — permanent failure, don't retry.
    /// Bad credentials, schema mismatch, invalid config.
    Fatal {
        code: i32,          // -32002
        message: String,
    },

    /// Rate limited — source API throttling.
    /// Mother should wait and retry.
    RateLimited {
        code: i32,          // -32003
        message: String,
        retry_after_ms: u64,
    },

    /// Partial — some facts emitted, then failure.
    /// Mother keeps what it got, retries the rest.
    Partial {
        code: i32,          // -32004
        message: String,
        emitted: u64,       // facts successfully sent before failure
    },
}
```

JSON-RPC error code ranges:
- `-32001`: Transient (network timeout, service unavailable)
- `-32002`: Fatal (bad auth, schema not found, invalid config)
- `-32003`: Rate limited (429 from source API)
- `-32004`: Partial success (some facts emitted before failure)
- `-32600` to `-32700`: Standard JSON-RPC errors (parse, method, etc.)

### 1.6 Fact Signing and Content Addressing

Every fact emitted through pipe protocol is:

1. **Canonicalized**: `data` field serialized via canonical JSON
2. **Hashed**: blake3 over canonical bytes → `content_hash`
3. **Signed**: ed25519 signature over `content_hash` using persona
   keypair → `signature`

The SDK handles all three automatically. Child authors never touch
crypto. Mother validates signature and checks content_hash for dedup
before writing to events.db.

```
Child calls emit("github", "issue", data)
  → SDK: canonical_json(data) → bytes
  → SDK: blake3(bytes) → content_hash
  → SDK: ed25519_sign(content_hash, persona_key) → signature
  → SDK: send pipe/fact notification with all fields
  → Mother: verify signature, check content_hash for dedup
  → Mother: write to events.db (or skip if duplicate)
```

Note: persona keypair infrastructure depends on
[[spec-persona-federation]]. Until that ships, signing is stubbed
(content_hash still works for dedup, signature field is empty).

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
- Sandboxed by OS (macOS sandbox-exec, Linux Landlock)
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

    fn health(&self, params: &FetchParams) -> Result<Status, PipeError> {
        let client = GitHubClient::new(&params.auth)?;
        client.check_rate_limit()
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

### 4.2 Fan-Out

Multiple destinations can reference the same connection. Mother
optimizes:

**Separate spawns** (default, simple):
```
sources.toml (project A): connection = "github", types = ["issues"]
sources.toml (project B): connection = "github", types = ["prs"]
→ Mother spawns github-connector twice with different params
```

**Shared spawn** (optimization, when params overlap):
```
sources.toml (project A): connection = "github", types = ["issues", "prs"]
sources.toml (lake):      connection = "github", types = ["issues", "prs"]
→ Mother spawns once, routes facts to both destinations
→ Content-hash dedup handles any overlap
```

Mother decides strategy. Children don't know about fan-out.

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

## 5. Persona Enforcement

**Audit fix: hand-waved persona enforcement.**

### 5.1 Persona Scoping

Every pipe protocol interaction is scoped to a persona:

- `pipe/initialize` includes `persona` field
- All emitted facts carry `persona` in the notification
- Mother routes facts only to destinations owned by the same persona
- Cross-persona routing is denied at the broker level

### 5.2 Enforcement Points

| Point | What | How |
|---|---|---|
| Config delivery | Child receives persona ID | pipe/initialize params |
| Fact validation | Fact's persona matches session | Mother checks on receipt |
| Storage routing | Facts go to persona's namespace | Mother routes by persona |
| Cross-persona | Denied | Mother refuses to route |
| Multi-persona | Isolated namespaces | Mother runs separate sessions |

### 5.3 Multi-Persona on One Node

Mother supports multiple personas on the same machine (e.g., work
persona and personal persona). Each persona:

- Has its own connections, destinations, credentials
- Has its own events.db namespace
- Has its own belief set
- Cannot see facts from other personas

The persona is set at the destination level in sources.toml:
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
pub fn run<C: Child>(child: C) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        let request: Request = serde_json::from_str(&line?)?;
        match request.method.as_str() {
            "pipe/initialize" => {
                let config = parse_init_params(&request)?;
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
                let params = parse_health_params(&request)?;
                match child.health(&params) {
                    Ok(status) => send_response(&mut stdout, &request, &status)?,
                    Err(e) => send_error(&mut stdout, &request, &e)?,
                }
            }
            "pipe/shutdown" => {
                send_response(&mut stdout, &request, &json!({}))?;
                break;
            }
            _ => send_error(&mut stdout, &request,
                &PipeError::Fatal { code: -32601, message: "Method not found".into() })?,
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
   - Native: OS sandbox (macOS sandbox-exec, Linux Landlock).
     No filesystem access, no process spawning, no arbitrary network.
     Only inherited stdio + declared domains.

### 8.2 Credential Security

- Credentials stored in age-encrypted vault (Keychain + Touch ID)
- Mother resolves credentials and passes via pipe/initialize (stdin)
- Credentials never in environment variables or files for children
- OS sandbox prevents native children from accessing vault directly
- WASM sandbox prevents WASM children from accessing filesystem
- Credential leak detection: Mother scans emitted fact data for
  credential values (pattern from host_support.rs:leak_check)

### 8.3 Native Child Sandbox Detail

```
macOS sandbox-exec profile:
  (deny default)
  (allow network-outbound
    (remote tcp (require-all
      (regex #"^api\.github\.com$")  ; from child.toml domains
      (port 443))))
  (allow file-read-data (subpath "/dev/stdin"))
  (allow file-write-data (subpath "/dev/stdout"))
  (allow file-write-data (subpath "/dev/stderr"))
```

Linux Landlock equivalent restricts the same surfaces.

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

## 14. Future Scope (Explicitly Marked)

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
