# Design: Knowledge Child Platform

## Why This Design

Patina already has two relevant systems:

1. A native Mother + broker + child runtime
2. A WASM plugin substrate with capability-gated host functions

What is missing is the thing the original children-plugin effort was
actually trying to create: a **knowledge-child platform** where Mother
is the native authority host and children are rich, isolated, resumable
WASM actors operating on Patina's knowledge system through typed host
capabilities.

The current `mother-child` world is sufficient for experimentation but
not for autonomous platform buildout:

- child business actions are stringly
- toys are shell command recipes
- state and checkpoint ownership are underspecified
- event subscriptions are absent
- graph and belief mutation are not first-class host capabilities
- compatibility between old child shapes and the desired platform is
  conceptual, not specified

This design closes that gap. The spec defines **what** the platform
ships. This design defines **how** it is built with enough precision
to implement autonomously.

## Principles

### 1. Mother owns authority, state, and execution

Children do not own durable state, direct shell execution, or privileged
native I/O. Mother owns:

- persistent state
- checkpoints
- event offsets
- task queue and leases
- graph / belief mutation auditing
- execution of typed intents

### 2. Children are WASM-first for knowledge work

Knowledge children should run as WASM components and interact through
typed host imports. Heavy local engines remain native host services.

### 3. Reads via host, writes via intents

Children read through capability-gated host APIs and request writes or
work through typed intents. No raw shell recipes, no ambient file access,
no direct process spawn from plugins.

### 4. Wrong design should not be preserved

Patina is pre-1.0 and has no user compatibility burden. Transitional
child shapes should be removed or isolated when they conflict with the
target architecture. Compatibility is only acceptable as an
implementation aid, not a design goal.

### 5. Preserve child locality

Children should still feel like small apps. That means:

- child owns workflow
- child owns state machine
- child owns retries and partial-failure policy
- child owns escalation decisions
- Mother provides primitives and safety, not workflow orchestration

Resource ownership moves to Mother. Behavioral ownership stays in the
child.

This is a hard rule for implementation:

- Mother may enforce, persist, schedule, and execute
- Mother may not decide child business workflow

## Runtime Shape

```
WASM Knowledge Child
    │
    │ typed host imports
    ▼
Mother Host Runtime
    ├─ state/checkpoint store
    ├─ lake/storage host
    ├─ event queue + offsets
    ├─ task queue + leases
    ├─ graph host
    ├─ belief host
    ├─ query host
    ├─ HTTP host
    ├─ emit host
    ▼
Native Patina services
    ├─ SQLite / graph DB
    ├─ vault
    ├─ DuckDB / DuckLake
    ├─ git / repos
    └─ connector / broker path
```

The child never escapes this boundary. If a child needs work done, it
requests an intent and Mother performs it.

## §1A — Mother / Child / Toy Model

The mental model is not decorative. It must be represented in the types.

- Mother = native authority host
- Child = plugin with agency
- Toy = typed capability bundle the child can use

Children should not program against a giant bag of host functions.
Children should program against toys.

### Toy rules

A toy must be:

- a coherent unit of authority
- a coherent unit of behavior
- impossible to use unless granted
- narrow enough to audit
- ergonomic enough to feel like a tool a child "plays with"
- coarse enough to support app-like child code

Toys are intentionally **coarse and app-like**, not fine-grained
capability primitives. Fine-grained primitives would either force
children to assemble workflows from plumbing or push orchestration back
into Mother.

Bad:

- one mega `HostContext` with dozens of unrelated methods
- shell command recipes called toys
- stringly toy invocation
- toys split into dozens of tiny capability atoms

Good:

- `FetchToy`
- `LakeToy`
- `BeliefToy`
- `GraphToy`
- `QueryToy`
- `MeasureToy`

### Child-facing shape

The SDK should make child authority explicit:

```rust
pub struct DuckLakeToys {
    pub fetch: FetchToy,
    pub lake: LakeToy,
    pub measure: MeasureToy,
}

pub struct DuckLakeChild {
    toys: DuckLakeToys,
    state: DuckLakeState,
}
```

