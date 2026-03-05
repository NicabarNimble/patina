---
type: feat
id: host-emit-wit
status: draft
created: 2026-03-04
sessions:
  origin: 20260304-120702
beliefs:
- reads-via-host-writes-via-intents
- wit-is-contract-wasm-is-one-runtime
- events-are-autobiography-not-telemetry
exit_criteria:
- id: emit-interface-in-wit
  text: WIT `emit` interface exists in `patina:host/emit` with `emit-fact(event-type, schema, data) -> result<u64, string>`
  checked: true
- id: emit-wired-to-mother-child
  text: mother-child world imports `patina:host/emit` — plugins in this world can emit facts
  checked: true
- id: emit-writes-to-eventlog
  text: emitted facts appear in events.db eventlog with correct event_type, schema reference, and provenance=external
  checked: true
- id: emit-validates-schema
  text: host validates event-type matches a declared schema in the plugin manifest before writing to eventlog
  checked: true
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
- [[events-are-autobiography-not-telemetry]] — emitted facts are
  autobiography with external provenance — the project's record of what
  it discovered from outside sources. They need provenance metadata to
  distinguish from locally-originated events.

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

## Design Decisions (resolved in [[spec-plugin-infrastructure]] DESIGN.md)

- **Direct host import, not intent pattern.** Resolved via Helland
  analysis: facts are independently valid (partial writes aren't
  corruption), the host validates and writes (trust boundary preserved),
  and the plugin gets confirmation (sequence number on success). The
  intent pattern (toys) is for operations with dangerous side effects.
  Appending to an immutable log is safe. Direct call also supports
  streaming connectors that can't accumulate facts until execution ends.
  See [[spec-plugin-infrastructure]] DESIGN.md, "host_emit — The Missing
  Write Path."

- **Schema validation: manifest baseline, deepen later.** Validate that
  the schema exists, the plugin declares it in its manifest, and the
  fact-type exists in the schema. Full field-level type checking against
  schema.toml definitions is future depth. Start with structural
  validation (schema + fact-type + valid JSON).

- **Single emit, no batch.** `emit-fact()` called per event. Each fact
  stands alone (Helland: independently valid). Batch adds complexity
  without safety benefit — append-only writes don't need atomicity.
  Add `emit-batch()` later if performance demands.

## Non-Goals

- **Implementing the schema interface.** `patina:host/schema` is defined
  in host.wit but not wired. This spec adds emit, not schema queries.
  Schema queries may be a separate spec if plugins need to discover
  available schemas at runtime.
- **Emit from pipeline or command worlds.** Those remain read-only.
  Pipelines are pure compute; commands query and display.
- **Building any specific connector.** This spec builds the infrastructure.
  Forge connector is [[forge-plugin-extraction]].
