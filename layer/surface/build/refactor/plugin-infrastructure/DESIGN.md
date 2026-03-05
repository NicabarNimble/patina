# Design: Plugin Infrastructure — Host Emit and Roles

## Why This Work Exists

Patina was becoming a swiss army knife. Forge baked in, code analysis baked
in, specs baked in — the core was fat. The architectural pivot: shrink core
to protocol + stores, everything else extends via plugins. But the plugin
system was built for old-Patina (the dev tool). It has capability boundaries
(4 WASM worlds) and a manifest system, but two critical gaps block the pivot:

1. **Plugins can't write.** The host interface is read-only — `host/query`
   reads, `host/layer` reads, `host/http` fetches. A plugin that fetches
   GitHub issues can't write those facts to the eventlog. Without a write
   path, plugins are observers, not participants. Extraction can't happen.

2. **Plugins don't declare purpose.** Manifests declare a world (capability
   boundary) and what the plugin provides (commands, pipeline ops). But they
   don't say what the plugin IS FOR. Mother can't route data to a connector
   if she can't identify which plugins are connectors. The scrape pipeline
   can't dispatch to grammars if nothing says "I'm a grammar."

This container spec builds the infrastructure that
[[spec-core-extraction]] and [[spec-mother-maturation]] depend on.
Everything starts here.

**Origin:** [[session-20260303-190855]] (forge audit revealed plugin gaps),
[[session-20260304-120702]] (refined into 3-stream spec structure).

## The Principle: WIT Is Contract, WASM Is One Runtime

[[wit-is-contract-wasm-is-one-runtime]] is the foundational design
principle for all plugin infrastructure work.

WIT defines the universal interface contracts for everything outside Patina
core. WASM is one runtime that implements those contracts — wasmtime
locally, Cloudflare Workers at the edge, native binaries for
performance-critical paths. The interface standard (WIT types, capability
grants) is the architecture. The execution environment is an implementation
detail.

**What this means concretely:**
- `host_emit` is defined in WIT. Whether the emitter is a WASM plugin,
  a native connector binary, or a cloud function, the fact schema and
  validation contract are the same.
- Plugin roles are manifest metadata. They describe purpose, not runtime.
  A connector's role is "connector" whether it runs as WASM or native.
- The first implementation targets wasmtime (the local WASM runtime).
  But nothing in the interface design assumes WASM specifically.

This principle emerged from the Cloudflare analysis in
[[session-20260304-120702]]: WASM is great for grammars (pure compute,
sandboxed) but awkward for I/O-heavy connectors and impossible for large
ML models. WIT as contract language decouples interface from runtime.

## host_emit — The Missing Write Path

### Current State

Plugins read via host imports:
- `host/query` — scry, assay, context (capability-gated)
- `host/layer` — project root, config, UID (read-only)
- `host/http` — domain-allowlisted HTTP requests
- `host/measure` — measurement reporting

Plugins write via one narrow path:
- `toys()` export — mother-child and task worlds return `list<toy>` work
  intents. Mother/host executes them. This is the only write mechanism.

There is no general-purpose fact emission. A plugin that discovers a GitHub
issue, an email thread, or a Slack message has no way to record that fact
in the eventlog.

### Design Decision: Host Import (Pattern A)

host_emit is a **host import** — a function the plugin calls during
execution. Not an intent returned after execution (Pattern B / toy
pattern).

**The reasoning (via Helland):**

Pat Helland's "Life beyond Distributed Transactions" and "Memories,
Guesses, and Apologies" provide the framework:

1. **The host is the entity that owns the eventlog.** The plugin is across
   a trust boundary (the WASM sandbox). Only the entity writes to its own
   state. But the host import pattern preserves this — the plugin calls
   `emit-fact()`, the host function validates and writes. The plugin never
   touches SQLite directly.