That preserves the Mother / Child / Toy model in the code itself.

### Child / toy / Mother division of labor

- child has agency
- toy has capability
- Mother has authority

Invariants:

- child can `tick`, hold local state, and decide policy
- toy cannot `tick`, subscribe, or own durable state
- Mother grants toys and powers them, but does not co-author the
  workflow

## §2 — Worlds and Migration

### New world

Add:

- `wit/knowledge-child/knowledge-child.wit`
- `src/plugin/internal/knowledge_child.rs`

The new world is purpose-built for knowledge children and typed host
capabilities. It is the target world for child development and the
shape the SDK should teach.

### Existing world

`mother-child` may remain temporarily as a migration aid, but it is not
the target model for new knowledge children.

### World contract

The world keeps child business actions string-dispatched, but types the
platform boundary.

```wit
package patina:knowledge-child@0.1.0;

world knowledge-child {
    import patina:host/log@0.1.0;
    import patina:host/measure@0.1.0;
    import patina:host/query@0.1.0;
    import patina:host/http@0.1.0;
    import patina:host/emit@0.1.0;
    import patina:host/state@0.1.0;
    import patina:host/checkpoint@0.1.0;
    import patina:host/lake@0.1.0;
    import patina:host/events@0.1.0;
    import patina:host/task@0.1.0;
    import patina:host/graph@0.1.0;
    import patina:host/belief@0.1.0;
    import patina:host/types@0.1.0;

    use patina:host/types@0.1.0.{child-health, pending-event, task-intent};

    export init: func();
    export name: func() -> string;
    export on-load: func() -> result<_, string>;
    export on-unload: func();
    export health: func() -> child-health;
    export handle: func(action: string, payload: string) -> result<string, string>;
    export drain: func(limit: u32) -> result<list<pending-event>, string>;
    export tick: func() -> list<task-intent>;
}
```

#### Why `drain`

The current `tick()` model is not enough for subscription-driven work.
`drain(limit)` lets Mother ask the child to pull and classify its pending
events into work units without introducing callback complexity.

#### Why keep `handle`

Child-specific business actions vary. Type the platform boundary, not
every domain action.

## §3 — Typed Host APIs

### 2.1 `patina:host/state`

Opaque plugin-owned key/value state.

```wit
interface patina:host/state@0.1.0 {
    get: func(key: string) -> option<string>;
    put: func(key: string, value-json: string) -> result<_, string>;
    delete: func(key: string) -> result<_, string>;
    list-prefix: func(prefix: string) -> list<string>;
}
```

Host behavior:

- namespace is implicitly the plugin name
- values are JSON strings
- size cap: 256 KB per value
- total soft cap per plugin: 10 MB

### 2.2 `patina:host/checkpoint`

Stream checkpoint ownership.

```wit
interface patina:host/checkpoint@0.1.0 {
    load: func(stream: string) -> option<string>;
    save: func(stream: string, checkpoint-json: string) -> result<_, string>;
}
```

Rules:

- one checkpoint per `(plugin_name, stream)`
- opaque JSON payload, versioned by plugin if needed
- host does not interpret beyond size and JSON validity

### 2.3 `patina:host/lake`

Host-mediated lake and storage operations for DuckLake-style children.

```wit
interface patina:host/lake@0.1.0 {
    ensure-lake: func(name: string) -> result<string, string>;
    load-cursor: func(lake: string, source: string, data-type: string) -> option<string>;
    save-cursor: func(lake: string, source: string, data-type: string, cursor: option<string>, written: u64, status: string, last-error: option<string>) -> result<_, string>;
    ensure-table: func(lake: string, table: string) -> result<_, string>;
    append-json-batch: func(lake: string, table: string, source: string, rows-json: list<string>) -> result<u64, string>;
    query-json: func(lake: string, sql: string) -> result<string, string>;
}
```

Rules:

- the child only sees logical lake names/ids granted by Mother
- the child never receives direct filesystem authority
- DuckDB / DuckLake stays native host-side
- the child owns workflow; Mother owns storage execution

This preserves the intended experience:

