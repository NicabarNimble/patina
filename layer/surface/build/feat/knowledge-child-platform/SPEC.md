---
type: feat
id: knowledge-child-platform
status: active
created: 2026-03-11
related:
- mother-maturation
- ducklake
beliefs:
- children-have-agency-toys-are-capabilities
- separate-worlds-for-isolation
- reads-via-host-writes-via-intents
- protocol-boundaries-must-be-typed
- host-proxied-io-is-the-security-model
- specs-describe-current-code-not-aspirations
exit_criteria:
- id: knowledge-child-world-exists
  text: "A new WASM-first knowledge child world exists with typed host imports for state, checkpoints, lake/storage, graph, belief, events, tasks, query, HTTP, emit, log, and measure"
  checked: true
- id: typed-toys-replace-shell-recipes
  text: "Mother executes typed toy intents instead of child-requested shell command strings"
  checked: true
- id: mother-owns-runtime-state
  text: "Mother persists child state, checkpoints, subscriptions, task leases, and run history in SQLite with no plugin filesystem dependency"
  checked: true
- id: capability-manifests-enforced
  text: "Plugin manifests declare knowledge capabilities and Mother enforces them at load time and call time"
  checked: true
- id: event-subscriptions-work
  text: "Knowledge children can subscribe to Mother-owned event streams, resume from offsets, and process work idempotently"
  checked: true
- id: graph-and-belief-host-apis-work
  text: "WASM knowledge children can perform capability-gated graph and belief reads/mutations through typed host APIs with audit logging"
  checked: true
- id: toy-model-explicit-in-sdk
  text: "The SDK exposes children using typed toy bundles rather than generic host call bags, and DuckLake serves as the canonical child-with-toys example"
  checked: true
- id: two-real-children-ship
  text: "DuckLake ships as the canonical WASM knowledge child example and at least one additional real knowledge child ships end-to-end against the new platform"
  checked: true
- id: old-child-model-removed
  text: "The old knowledge-child model is removed or explicitly isolated; no compatibility goal preserves shell-toy or child-spawn patterns as part of the target design"
  checked: true
---
# feat: Knowledge Child Platform — WASM-First Children for Patina's Knowledge System

> Build Mother into a real host platform for knowledge children.
> Children become WASM plugins with typed, capability-gated access to
> state, checkpoints, lake/storage, graph mutation, beliefs,
> subscriptions, tasks, query, HTTP, emit, and measurement. Mother
> stays native. Heavy local engines stay native. Toys become typed
> intents executed by Mother, not shell command recipes requested by
> children.

## Problem

Patina already has a meaningful WASM substrate:

- a `mother-child` WIT world
- manifest parsing and capability gating
- host-gated HTTP/query/emit/measure support
- daemon loading of WASM children

But the current child platform is still too thin for the knowledge
system:

- child request handling is stringly (`action`, `payload`)
- toys are shell command recipes, not typed intents
- durable child state is not Mother-owned
- event subscriptions are not first-class
- graph and belief operations are not exposed as host capabilities
- real data-path work still falls back to native children and ad hoc
  runtime behavior

The result is that Patina has a plugin mechanism, but not yet a true
knowledge-child platform.

## Goal

Ship a one-shot platform upgrade that makes WASM children first-class
for knowledge work without replatforming Mother, DuckDB, DuckLake,
vault, git, or the native broker.

**Target shape:**

- Mother remains the native authority host
- knowledge children become WASM plugins
- heavy local systems remain native host services
- all child state and execution control flows through Mother
- toys become typed host-mediated intents
- DuckLake becomes the canonical child example for the SDK
- the SDK teaches children-with-toys, not generic RPC host access

## Status

Implementation is active and functionally complete in-tree as of
2026-03-11.

Delivered:

- `knowledge-child` WIT world plus host bindings
- Mother-owned SQLite runtime state, checkpoints, offsets, tasks, runs,
  and audit logs