2. **Facts are independently valid.** If a connector emits 50 GitHub issues
   then crashes on issue 51, those 50 facts are still true. The eventlog is
   append-only — partial writes aren't corruption, they're progress. No
   batch atomicity needed. This is Helland's "apologize and compensate"
   model: every fact stands alone.

3. **The plugin gets confirmation.** `emit-fact()` returns a result — the
   event sequence number on success, an error on failure. The plugin can
   react: skip and continue, retry, or abort. This is Helland's "confirmed
   message" — the entity acknowledges receipt.

**Why not Pattern B (intent return)?**

The toy pattern (return intents, host executes later) exists because toys
spawn external processes — dangerous side effects that need host control
over timing and lifecycle. Facts are different. Writing an immutable record
to an append-only log is safe. Deferring fact writes adds complexity
(accumulate in WASM memory, return giant list) without safety benefit.

Pattern B also fails for streaming connectors. A connector processing a
WebSocket stream needs to emit facts as events arrive, not accumulate
them until the stream ends (which may be never).

### The Interface

```wit
interface emit {
    /// Emit a structured fact to the eventlog.
    ///
    /// schema: schema package (e.g., "forge")
    /// fact-type: fact within schema (e.g., "issue")
    /// data: JSON payload conforming to schema definition
    ///
    /// Returns event sequence number on success.
    /// Host validates: schema exists, fact-type exists, data shape, capabilities.
    emit-fact: func(schema: string, fact-type: string, data: string) -> result<u64, string>;
}
```

Host-side validation:
- Schema must be installed (`.patina/schemas/`)
- Plugin manifest must reference the schema (`[schemas.<name>]`)
- Fact-type must exist in schema
- Data shape must conform (field types, required fields)
- Plugin must have `host_emit` in its capabilities

The host controls what gets written. The plugin proposes. This is
[[reads-via-host-writes-via-intents]] adapted for the import pattern —
reads are direct host calls, writes are host-validated calls. The
principle (host mediates all writes) is preserved; the mechanism
(synchronous import vs deferred intent) fits the use case.

## Plugin Roles — What Plugins DO

### Worlds vs Roles

The plugin system has two axes, and today only one is expressed:

**Worlds** (capability boundary — WHAT you CAN do):
- `mother-child` — daemon-resident, heartbeat, toys, http, query
- `command` — CLI command dispatch, query, layer
- `task` — background work, http, query, toys
- `pipeline` — pure transform, log only