- Mother buys and owns the toy
- child uses the toy directly
- Mother is not "playing with the toy together" in the workflow sense

### 2.4 `patina:host/events`

Subscription introspection and offset advancement.

```wit
record pending-event {
    stream: string,
    offset: u64,
    event-type: string,
    payload-json: string,
    occurred-at: string,
}

interface patina:host/events@0.1.0 {
    pull: func(stream: string, after-offset: option<u64>, limit: u32) -> result<list<pending-event>, string>;
    ack-through: func(stream: string, offset: u64) -> result<_, string>;
    list-streams: func() -> list<string>;
}
```

Rules:

- plugin may only call `pull` on granted streams
- `ack-through` monotonically advances offset
- events are at-least-once until acked

### 2.5 `patina:host/task`

Typed task intent submission and lease operations.

```wit
enum task-intent-kind {
    fetch-source,
    run-query,
    emit-facts,
    materialize-index,
    verify-belief,
    sync-graph,
    refresh-credential,
    native-job,
}

record task-intent {
    kind: task-intent-kind,
    payload-json: string,
    dedupe-key: option<string>,
}

interface patina:host/task@0.1.0 {
    enqueue: func(intent: task-intent) -> result<string, string>;
}
```

Rules:

- plugin may only enqueue granted intent kinds
- payload schema validated by host per kind
- `dedupe-key` suppresses duplicate queued work

### 2.6 `patina:host/graph`

Graph reads and narrow writes.

```wit
interface patina:host/graph@0.1.0 {
    query: func(kind: string, params-json: string) -> result<string, string>;
    mutate: func(action: string, payload-json: string) -> result<_, string>;
}
```

Allowed `mutate` actions in v0:

- `link`
- `unlink`
- `weight`
- `tag`

Allowed `query` kinds in v0:

- `neighbors`
- `shortest-path`
- `search`
- `project-subgraph`

The host validates requested action against manifest grants.

### 2.7 `patina:host/belief`

Belief operations with narrow write surfaces.

```wit
interface patina:host/belief@0.1.0 {
    query: func(kind: string, params-json: string) -> result<string, string>;
    mutate: func(action: string, payload-json: string) -> result<_, string>;
}
```

Allowed `mutate` actions in v0:

- `attach-evidence`
- `record-verification`
- `link-related`
- `supersede`

Allowed `query` kinds in v0:

- `by-id`
- `search`
- `pending-verification`
- `related`

### 2.8 Existing hosts reused

Reuse existing host logic where possible:

- `host_log`
- `host_measure`
- `host_query`
- `host_http`
- `host_emit`

These already exist and are gated in
`src/plugin/internal/host_support.rs`.

## §4 — Manifest Schema

### `plugin.toml`

Extend plugin manifest parsing with new sections.

```toml
[plugin]
name = "belief-verifier"
version = "0.1.0"
world = "knowledge-child"
role = "extension"
description = "Verify beliefs against evidence and record results"

[capabilities]
host_log = true
host_measure = true
host_query = ["scry", "assay"]
host_http = ["api.github.com"]
host_emit = true

[toys]
fetch = true
lake = ["default"]
measure = true

[capabilities.state]
enabled = true

[capabilities.checkpoint]
streams = ["belief.changed"]

[capabilities.events]
subscribe = ["belief.changed", "session.completed"]

[capabilities.tasks]
intents = ["verify-belief"]

[capabilities.graph]
read = true
write = []

[capabilities.belief]
read = true
write = ["record-verification", "attach-evidence"]

[schemas.beliefs]
package = "patina:schema/beliefs@1.0.0"
```

### Load-time validation

`check_capabilities()` must enforce:

- `world = "knowledge-child"` for knowledge sections
- `events.subscribe` entries are known stream names
- `tasks.intents` values are known task intent kinds
- `graph.write` values are in allowed graph mutation vocabulary
- `belief.write` values are in allowed belief mutation vocabulary
- `checkpoint.streams` are independent plugin-owned durable cursors; they
  are not required to be event subscriptions
- `host_emit` still requires at least one schema

### Runtime grant model

