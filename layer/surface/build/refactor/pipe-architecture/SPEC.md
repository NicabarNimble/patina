---
type: refactor
id: pipe-architecture
status: active
created: 2026-03-06
sessions:
  origin: 20260305-170212
related:
- forge-plugin-extraction
- lake-registry
- core-extraction
- continuous-operation
- persona-federation
- mother-maturation
- scrape-simplification
beliefs:
- wit-is-contract-wasm-is-one-runtime
- patina-is-domain-agnostic-knowledge-system
- pipes-are-processes-not-wasm
- host-proxied-io-is-the-security-model
- mother-holds-connections-pipes-transform
- pipe-protocol-is-transport-agnostic
- persona-keypair-is-node-identity
- wit-defines-pipe-contract-not-runtime
- connectors-own-tables-schemas-are-contracts
exit_criteria:
- id: children-complete
  text: All child specs (pipe-protocol-types, pipe-native-transport, github-connector, patina-connect, mother-broker) are complete
  checked: false
---
# refactor: Pipe Architecture — Protocol + Broker Model

> Pipe is the protocol — how Patina components exchange data. Like
> HTTP is a protocol, not a server. You don't "build a pipe." You
> speak pipe protocol. Mother is the broker — routes facts from
> sources to destinations based on declarations. Children are the
> managed services that do the work.

## Context

### What We Learned

Sessions 5-8 (20260305-224446 through 20260306-123021) explored,
audited, and redesigned the pipe architecture through:

- **Session 5** (exploration): Discovered pipes should be processes
  not WASM, traced security model to host-side code, designed OS
  sandbox model, mapped P2P node architecture, captured 6 beliefs.
- **Session 6** (first spec write): Wrote SPEC.md and DESIGN.md
  framing pipes as "universal data flow primitive."
- **Session 7** (five-lens audit): Two independent audits found 10
  issues — 3 contradictory protocol models in the same document,
  Vec<Fact> OOM risk, broken content addressing, undefined errors,
  no delivery guarantees, hand-waved persona enforcement, encryption
  gap. Verdict: the spec had a framing error.
- **Session 8** (architecture reframe): Through design discussion,
  discovered the fundamental framing error and arrived at the model
  this spec describes.

### The Framing Error

The original spec defined "pipe" as a process/binary (GitHubPipe,
SlackPipe, LakeReaderPipe). Every arrow in the data architecture
was "a pipe." This led to:

- Internal transforms (lake → block) being called "pipes" even
  though they break the OS sandbox model (need filesystem access)
- "Universal data flow primitive" scope drift — if everything is
  a pipe, the word means nothing
- 3 contradictory protocol descriptions in one document (NDJSON,
  JSON-RPC, MCP) because "pipe" was too vague to have one protocol

### The Correction

Through design discussion in session 8, we discovered:

- **Pipe is the PROTOCOL** — how components exchange data. Like HTTP
  is a protocol, not a server. JSON-RPC 2.0 + WIT types.
- **Children are the managed services** — connector, transport,
  lakehouse, transform. All speak pipe protocol. Mother manages them.
- **Mother is the BROKER** — Netflix/Kafka pattern. Routes facts from
  sources to destinations based on declarations. Never transforms.
- **Connection links pipe protocol to auth** — `patina connect github`
  creates credential + connector child config in one command.
- **host_emit IS already pipe protocol** — it emits typed,
  schema-validated facts over WASM host function calls. This spec
  formalizes that and adds a native transport (stdio/HTTP). Not a
  revolution — naming what exists and extending it.

## Current State

**What exists and works (infrastructure survives):**
- `host_emit` → schema-validated event emission to events.db
  (`src/plugin/internal/host_support.rs:emit_fact`)
- `host_http` → domain-allowlisted HTTP with credential injection
  (`src/plugin/internal/host_support.rs:http_get/http_post`)