**Roles** (purpose — WHAT you're FOR):
- `connector` — fetches external data, emits facts
- `grammar` — parses local files into structured facts
- `extension` — adds commands, analysis, tooling
- `app` — IS the action layer, consumes beliefs, generates events

Worlds are enforced at the WIT level — the WASM component model prevents
a pipeline plugin from calling http. Roles are metadata in the manifest —
they tell the system what to do with the plugin, not what the plugin can do.

**Why roles matter:**
- Mother needs to know "this is a connector" to manage its sync schedule
  and route source data to it
- The scrape pipeline needs to know "this is a grammar" to include it
  when processing files of matching languages
- `patina plugin list` should show purpose, not just capability
- Future: role-based dispatch, role-specific lifecycle management

### Role in Manifest

```toml
[plugin]
name = "github-connector"
world = "mother-child"
role = "connector"          # NEW — purpose declaration
```

A plugin has exactly one role. The role doesn't grant capabilities — the
world does that. The role tells the system what the plugin is for.

### Roles and Worlds — Valid Combinations

Not every role/world combination makes sense:

| Role | Likely World | Why |
|------|-------------|-----|
| connector | mother-child | Continuous sync, Mother-managed lifecycle |
| connector | task | One-shot fetch, project-scoped |
| grammar | pipeline | Pure transform, no I/O needed |
| extension | command | Adds CLI commands |
| app | mother-child | Long-lived, event-generating |

Invalid combinations (e.g., grammar + mother-child) aren't blocked at the
manifest level — that's over-engineering. Doctor can warn about unusual
combinations. The manifest is declarative truth, not a constraint system.

**Grounded in:** [[code-is-not-core]] (grammars are plugins, not protocol),
[[scrape-is-local-capture]] (connectors are separate from scrape — different
role, different lifecycle).

## Connector Architecture — Three I/O Patterns

Connectors are the pipes between sources and Patina's data layers. The
architecture must support three I/O patterns, even though the first
implementation only ships request/response.

### 1. Request/Response (ship first)

```
Plugin: "GET /repos/X/issues?page=2"  →  Host: enforces allowlist,
                                           injects credentials,
                                           makes HTTPS call
                                        ←  returns response body
Plugin: parses JSON, emits facts
Plugin: "GET ...?page=3"               →  (repeat)
```

Plugin drives the pace. One request, one response, parse, emit, repeat.
Uses existing `host/http` interface. GitHub REST API, Salesforce, most
SaaS integrations. Forge extraction proves this pattern end-to-end.

### 2. Polling (works today)

```
Every N seconds:
  Mother calls tick() on connector
  Plugin: requests latest data via host/http
  Plugin: diffs against stored cursor/checkpoint
  Plugin: emits new facts
```

Same request/response mechanism on a schedule. The mother-child world's
`tick()` heartbeat already supports this. The plugin remembers its
high-water mark; each tick fetches what's new. RSS feeds, IMAP mailboxes,
S3 bucket listings.

### 3. Streaming (design for now, build later)

```
External stream  →  Mother (native, always-on)  →  buffer
                                                      ↓  tick()
                                            Plugin (WASM, batch parse)
                                                      ↓  emit-fact()
                                            Eventlog
```

Streaming sources (WebSocket, SSE, blockchain events, Slack RTM) push
data continuously. The plugin doesn't drive the pace — the source does.

**The hybrid model:** Mother opens the persistent connection natively —
she's already a long-lived daemon. She buffers incoming events. On each
`tick()`, the WASM plugin receives a batch of buffered events, parses
them, emits facts.

This separates concerns:
- **Transport** (connect, keep alive, reconnect, backpressure) → Mother, native
- **Parsing** (what do events mean, what facts to emit) → plugin, WASM, sandboxed

**Why not streaming inside WASM?** WASM execution is synchronous. A
plugin blocked on `read-next()` occupies a wasmtime instance. Fine for
one stream, bad for many. Mother is already async and always-on — she
should own transport.

**What this requires (future, not this spec):**
- A `host/stream` or buffer interface for Mother to deliver batched events
- Mother-side connection management (reconnect, health, backpressure)
- Cursor/checkpoint management across ticks

**What this spec provides that makes streaming possible:**
- `host_emit` as a host import — connectors emit per-event, not per-batch
- Connector role in manifest — Mother can manage connector lifecycle
  differently based on I/O pattern
- WIT-defined interfaces — native streaming transport uses the same fact
  types as WASM parsing

### Connector Destination Independence

A connector's interface is the same regardless of where facts land. The
connector calls `emit-fact()`. Where that fact gets written — a data lake,
a data block, a project's eventlog — is routing configuration, not
connector logic.

```
Source → connector → emit-fact() → [routing decides destination]
                                        ↓           ↓           ↓
                                      Lake        Block      Project
```

Mother manages routing for lake-level connectors. Projects manage routing
for direct connectors. The connector doesn't know or care. This maps to
the data flow architecture: sources can be consumed by lakes, blocks, or
projects/apps — the connector is the pipe, not the destination.

## How Children Relate

```
┌──────────────────────────────┐
│    plugin-infrastructure     │  ← this container
│          (container)         │
└──────────┬───────────────────┘
           │
     ┌─────┴──────┐
     ↓            ↓
┌──────────┐ ┌──────────┐
│host-emit │ │ plugin-  │
│   -wit   │ │  roles   │
│          │ │          │
│ FIRST    │ │ SECOND   │
│ (or par) │ │ (or par) │
└──────────┘ └──────────┘
```

- **[[spec-host-emit-wit]]** opens the write path. Without this, no
  plugin can emit facts. Core extraction can't start. This is the unlock.
- **[[spec-plugin-roles]]** adds vocabulary. Without this, Mother can't
  route by purpose and scrape can't dispatch to grammars by role.

Both are infrastructure. Neither delivers user-visible features. They
exist so that [[spec-core-extraction]] and [[spec-mother-maturation]]
can happen.

Build order: host-emit-wit first (or parallel if independent). Roles
don't depend on emit technically, but emit is the higher-priority unlock
because forge extraction is blocked on it.

## What's NOT In Scope

- **No connector implementations.** Forge extraction is
  [[spec-forge-plugin-extraction]] under [[spec-core-extraction]].
- **No Mother routing by role.** Role-based dispatch and connector
  management are [[spec-mother-maturation]]'s concern.
- **No new worlds.** The 4 worlds (mother-child, command, task, pipeline)
  are sufficient. New worlds would be a separate spec.
- **No streaming host interface.** The architecture supports streaming
  (hybrid model via Mother), but the `host/stream` interface is future
  work. This spec ships request/response and polling.
- **No schema validation wiring.** The `host/schema` interface exists in
  host.wit but isn't implemented in any world. Wiring it is related but
  separate work.

## Belief Anchors

**Mechanical (how plugins work):**
- [[wit-is-contract-wasm-is-one-runtime]] — the foundational principle.
  Interface standard is the architecture, runtime is implementation detail.
- [[reads-via-host-writes-via-intents]] — the pattern host_emit adapts.
  Reads are direct host calls, writes are host-validated. Principle
  preserved, mechanism adapted (import vs intent) based on Helland analysis.
- [[patina-is-knowledge-protocol]] — why plugins exist. Protocol core is
  small (capture/index/search/believe/evolve). Everything else extends
  via plugins.

**Motivational (why we're doing this):**
- [[code-is-not-core]] — code analysis is a grammar plugin, not protocol.
  Roles make this explicit.
- [[scrape-is-local-capture]] — connectors are separate from scrape.
  Different role, different lifecycle. Roles formalize this separation.
- [[beliefs-are-the-product]] — plugin infrastructure exists to make the
  belief system better. Connectors bring evidence. Grammars structure it.
  Extensions analyze it. Apps act on it.

## Key Files (Current Plugin System)

- `src/plugin/internal/mod.rs` — PluginManifest, PluginWorld, PluginProvides,
  GrantedCapabilities. Role field will be added here.
- `src/plugin/internal/host_support.rs` — host function implementations.
  host_emit implementation goes here.
- `src/plugin/internal/mother_child.rs` — WASM runtime, WasmChild adapter.
  Will import the new emit interface.
- `wit/deps/patina-host/host.wit` — host interfaces. The `emit` interface
  will be added here alongside log, layer, query, http, schema, measure.
- `wit/mother-child/mother-child.wit` — mother-child world. Will import
  the new emit interface.

## Open Questions

1. **Which worlds get host_emit access?** Likely: mother-child, command,
   task (the worlds that do work and need to record it). Unlikely: pipeline
   (pure transform — returns output, doesn't emit). But should pipeline
   plugins be able to emit metadata facts (parse stats, warnings)?

2. **Schema validation depth.** The manifest declares which schemas a
   plugin uses. host_emit validates against installed schemas. How deep?
   Minimum: schema exists, fact-type exists. Maximum: full field-level
   type checking against schema.toml definitions. The [[spec-host-emit-wit]]
   child spec resolves this — the DESIGN.md for that spec suggested
   manifest validation as baseline with depth as an exploration question.

3. **Native connector emit path.** WASM plugins call `emit-fact()` through
   the component model. Native connectors (future) need the same validation
   but through a different transport — CLI pipe, Unix socket, HTTP endpoint.
   The fact schema and validation logic are shared; the transport differs.
   Not blocking for this spec, but the validation logic should be factored
   for reuse.