Extend `GrantedCapabilities` with:

```rust
pub struct GrantedCapabilities {
    pub query_kinds: HashSet<String>,
    pub query_scope: QueryScope,
    pub http_domains: HashSet<String>,
    pub credential_mappings: HashMap<String, CredentialMapping>,
    pub host_emit: bool,
    pub schema_facts: HashMap<String, HashMap<String, String>>,

    pub state_enabled: bool,
    pub checkpoint_streams: HashSet<String>,
    pub lake_names: HashSet<String>,
    pub subscribed_streams: HashSet<String>,
    pub task_intents: HashSet<TaskIntentKind>,
    pub graph_read: bool,
    pub graph_write_actions: HashSet<String>,
    pub belief_read: bool,
    pub belief_write_actions: HashSet<String>,
}
```

Add toy grants resolved from manifest:

```rust
pub struct GrantedToys {
    pub fetch: bool,
    pub lake_names: HashSet<String>,
    pub belief: bool,
    pub graph: bool,
    pub query: bool,
    pub measure: bool,
}
```

Toys are the child-facing authority model. Low-level capabilities remain
host-internal enforcement detail.

## §5 — Mother-Owned Persistence

### Tables

Add to Mother DB:

```sql
CREATE TABLE IF NOT EXISTS mother_child_state (
    plugin_name TEXT NOT NULL,
    key TEXT NOT NULL,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (plugin_name, key)
);

CREATE TABLE IF NOT EXISTS mother_child_checkpoints (
    plugin_name TEXT NOT NULL,
    stream TEXT NOT NULL,
    checkpoint_json TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (plugin_name, stream)
);

CREATE TABLE IF NOT EXISTS mother_child_subscriptions (
    plugin_name TEXT NOT NULL,
    stream TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (plugin_name, stream)
);

CREATE TABLE IF NOT EXISTS mother_child_offsets (
    plugin_name TEXT NOT NULL,
    stream TEXT NOT NULL,
    acked_offset INTEGER NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (plugin_name, stream)
);

CREATE TABLE IF NOT EXISTS mother_child_tasks (
    id TEXT PRIMARY KEY,
    plugin_name TEXT NOT NULL,
    intent_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    dedupe_key TEXT,
    status TEXT NOT NULL,
    lease_owner TEXT,
    lease_until TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_mother_child_tasks_dedupe
ON mother_child_tasks (plugin_name, dedupe_key)
WHERE dedupe_key IS NOT NULL;

CREATE TABLE IF NOT EXISTS mother_child_runs (
    id INTEGER PRIMARY KEY,
    plugin_name TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    status TEXT NOT NULL,
    metrics_json TEXT,
    error TEXT
);

CREATE TABLE IF NOT EXISTS graph_mutation_log (
    seq INTEGER PRIMARY KEY,
    plugin_name TEXT NOT NULL,
    action TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS belief_mutation_log (
    seq INTEGER PRIMARY KEY,
    plugin_name TEXT NOT NULL,
    action TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);
```

### Why SQLite, not files

- transactional ack + enqueue flows
- resumable runs and leases
- audit logs in one place
- no plugin filesystem coupling

DuckLake-style children use host lake APIs backed by native Mother
services, not direct files from the child.

## §6 — Event Model

### Stream sources in v0

Expose a narrow set of Mother-managed streams:

- `belief.changed`
- `graph.changed`
- `fact.ingested`
- `session.completed`
- `repo.synced`

### Event envelope

Internal Rust shape:

```rust
pub struct PendingEvent {
    pub stream: String,
    pub offset: u64,
    pub event_type: String,
    pub payload_json: String,
    pub occurred_at: String,
}
```

### Ordering

Guarantees:

- offsets are strictly increasing per stream
- no cross-stream total order guarantee
- delivery is at-least-once

### Replay / idempotency

Plugins must be able to replay from prior offsets. Mother provides:

- pull after offset
- ack-through offset

Recommended plugin pattern:

- process batch
- enqueue deduped task intents
- ack only after durable plugin-side host state is updated

## §7 — Typed Intents