- typed lake, event, task, graph, and belief host APIs
- `patina-toy-sdk` and `patina-child-sdk` for third-party children
- `ducklake` and `belief-verifier` as real WASM knowledge children
- explicit isolation of legacy `mother-child` shell-toy behavior from
  the new knowledge-child runtime

Verified:

- `cargo check -p patina-ai`
- `cargo test -p patina-ai knowledge_child -- --nocapture`
- `cargo test -p patina-ai state_checkpoints_and_offsets_are_namespaced_and_persistent -- --nocapture`
- `cargo test -p patina-ai task_dedupe_and_leasing_work -- --nocapture`
- `cargo test -p patina-child-sdk -p patina-toy-sdk`
- `cargo build --target wasm32-wasip2 -p patina-plugin-ducklake -p patina-plugin-belief-verifier`

## Non-Goals

This build does NOT include:

- replacing the native broker / `patina-pipe` path
- removing existing native children
- designing a distributed scheduler

This is a platform build, not a total runtime replacement.

This build does **not** require compiling DuckDB / DuckLake itself to
WASM. It does require making DuckLake's child logic conform to the new
WASM child model through host-mediated storage capabilities.

## Solution

### 1. Add a Knowledge Child World

Create a new WIT world dedicated to knowledge children.

New imports:

- `patina:host/log`
- `patina:host/measure`
- `patina:host/query`
- `patina:host/http`
- `patina:host/emit`
- `patina:host/state`
- `patina:host/checkpoint`
- `patina:host/lake`
- `patina:host/events`
- `patina:host/task`
- `patina:host/graph`
- `patina:host/belief`

This world is the typed authority surface for knowledge children and the
default target for the system going forward. It exists because knowledge
children need richer, narrower, more auditable primitives than generic
string dispatch alone.

### 2. Replace Shell Toys with Typed Intents

Current `Toy` shape is too low-level:

```rust
pub struct Toy {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}
```

Replace that model with typed intents executed by Mother, for example:

- `fetch-source`
- `run-query`
- `emit-facts`
- `materialize-index`
- `verify-belief`
- `sync-graph`
- `refresh-credential`
- `native-job`

Toys remain **coarse and app-like**, not fine-grained capability
primitives. The goal is to preserve the child mental model of "I use my
toys" rather than forcing children to assemble workflows from tiny host
operations.

Children request *what* should happen. Mother decides *how* to execute
it, validates capability scope, records audit state, manages retries,
and owns leases.

This matches [[reads-via-host-writes-via-intents]] and removes shell
strings from the capability model.

Toy invariants:

- toys have no lifecycle of their own
- toys have no independent identity in the system
- toys own no durable state
- toys do not subscribe to events
- toys do not escalate or make workflow decisions
- toys expose bounded typed methods only

This prevents toys from collapsing back into children.

### 3. Make Mother Own Child State

Add Mother-owned persistence for:

- child key/value state
- checkpoints
- subscription offsets
- task queue and leases
- run history
- mutation audit logs

Back this with SQLite.

Children must not depend on filesystem writes. A knowledge child should
be restartable, portable, and resumable from Mother-owned state alone.

Child locality rule:

- Mother owns resource authority and execution safety
- child owns workflow, retries, partial-success policy, cursor policy,
  and escalation decisions

Mother must not become the orchestrator of child internals.

### 4. Add Event Subscriptions

Knowledge children need to react to system change, not only poll.

Support subscriptions to streams such as:

- new facts
- graph mutations
- belief changes
- session completion
- repo sync completion

Implementation starts with a queued pull model:

- child declares subscriptions in manifest
- Mother records offsets
- child receives batched pending events during heartbeat / task cycle

No callback complexity in v1.

### 5. Add Lake / Storage Host APIs

If DuckLake is the canonical child example, the platform needs a
first-class storage capability. Add host APIs for:

- lake open / ensure
- cursor read / write
- table ensure
- batch write
- query / inspect
- storage-scoped status and error reporting