- CQRS projection → events.db → patina.db materialized views
- FTS5 indexing from projection tables
- Age-encrypted vault with macOS Keychain + Touch ID
- Mother child plugin framework (spawn, heartbeat, health)
- MCP server over stdio JSON-RPC 2.0 (`src/mcp/server/mod.rs`)
- Forge GitHub connector: issues + PRs via REST API
  (`plugins/forge/src/github.rs`)

**What the spec changes:**
- "Pipe" goes from meaning "a binary" to meaning "the protocol"
- "Child" expands from "WASM plugin in mother-child world" to
  "any managed service of Mother" (WASM or native)
- Mother gains broker responsibilities (routing, fan-out, scheduling)
- Connections replace manual credential setup
- Destinations declare what they want (pub/sub)
- host_emit becomes one transport (WASM) for the universal protocol

## Target State

### Patina Core = The Belief Loop

Patina is not a data pipeline. Patina is a knowledge system. The core:

- **scrape** — capture reality (code, git, sessions)
- **oxidize** — make it understandable (embeddings, projections)
- **scry** — find by meaning (semantic vector search)
- **assay** — know the structure (modules, imports, FTS5)
- **context** — synthesize the worldview (patterns + beliefs)
- **beliefs** — what Patina thinks is true (THE PRODUCT)

This loop is currently dev-domain-specific. [[spec-core-extraction]]
makes it domain-agnostic. Pipe protocol feeds it from any source.
Projects and apps both run this loop. Apps = agents (LLM in the
loop, act on beliefs).

Everything below beliefs is plumbing. **Beliefs are the exit layer.**
The only thing Patina exports. Data can sync across Mothers on the
same persona but CANNOT cross persona boundaries.

### Data Layers

```
External Sources (GitHub, Slack, APIs)
  | connector children (speak pipe protocol)
  v
Data Lakes (Parquet, raw/complete, managed by lakehouse children)
  | transform children (speak pipe protocol)
  v
Data Blocks (curated datasets, output of transforms)
  | pipe protocol
  v
Projects / Apps (run Patina core loop)
  | scrape -> oxidize -> scry/assay -> context -> beliefs
  v
Beliefs (THE EXIT LAYER -- syncs across Mothers per persona)
```

Every arrow speaks pipe protocol. Every connection has a credential.
Every fact type is WIT-defined. Data is also created internally —
sessions, decisions, code changes create facts inside projects that
feed the belief loop alongside external facts.

### Separation of Concerns

Four actors. Mother has a dual role — she both manages children AND
routes facts. Pipe protocol and connections are properties of
interactions between actors, not actors themselves.

```
  Connector Child        Mother (manage)      Mother (route)       Consumer
  bridge to external     spawn, monitor       match source→dest    subscribe
  hold connection        resolve auth         fan-out by decl      project/index
  transform to facts     validate manifest    schema validate      act on beliefs
       |                      |                    |                    |
  doesn't:               doesn't:             doesn't:             doesn't:
  route                  transform            hold connections     know source
  know destinations      know data format     know data format     know transport
  schedule               hold connections     transform            fetch
```

**Pipe protocol** (the language) and **connections** (pipe + auth)
cut across all actors — they're how actors interact, not what they do.

**The independence test:** can you replace any piece without touching
the others?
- Swap GitHub API v3 → v4? Change connector child only.
- Swap WebSocket → SSE? Change transport child only.
- Swap S3 → R2? Change lakehouse child only.
- Add new destination? Change Mother routing config only.
- Change what you do with data? Change consumer only.
- Swap OAuth → API key? Change connection config only.
- Swap JSON-RPC → MessagePack-RPC? Change pipe protocol only.

### Pipe Protocol

The pipe protocol is how all Patina components exchange facts.
JSON-RPC 2.0 + WIT type contracts.

