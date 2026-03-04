---
type: feat
id: host-emit-wit
status: draft
created: 2026-03-04
blocked_by:
- plugin-infrastructure
sessions:
  origin: 20260304-120702
beliefs:
- reads-via-host-writes-via-intents
- wit-is-contract-wasm-is-one-runtime
- events-are-autobiography-not-telemetry
exit_criteria:
- id: emit-interface-in-wit
  text: WIT `emit` interface exists in `patina:host/emit` with `emit-fact(event-type, schema, data) -> result<u64, string>`
  checked: false
- id: emit-wired-to-mother-child
  text: mother-child world imports `patina:host/emit` — plugins in this world can emit facts
  checked: false
- id: emit-writes-to-eventlog
  text: emitted facts appear in events.db eventlog with correct event_type, schema reference, and provenance=external
  checked: false
- id: emit-validates-schema
  text: host validates emitted data against the plugin's declared schema before writing to eventlog
  checked: false
---
# feat: Host Emit WIT Interface — Plugins Can Write Facts

> Add emit interface to WIT host so plugins can write facts to the
> eventlog. Foundation for all plugin data ingestion.

## Problem

Plugins can read from Patina (scry, assay, context via `patina:host/query`)
but cannot write facts back. This means:
- Forge data must be written by core code, not the forge plugin
- No connector plugin can ingest external data
- The plugin system is read-only, which blocks all extraction work

**Code references:**
- `wit/deps/patina-host/host.wit` — defines `schema` interface (lines
  123-134) but it's NOT imported by any world and NOT implemented
- `src/plugin/internal/host_support.rs` — has `query()` and `http_post()`
  but no `emit()`
- `src/plugin/internal/mother_child.rs` — WasmChild bindgen doesn't
  include any emit host trait

**Belief grounding:**
- [[reads-via-host-writes-via-intents]] — "plugin writes are returned as
  intents (host validates, audits, executes)." Emit is a write operation.
  Should it follow the intent pattern (return intent, host executes) or
  be a direct host call? See Exploration section.
- [[events-are-autobiography-not-telemetry]] — emitted facts are external
  evidence, not autobiography. They need provenance metadata.

## Solution

Add `patina:host/emit` interface to the WIT host package:

```wit
interface emit {
    /// Emit a fact to the project's eventlog.
    /// event-type: namespaced type (e.g., "forge.issue")
    /// schema: schema package reference (e.g., "patina:schema/forge@1.0.0")
    /// data: JSON-serialized fact payload
    /// Returns: event sequence number on success
    emit-fact: func(event-type: string, schema: string, data: string) -> result<u64, string>;
}
```

Host implementation in `host_support.rs`:
1. Validate `event-type` matches a declared schema in the plugin manifest
2. Validate `data` against the schema's WIT record types (or at minimum,
   valid JSON matching expected fields)
3. Write to events.db with `provenance=external` metadata
4. Return the seq number for the emitted event

Wire into mother-child world first (connectors need it). Task world
second (if connectors also run as tasks). Pipeline and command worlds
do NOT get emit — they remain read-only/pure-compute.

## Steps

1. Define `patina:host/emit` interface in `wit/deps/patina-host/host.wit`
2. Import `patina:host/emit` in `wit/mother-child/mother-child.wit`
3. Implement `Host` trait for emit in `src/plugin/internal/host_support.rs`
4. Wire bindgen in `src/plugin/internal/mother_child.rs`
5. Add `host_emit` to capability gating in `PluginManifest` validation
6. Add integration test: mother-child plugin emits a fact, verify it
   appears in events.db with correct provenance

## Exploration Needed

- **Intent pattern vs direct call?** [[reads-via-host-writes-via-intents]]
  says writes should be intents. But emit is append-only (no destructive
  side effects). Direct host call may be simpler and safe. The intent
  pattern makes more sense for git operations (commit, tag) where the
  host needs to audit and may reject. Emit is more like logging — always
  allowed if the schema matches. **Decision needed before build.**

- **Schema validation depth.** Full WIT record type validation at the
  host boundary? Or just valid JSON + correct event_type? Full validation
  is safer but requires the host to parse WIT at runtime. JSON validation
  is simpler but trusts the plugin more. Could start with JSON and add
  WIT validation later.

- **Batch emit.** If a connector fetches 500 issues, does it call
  emit-fact 500 times? Or should there be `emit-batch(facts: list<...>)`?
  Single emit is simpler. Batch is more efficient. Could start with
  single and add batch if performance demands.

## Non-Goals

- **Implementing the schema interface.** `patina:host/schema` is defined
  in host.wit but not wired. This spec adds emit, not schema queries.
  Schema queries may be a separate spec if plugins need to discover
  available schemas at runtime.
- **Emit from pipeline or command worlds.** Those remain read-only.
  Pipelines are pure compute; commands query and display.
- **Building any specific connector.** This spec builds the infrastructure.
  Forge connector is [[forge-plugin-extraction]].