DuckLake should use these host capabilities rather than spawning
connector binaries or assuming direct local storage ownership from child
code.

### 6. Add Graph and Belief Host APIs

Knowledge children need first-class operations for Patina's core
domain, not just HTTP and emit.

Add host APIs for:

- graph query
- graph mutation
- belief create / update
- evidence attach
- verification record
- relationship query / linking

Mutations must be capability-gated and audit logged.

### 7. Extend Capability Manifests

Add new manifest sections for:

- toys
- state
- checkpoint
- graph
- belief
- events
- tasks

Example:

```toml
[capabilities]
host_query = ["scry", "context"]
host_http = ["api.github.com"]
host_emit = true

[capabilities.graph]
read = true
write = ["link", "weight"]

[capabilities.belief]
read = true
write = ["verification", "evidence"]

[capabilities.events]
subscribe = ["belief.changed", "forge.pr", "session.completed"]

[capabilities.tasks]
intents = ["verify-belief", "sync-graph"]
```

Manifest parsing resolves into `GrantedCapabilities`. Mother enforces
these at load time and call time.

The manifest and SDK must preserve the Mother / Child / Toy mental
model explicitly. A child declares toys, Mother grants toys, and the SDK
materializes those toys as typed guest-side bundles.

### 8. Add a Mother Task Runtime

Mother needs a native execution layer for typed intents:

- persistent task queue
- lease acquisition / release
- retry policy
- rate budgets
- intent executor

This runtime executes typed intents on behalf of children and is the
native substrate behind "toys."

### 9. Ship a Toy-Centric SDK Surface

The SDK must expose:

- typed toy bundles per child
- ergonomic guest-side toy wrappers
- no giant unstructured host context
- no raw shell-command or process-spawn APIs

The default child authoring experience should be:

- declare child
- declare toys needed
- use toys in local workflow code
- compile to WASM plugin

The SDK should optimize for code that reads like a small app using toys,
not a guest manually coordinating RPC calls.

### 10. Prove the Platform with DuckLake and One Additional Real Child

Ship DuckLake plus one additional real WASM knowledge child:

#### `ducklake`

- canonical child example for the SDK
- demonstrates typed lake/storage, fetch, state, checkpoint, task, and
  measurement capabilities through toy-centric child code
- uses toys directly from child code
- does not spawn connector binaries directly from child code
- does not request shell-command toys
- uses the final host-mediated child/toy model rather than the current
  transitional native-child pattern

#### `belief-verifier`

- subscribes to belief and evidence changes
- schedules verification work
- writes verification results through host belief API
- persists checkpoints and work state through Mother

These prove the platform against actual knowledge-system needs while
giving the SDK one canonical heavy local example (`ducklake`) and one
pure knowledge-flow child (`belief-verifier`).

### 11. Replace the Old Child Model

There is no pre-1.0 product reason to preserve a known-wrong design.
The old child model should be removed or isolated if and only if that
reduces migration risk during implementation. It is not a target
deliverable.

Knowledge children should converge on:

- WASM child
- host-owned state
- typed toy intents
- no direct child process spawning
- no shell-command recipes

Where existing child code violates that model, rewrite it rather than
enshrining it as compatibility.

## Data Model

Add Mother tables for:

- `mother_child_state(plugin_name, key, value_json, updated_at)`
- `mother_child_checkpoints(plugin_name, stream, checkpoint_json, updated_at)`
- `mother_child_subscriptions(plugin_name, subscription_type, filter_json)`
- `mother_child_offsets(plugin_name, subscription_key, offset)`
- `mother_child_tasks(id, plugin_name, intent_type, payload_json, status, lease_owner, lease_until, attempts, last_error, created_at, updated_at)`
- `mother_child_runs(plugin_name, started_at, finished_at, status, metrics_json, error)`
- `graph_mutation_log(seq, plugin_name, action, payload_json, created_at)`
- `belief_mutation_log(seq, plugin_name, action, payload_json, created_at)`