- **Foundation**: JSON-RPC 2.0 (stable RFC, universal, won't change)
- **Methods**: `pipe/initialize`, `pipe/fetch`, `pipe/ingest`,
  `pipe/health`, `pipe/shutdown` (initial scope); `pipe/emit`,
  `pipe/capabilities` (future stream mode) — self-owned, not
  dependent on MCP. `pipe/fetch` is Mother→connector (child streams
  facts back). `pipe/ingest` is Mother→storage child (Mother sends
  bounded record batches down).
- **Types**: WIT-defined fact shapes, capability declarations, config
  schema — compile-time type safety across languages
- **Transports**: WASM host calls (current, via patina-sdk), native
  stdio (new, via patina-pipe), HTTP+SSE (future, remote children)
- **Signing**: Every fact signed with persona keypair (automatic)
- **Hashing**: Every fact content-addressed with blake3 over canonical
  serialization (deterministic key ordering) for dedup across nodes
- **Streaming**: Facts delivered as individual JSON-RPC notifications,
  not collected into Vec — no OOM risk on large datasets
- **Errors**: Typed error variants (retryable vs fatal) with JSON-RPC
  error codes

MCP-compatible (JSON-RPC is JSON-RPC, an MCP client can talk to a
child) but MCP-independent (we own our methods, our types, our
evolution). host_emit IS pipe protocol over WASM transport — the
existing forge connector already speaks pipe protocol, it just
doesn't know it yet.

### Child Taxonomy

Children are all managed services of Mother. All speak pipe protocol.
All have lifecycle management (spawn, health, restart, shutdown).

| Type | Purpose | Examples |
|---|---|---|
| **Connector** | Bridge external sources | GitHub, Slack, RSS |
| **Transport** | Hold complex connections | WebSocket, webhook listener |
| **Lakehouse** | Manage data storage | Parquet across local/S3/remote |
| **Transform** | Curate and enrich data | Filter, embed, aggregate |

Children can be WASM components (current mother-child world) or
native processes (new, over stdio). The pipe protocol is the same
either way — Mother doesn't care how a fact arrived.

### Mother as Broker

Mother is the node. One Mother per machine. Mother routes facts from
sources to destinations based on declarations.

Mother does:
- Spawn and monitor children (lifecycle management)
- Resolve credentials (vault → config, session caching)
- Route facts from children to declared destinations (fan-out)
- Validate facts against declared schemas
- Write valid facts to destination: project events.db (direct) or
  route to lakehouse child via pipe/ingest (lake destination)
- Schedule children (poll intervals, stream health, manual triggers)
- Multi-persona isolation (separate data namespaces)
- P2P sync with other Mothers via Iroh (same persona, belief sync)

Mother does NOT:
- Transform data (that's what transform children do)
- Hold external connections directly (transport children do that)
- Know the internal format of facts (schema validation only)
- Make decisions about data (consumers do that)

### Connection Model

A connection links pipe protocol to auth. One command creates
everything needed:

```
patina connect github
  -> OAuth device flow (browser popup, user approves)
  -> Token stored in vault: github:user
  -> Connector child configured in sources.toml
  -> Done. GitHub data flows on next scrape/schedule.
```

Connections are referenced by name in destination configuration.
Evolution of `patina secrets` into `patina connect` — same vault,
better UX, one command instead of four manual steps.

### Destination Declarations (Pub/Sub)

Destinations declare what they want. Mother routes accordingly.
Fan-out is config, not child logic.

```toml
# .patina/sources.toml (project-level destination)

[sources.github]
connection = "github"
params = { owner = "NicabarNimble", repo = "patina" }
types = ["issues", "prs"]
schedule = "on-scrape"

[sources.slack]
connection = "slack"
params = { channels = ["#dev", "#incidents"] }
types = ["messages"]
schedule = "hourly"
```

Destination types: projects (`.patina/` in a repo), data lakes
(`~/.patina/lakes/<name>/`), data blocks (`~/.patina/blocks/<name>/`).

### One Protocol, Multiple Transports

| Transport | Binding | Who Uses It |
|---|---|---|
| WASM host calls | `patina-sdk` crate | Current plugins (mother-child world) |
| Native stdio | `patina-pipe` crate | New native children |
| HTTP+SSE | Future | Remote children on VPS |
| Streamable HTTP | Future | Shared children serving multiple Mothers |

Same JSON-RPC methods. Same fact schema. Same signing and hashing.
Different wire. The existing `host_emit` call IS the WASM transport
binding — renamed to `emit` for clarity (host_* prefix is WASM
jargon that doesn't communicate to LLMs or humans).

Crate structure:
- `patina-pipe-types` — shared types (Fact, Error, Capabilities),
  used by both WASM and native children
- `patina-sdk` — WASM transport binding (current, gains pipe
  protocol awareness)
- `patina-pipe` — native transport binding (new, stdio JSON-RPC)

### Security Model

Three layers, always active for all children:

1. **Protocol enforcement**: Mother validates facts against declared
   schemas. Child can only emit what its manifest allows.
2. **Capability manifest**: child.toml declares domains, schemas,
   auth requirements. Mother refuses undeclared resources.
3. **Sandbox**: WASM sandbox (current plugins) or OS sandbox
   (native children — macOS sandbox_init(), Linux Landlock).

Native children: OS sandbox prevents filesystem access and process
spawning. Network allowed for declared domains only. Credentials
arrive via stdin config, not environment or files. ~2ms startup,
~0ns runtime — Chrome renderer process pattern.

WASM children: wasmtime sandbox (existing model). All I/O proxied
through host functions. Proven by forge connector.

Future: UCAN capability tokens for scoped credential delegation.

### Deployment Contexts

Same child binary works everywhere. Transport adapts:

| Context | Runtime | Transport |
|---|---|---|
| Local (your machine) | Native process | stdio |
| Local (WASM) | wasmtime | host calls |
| Remote (VPS) | Native process | HTTP+SSE |
| Edge (Cloudflare) | WASM Worker | Streamable HTTP |
| P2P (other nodes) | Native process | Iroh/HTTP |

Child code doesn't change across contexts. Persona keypair provides
identity in all of them.

### Lifecycle Modes

- **Poll**: spawn → fetch → emit facts → exit. Schedule-driven
  (hourly, daily, on-scrape). Ephemeral.
- **Stream**: spawn → stay alive → emit facts continuously. Mother
  monitors health, restarts on crash. Long-lived.
- **Manual**: one-shot on user command (`patina mother run github`).
  For testing, backfill, debugging.

For real-time sources with complex connections (WebSockets, webhooks):
transport children hold the connection, buffer data, and feed it to
connector children via pipe protocol. The connector doesn't know
about WebSocket — it transforms structured input into facts.

### Multiplexing

Children don't multiplex. Each child handles one concern. Multiplexing
happens at the architecture level:

| Level | Who | How |
|---|---|---|
| Source connection | Transport child | Demux one connection → many children |
| Process management | Mother | Many children → many destinations |
| Network transport | Future | QUIC/HTTP/2 multi-stream between nodes |

### Backpressure

- **stdio**: OS pipe buffer (~64KB). Mother stops reading → buffer
  fills → child's write() blocks. Free, kernel-provided.
- **HTTP**: Pull-based. Mother controls `limit` per fetch call.
- **WASM**: Host function call is synchronous. Natural backpressure.

## What Happens to forge-plugin-extraction

The forge WASM plugin proved the infrastructure:
- host_emit works (events.db, provenance, schema validation)
- The security model works (domain allowlist, credential injection)
- The projection pipeline works (CQRS, FTS5)

These survive unchanged. What changes:

1. The forge connector can stay WASM (using patina-sdk with pipe
   protocol awareness) or become a native binary (using patina-pipe).
   The protocol is the same either way.
2. `host_emit` is recognized as the WASM transport for pipe protocol.
   Renamed: `host_emit` → `emit`, `host_http` → `fetch`,
   `host_log` → `log`.
3. Manual PAT + secret-grants.toml → `patina connect github` with
   OAuth device flow.
4. Mother routes forge facts to destinations based on declarations
   instead of forge writing directly to project eventlog.

The GitHub connector code (`plugins/forge/src/github.rs`) migrates
with minimal changes: `host_http` calls become either renamed SDK
calls (WASM) or direct `reqwest` calls (native).

## Relationship to Other Specs

- **[[spec-forge-plugin-extraction]]** (active) — proved the pattern.
  Pipe architecture formalizes the protocol that host_emit already
  implements. Forge connector is the first child to gain pipe
  protocol awareness.
- **[[spec-core-extraction]]** (active) — children are NOT core.
  They're user-level services managed by Mother. Core is protocol +
  stores.
  `role=connector` in a plugin manifest = this plugin IS a Mother
  child.
- **[[spec-continuous-operation]]** (draft) — Mother daemon manages
  child scheduling. The "belief stream router" IS the broker pattern.
  Should adopt pipe protocol vocabulary.
- **[[spec-lake-registry]]** (draft) — lakes are destination types.
  Lakehouse children manage storage. Lake metadata in graph.db.
- **[[spec-persona-federation]]** (draft) — dependency. Persona
  keypair = signing key + node identity + UCAN issuer. The identity
  primitive that makes pipe protocol work across nodes.
- **[[spec-mother-maturation]]** (draft) — the container spec. Mother
  as broker is the unifying model for all its responsibilities.
- **[[spec-scrape-simplification]]** (draft) — scrape stays local
  (git). External data arrives via connector children, not scrape.
  `patina scrape` triggers projection of pipe-emitted events.

## Resolved Questions

1. **Pipe = process or protocol?** → Protocol. JSON-RPC 2.0 + WIT.
   Children are the processes/components. (Session 8)

2. **WASM or native?** → Both. One protocol, multiple transports.
   WASM children via patina-sdk, native children via patina-pipe.
   (Sessions 5, 8)

3. **Mother's role?** → Broker. Routes facts from sources to
   destinations based on declarations. Never transforms data.
   (Session 8, Netflix/Kafka pattern)

4. **Internal transforms?** → Children, not pipes. Transform
   services are Mother-managed children that speak pipe protocol.
   Internal transforms should not break the sandbox model.
   (Session 7 audit finding)

5. **Streaming sources?** → Transport children hold connections.
   Connector children transform data. Pipe protocol is the same
   for both. (Session 5, 8)

6. **Delivery guarantees?** → At-least-once with content-addressed
   dedup. blake3 hash over canonical serialization. Same fact from
   two sources resolves to one entry. (Session 7 audit finding)

7. **host_emit naming?** → host_* prefix is WASM jargon. Rename:
   `host_emit` → `emit`, `host_http` → `fetch`, `host_log` → `log`.
   Names should be self-describing for LLMs and humans. (Session 8)

8. **Schema ownership?** → Schema ships with the child (schema.toml
   alongside child binary). Manual install during development (copy
   to `.patina/schemas/<name>/`). Auto-install from manifest is
   future work. Three-part resolution: manifest declares intent,
   schema.toml defines structure, broker validates against it.
   (Session 14, DESIGN.md §4.5)

9. **WASM-to-native migration path?** → Forge stays WASM, gains pipe
   protocol awareness via patina-sdk updates (host_emit → emit
   rename). Native github-connector coexists to prove both runtimes.
   No forced migration — WASM path continues to work. (Sessions 5, 8)

10. **Encryption?** → Mother encrypts at the storage boundary.
   Signing proves WHO. Hashing proves INTEGRITY. Encryption (future)
   provides CONFIDENTIALITY. Not in scope for initial pipe protocol
   — acknowledged gap, addressed when persona-federation ships
   keypair infrastructure. (Session 7 finding)

## Open Questions

1. **Child discovery.** How do users find and install community
   children? Registry? GitHub releases? Manual download?

2. **Child versioning.** When output format changes, how do
   projections migrate? Schema version in event metadata?

3. **Multi-provider children.** One github connector for all GitHub
   instances, or separate? Lean: one, configured with base_url.

## Discovery Notes (pushed from other specs)

### mother-child.wit alignment (from [[raw-lake-ingestion]] session 20260308-164629)

`mother-child.wit` defines a generic daemon world (init, load,
unload, health, handle, tick) that predates the pipe protocol
design. Native children speak pipe protocol (JSON-RPC: initialize,
fetch, health, shutdown). WASM children use mother-child.wit with
handle(action, payload) dispatching pipe methods.

The two interfaces are functionally equivalent (handle() dispatches
pipe methods) but nominally unaligned. Two options:

1. mother-child.wit evolves to export pipe protocol methods directly
   (initialize, fetch, ingest, health, shutdown)
2. mother-child.wit is deprecated; new `pipe-child.wit` matches pipe
   protocol; legacy WASM children continue on mother-child.wit

Not blocking — the lakehouse child is native. Track for resolution
when pipe-protocol-types or pipe-native-transport specs address
WASM/native convergence.

### pipe/ingest method (from [[raw-lake-ingestion]]) — RESOLVED

pipe/ingest is now a first-class method in DESIGN.md §1.2 with
direction annotation (Mother → storage child). Hard specification
remains in [[raw-lake-ingestion]] DESIGN.md. When pipe-protocol-types
crate is implemented, `IngestParams`, `IngestRecord`, `IngestResult`,
and `IngestProvenance` types should be added alongside existing types.

### host.wit emit routing (from [[raw-lake-ingestion]])

The `emit` interface in host.wit describes fact emission that
"writes to events.db." The WIT contract itself doesn't need to
change (children emit, Mother routes), but the host implementation
must gain destination awareness when raw-lake-ingestion ships.
When source has `destination.type = "lake"`, emit routes to
lakehouse child instead of events.db. Track in [[mother-broker]]
or [[raw-lake-ingestion]].

## Children

This is a **container spec** — the architecture reference that child
specs point to for design decisions. Implementation happens in the
child specs below, not here.

| Spec | What it delivers | Build order |
|------|-----------------|-------------|
| [[spec-pipe-protocol-types]] | `patina-pipe-types` crate (Fact, PipeError, Capabilities, canonical_json), child.toml manifest format, patina-sdk rename (host_* → semantic names) | First (foundation, no blockers) |
| [[spec-pipe-native-transport]] | `patina-pipe` crate (Child trait, run(), FactEmitter, stdio JSON-RPC), OS sandbox profile | Second (blocked by protocol-types) |
| [[spec-pipe-mother-io]] | Mother-side pipe/http proxy + sandbox tightening (no outbound sockets), patina-pipe HTTP helper, Measure instrumentation | Third (blocked by native-transport) |
| [[spec-github-connector]] | GitHub connector as native child, replaces src/forge/, emits github.issue/github.pr facts (own schema, not forge). WASM forge plugin coexists. | Fourth (blocked by pipe-mother-io) |
| [[spec-patina-connect]] | `patina connect` CLI with OAuth device flow, connection config, credential delivery via pipe/initialize | Parallel (no blockers — uses existing vault, independent of pipe types) |
| [[spec-mother-broker]] | Mother routing engine (sources.toml, fan-out), child lifecycle (WASM + native), schema validation, scheduling | Last (blocked by protocol-types + native-transport) |

### Build Order (dependency graph)

```
pipe-protocol-types (foundation)
  ├── pipe-native-transport
  │     └── pipe-mother-io
  │            ├── github-connector
  │            └── mother-broker
  │
patina-connect (independent — uses existing vault, no pipe type deps)
```

pipe-native-transport and patina-connect can build in parallel.
After native-transport lands, pipe-mother-io is next (tightening the
sandbox + host proxy). patina-connect has no blockers — it creates
connection configs and stores tokens using existing vault infrastructure.
Only after pipe-mother-io is complete can github-connector and
mother-broker proceed. Mother-broker tests against test-child first; its
`mother-run-github` EC is verified after github-connector is complete.

## Exit Criteria

This spec is complete when all five children are complete.