### Intent vocabulary

Closed enum in Rust:

```rust
pub enum TaskIntentKind {
    FetchSource,
    RunQuery,
    EmitFacts,
    MaterializeIndex,
    VerifyBelief,
    SyncGraph,
    RefreshCredential,
    NativeJob,
}
```

### Payload schemas

Payloads remain JSON for v0, but schema is host-validated per kind.

Toy operations should be synchronous and bounded by default. If work is
too large, slow, or retry-heavy, the child should enqueue a typed intent
through the relevant toy/host surface rather than blocking forever in a
single call.

Examples:

#### `verify-belief`

```json
{
  "belief_id": "children-have-agency-toys-are-capabilities",
  "evidence_ids": ["commit-abc", "session-xyz"],
  "mode": "incremental"
}
```

#### `sync-graph`

```json
{
  "scope": "current_project",
  "reason": "new_fact_batch",
  "source_offsets": {"fact.ingested": 182}
}
```

#### `native-job`

Restricted host escape hatch for host-owned jobs only:

```json
{
  "job_kind": "measure-index-health",
  "params": {"project_uid": "abc123"}
}
```

`native-job` is not arbitrary command execution. `job_kind` must be from
an allowlisted host registry.

## §8 — Daemon Scheduling Model

### Heartbeat loop

Mother heartbeat for each loaded knowledge child:

1. record run start
2. call `drain(limit)`
3. host enqueues returned task intents
4. call `tick()`
5. host enqueues returned periodic task intents
6. execute eligible queued tasks for that plugin
7. record run finish

### Task execution

Lease rules:

- one lease holder per task
- `lease_until` = now + 60 seconds
- stale leased tasks are reclaimable

Status transitions:

- `queued`
- `leased`
- `running`
- `succeeded`
- `failed`
- `dead_letter`

Retry policy:

- max 5 attempts
- exponential backoff: 1m, 5m, 15m, 1h, 6h

### Concurrency

v0 policy:

- per-plugin serial task execution
- different plugins may run concurrently

This avoids plugin state races in the first implementation.

## §9 — Migration Strategy

### Loader

Extend plugin world enum with `KnowledgeChild`.

Daemon load path:

- if `world = knowledge-child`, use new `KnowledgeChildEngine`
- if `world = mother-child`, keep old loader only as a migration bridge

### Shared host logic

`src/plugin/internal/host_support.rs` remains the home for shared logic.

Add modules or helper functions for:

- state
- checkpoint
- events
- tasks
- graph
- belief

### Do not preserve wrong examples

Do not let old child shapes become permanent examples in docs or SDK.
If a current child embodies the wrong model, replace it with the final
shape rather than wrapping it indefinitely.

## §10 — Proof Children

### `ducklake`

DuckLake becomes the canonical example child for the SDK.

Manifest grants:

- lake names: `["default"]` or granted logical lake id
- tasks: `fetch-source`, `emit-facts`
- query: optional read-side introspection
- events: optional source refresh triggers

Flow:

1. child loads granted lake id and source config through host state
2. child loads per-type cursors through host lake API
3. child requests fetch work through typed intents or host query/fetch APIs
4. child ensures target tables through host lake API
5. child appends rows through host lake API
6. child persists cursor and status through host lake API
7. child records measures and escalates infrastructure failures

The important architectural point is that DuckLake no longer spawns a
connector binary or owns direct storage authority. It demonstrates the
final Mother/child/toy split the SDK should teach.

From the child author's perspective, DuckLake still feels like a small
app using toys directly:

```rust
let batch = self.toys.fetch.fetch_batch(request)?;
let written = self.toys.lake.write_batch(table, batch)?;
self.toys.lake.save_cursor(cursor)?;
```

The host mediation is an implementation detail behind the toy boundary,
not a different mental model.

#### DuckLake invariants to preserve

The redesign must preserve these semantics from the current hard-won
DuckLake implementation:

- typed capability grant on init
- connector toy as indivisible authority
- child-owned fetch→store workflow
- per-type cursors
- partial success across data types
- auth escalation to Mother
- process-owned measurement semantics