All writes are host-owned. Plugins never write these directly.

## File Plan

New files / modules:

- `wit/knowledge-child/knowledge-child.wit`
- `src/plugin/internal/knowledge_child.rs`
- `src/mother/state.rs`
- `src/mother/checkpoint.rs`
- `src/mother/lake_host.rs`
- `src/mother/events.rs`
- `src/mother/tasks.rs`
- `src/mother/graph_host.rs`
- `src/mother/belief_host.rs`
- `plugins/ducklake/`
- `plugins/belief-verifier/`

Major edits:

- `src/mother/child.rs`
- `src/plugin/internal/mod.rs`
- `src/plugin/internal/host_support.rs`
- `src/commands/mother/daemon.rs`

## Implementation Order

### Phase 1: Contracts

1. Add `knowledge-child` WIT world
2. Add bindgen / engine integration
3. Extend plugin manifest parser for knowledge capabilities
4. Extend granted capability model

### Phase 2: Mother Runtime

5. Add SQLite tables for child state, checkpoints, subscriptions, tasks, runs
6. Add state and checkpoint host APIs
7. Add lake / storage host API
8. Add event subscription and offset machinery
9. Add task queue and lease executor

### Phase 3: Knowledge APIs

10. Add graph host API
11. Add belief host API
12. Add mutation audit logging
13. Replace shell-toy model with typed intents in Mother runtime

### Phase 4: Proof Children

14. Build `ducklake` as the canonical child example
15. Build `belief-verifier`
16. Integrate both into daemon load path
17. Verify end-to-end runs against Mother state and audit logs

### Phase 5: Migration and Tests

18. Remove or isolate transitional child patterns
19. Add toy-centric SDK guest APIs and example crates
20. Add capability rejection tests
21. Add recovery / trap / lease tests
22. Add event replay / checkpoint resume tests

## Test Plan

### Capability Enforcement

- plugin denied if manifest requests unsupported knowledge capability
- plugin denied graph mutation outside granted action set
- plugin denied belief mutation outside granted action set
- plugin denied unsubscribed event stream access
- plugin denied HTTP domain outside allowlist
- plugin denied query kind outside grant

### State Isolation

- plugin A cannot read plugin B state
- plugin checkpoints survive restart
- offsets resume correctly after restart

### Runtime Recovery

- task lease expires and is retried safely
- trapped plugin does not poison Mother
- event replay after failure is idempotent
- run history records failure and recovery

### End-to-End

- `ducklake` runs end-to-end through the new host-mediated child/toy model
- `belief-verifier` processes belief updates and records verification
- SDK example code reads naturally as child-with-toys, not host-RPC plumbing

## Notes

- Checkpoints are plugin-owned durable cursors/state markers and are not
  required to correspond to subscribed event streams.
- Legacy `mother-child` plus shell-toy execution remains only as an
  isolated migration bridge. New knowledge children target
  `knowledge-child`.

## Risks

### Risk: Too Much String Dispatch Survives

Mitigation:

- type the new host imports
- keep string dispatch only for child-specific business actions
- type all cross-boundary runtime capabilities

### Risk: Mother Becomes a Blob

Mitigation:

- keep host modules narrow (`state`, `events`, `tasks`, `graph`, `belief`)
- enforce capability boundaries in one place
- avoid mixing broker concerns into knowledge-child host code

### Risk: Typed Intents Become Another Shell Escape Hatch

Mitigation:

- intent enum is closed
- native execution is host-internal only
- plugins do not submit arbitrary shell commands

## Deliverable

After this build, Patina will have:

- a real WASM-first child platform for knowledge work
- native Mother as authority host
- typed host APIs for core knowledge operations
- Mother-owned child state and subscriptions
- typed toys executed as host intents
- DuckLake as the canonical child example for the SDK
- one additional real knowledge child proving the model

This is the implementation plan to follow for building richer children
plugins for the Patina knowledge system.