If the redesign loses these, it has failed even if the plugin boundary
looks cleaner.

### `belief-verifier`

Manifest grants:

- events: `belief.changed`
- checkpoint: `belief.changed`
- tasks: `verify-belief`
- belief write: `record-verification`, `attach-evidence`
- query: `assay`

Flow:

1. pull belief events after acked offset
2. classify work into `verify-belief` intents
3. enqueue deduped intents
4. on host task execution, run verification logic
5. write verification result through host belief API
6. ack offset after durable completion

## §11 — SDK Shape

The SDK must teach "children use toys."

Recommended crate layout:

- `patina-child-sdk`
  guest-side child traits and lifecycle glue
- `patina-toy-sdk`
  guest-side toy wrappers (`FetchToy`, `LakeToy`, `BeliefToy`, etc.)
- `patina-host-toys`
  host-side toy backend traits and registrations

Guest-side authoring should look like:

```rust
pub trait KnowledgeChild {
    type Toys;

    fn init(toys: Self::Toys) -> Result<Self, String>
    where
        Self: Sized;

    fn tick(&mut self) -> Vec<TaskIntent>;
}
```

The SDK should not expose:

- raw shell command toys
- direct process spawn
- giant host bags of unrelated methods

The SDK should encourage this mental model:

- Mother grants toys
- child uses toys
- toys may be backed by native or WASM host implementations
- child code should not care which backend powers the toy

Native and WASM toy backends should share one host-side abstraction so
children do not care which implementation backs a granted toy.

## §12 — File Plan

### New

- `wit/knowledge-child/knowledge-child.wit`
- `src/plugin/internal/knowledge_child.rs`
- `crates/patina-child-sdk/`
- `crates/patina-toy-sdk/`
- `src/mother/toys.rs`
- `src/mother/state.rs`
- `src/mother/checkpoint.rs`
- `src/mother/lake_host.rs`
- `src/mother/events.rs`
- `src/mother/tasks.rs`
- `src/mother/graph_host.rs`
- `src/mother/belief_host.rs`
- `plugins/ducklake/`
- `plugins/belief-verifier/`

### Modified

- `src/plugin/internal/mod.rs`
- `src/plugin/internal/host_support.rs`
- `src/commands/mother/daemon.rs`
- `src/mother/mod.rs`
- `src/mother/child.rs`

## §13 — Commit Plan

1. `feat(plugin): add knowledge-child world and engine`
2. `feat(plugin): parse and validate knowledge-child capabilities`
3. `feat(mother): add child state and checkpoint storage`
4. `feat(mother): add event subscriptions and offsets`
5. `feat(mother): add typed task queue and lease executor`
6. `feat(mother): add graph and belief host APIs with audit logs`
7. `feat(mother): execute typed task intents instead of shell toys`
8. `feat(mother): add lake host API for DuckLake-style children`
9. `feat(plugin): add ducklake knowledge child`
10. `feat(plugin): add belief-verifier knowledge child`
11. `feat(sdk): add child and toy SDK crates`
12. `test(plugin): add capability, recovery, and replay coverage`

## Open Questions Resolved

### Should task payloads be fully typed in WIT?

No for v0. The enum of allowed intent kinds is typed; payloads stay JSON
to keep iteration cheap. Host-side schema validation preserves safety.

### Should children mutate Mother state directly?

No. Children use host APIs only.

### Should we port DuckLake now?

Yes, but not by compiling DuckDB / DuckLake itself to WASM. The DuckLake
child should move to the new WASM child model while the storage engine
remains a native host capability.

### Should shell-command toys survive?

No. They should be replaced for knowledge children. Native host jobs may
still execute commands internally, but that is Mother-owned behavior, not
plugin-requested shell strings.

## Build Readiness

This design is intended to be implementation-complete enough to build
autonomously:

- exact world boundary defined
- host interfaces defined
- manifest schema defined
- DB schema defined
- event semantics defined
- task semantics defined
- DuckLake path defined
- proof children defined

Any additional choices during implementation should be local code-shape
choices, not architecture decisions.
