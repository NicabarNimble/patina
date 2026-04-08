---
type: refactor
id: child-typed-exports
status: draft
created: 2026-04-08
sessions:
  origin: 20260408-064526-677971000
beliefs:
  - "[[wasi-is-foundation-not-option]]"
  - "[[children-have-agency-toys-are-capabilities]]"
related:
  - wit/child/child.wit
  - sdk/patina-sdk/src/child.rs
  - src/child/internal/child.rs
  - children/
  - mother/src/pando.rs
exit_criteria:

  - id: cte1-record-wit
    text: "A shared WIT interface package `patina:record@0.1.0` defines `record-envelope` with typed fields matching the current Record struct (record-id, source-path, source-hash, content-hash, content-type, content, encoding, schema-version, ingested-at, batch-id). No JSON serialization at this boundary."
    checked: false

  - id: cte2-child-world-additive
    text: "The `patina:child@0.1.0` world is extended with an optional typed export interface for record processing. `handle` remains as a fallback — existing children compile without changes."
    checked: false

  - id: cte3-sdk-trait
    text: "SDK provides a `RecordProcessor` trait (or equivalent) alongside the existing `Child` trait. Children that implement it get typed WIT exports wired automatically by the `register_child!` macro."
    checked: false

  - id: cte4-host-dispatch
    text: "Host linker (`src/child/internal/child.rs`) detects typed exports at instantiation and dispatches through them when available, falling back to `handle` for children that don't export typed interfaces."
    checked: false

  - id: cte5-two-children-migrated
    text: "At least two canon children (schema-enforcer and dedup-filter) export the typed record-processing interface. They no longer hardcode event stream names — input/output streams are declared in their world imports, not in Rust source."
    checked: false

  - id: cte6-pando-type-validation
    text: "Pando wiring can validate interface compatibility: Mother checks that the output type of one child matches the input type of the next child in the wiring chain at pando load time."
    checked: false

  - id: cte7-backward-compat
    text: "All service children (belief-verifier, session-writer, spec-manager, doctor) continue to use `handle` without changes. `cargo check --workspace -q` and `cargo test -q --lib` pass."
    checked: false

---
# refactor: Typed WIT exports for canon children

> Replace handle(string, string) data path with typed WIT interfaces on
> the 6 canon children. Keep handle for control-plane. Align child exports
> with component model standards.

## Why

Patina builds modular, reusable WASM compute blocks (children) that agents
compose into pandos. We chose the Bytecode Alliance component model for
sandboxing, portability, typed interfaces, and composability. We align
with WASI for capability interfaces and aspire to BA membership.

We are aligned at every layer except child data exports. Toys are typed
WIT. The binary format is component model. The sandbox is deny-by-default.
But the child-to-child data boundary is:

```wit
export handle: func(action: string, payload: string) -> result<string, string>;
```

This collapses the component model's type system at the most important
seam — where children connect. The `Record` struct is duplicated in 4
children's Rust source, serialized to JSON, and deserialized on the other
side. WIT never sees it. Stream names are hardcoded in child source,
making reuse require reading implementation.

Closing this gap matters for:

1. **Composability** — pandos compose children by matching typed interfaces,
   not by hoping JSON shapes align at runtime.
2. **Reusability** — a child's contract is visible from WIT alone, no
   source code reading required.
3. **Standards path** — WASI 0.3 `stream<T>` is the natural evolution,
   but only if we have typed records in WIT first.

## Principle

Follow WASM/WASI/Component Model standards. Adapt only where Patina's
needs are genuinely outside:

- **Standard:** typed WIT interfaces for data contracts between children
- **Standard:** component-level import/export declarations
- **Adaptation:** Mother as runtime coordinator (dynamic composition, event brokering, cursor/ack)
- **Adaptation:** child lifecycle (on_load, tick, drain, health — no WASI equivalent for managed component lifecycle)

## Design

### Split: control plane vs data plane

| Concern | Mechanism | Standard? |
|---------|-----------|-----------|
| Lifecycle | `on_load`, `on_unload`, `health` | Patina adaptation (platform-specific) |
| Control dispatch | `handle(action, payload)` | Patina adaptation (kept for service children) |
| Periodic work | `tick() -> Vec<TaskIntent>` | Patina adaptation |
| Event drain | `drain(limit) -> Vec<PendingEvent>` | Patina adaptation |
| **Data processing** | **Typed WIT interface per domain** | **Component model standard** |

### Shared record type (WIT)

```wit
// package patina:record@0.1.0
interface record-types {
    record record-envelope {
        record-id: string,
        source-path: string,
        source-hash: string,
        source-modified-at: string,
        source-size-bytes: u64,
        content: string,
        content-hash: string,
        content-type: string,
        encoding: string,
        line-count: u64,
        ingested-at: string,
        batch-id: string,
        schema-version: u32,
    }
}
```

This replaces the `Record` struct currently duplicated in 4 children's
Rust source — eliminating JSON serialization at child boundaries.

### Typed export interface

```wit
interface record-processor {
    use record-types.{record-envelope};
    process: func(records: list<record-envelope>) -> result<list<record-envelope>, string>;
}
```

Canon pipeline children export `record-processor`. Mother calls `process`
directly with typed records instead of routing through `handle` with JSON.

### Stream declaration (imports, not hardcoded)

Today children hardcode stream names in source:

```rust
subscribe("record.extracted", after_offset, limit)?;  // hardcoded
emit("record.validated", ...)?;                        // hardcoded
```

After: stream bindings are declared in the child's world imports and
configured by the pando wiring — not embedded in child code. The child
says "I process records" and the pando says "your input is this stream,
your output goes there."

### Migration path

| Phase | What | Children affected |
|-------|------|-------------------|
| 0 | Define `patina:record@0.1.0` WIT, add optional export to child world | 0 (additive) |
| 1 | Add `RecordProcessor` SDK trait, update host dispatch | 0 (additive) |
| 2 | Migrate schema-enforcer + dedup-filter | 2 |
| 3 | Migrate remaining canon children | 4 |
| Future | WASI 0.3 `stream<record-envelope>` when available | evolve |

## Non-Goals

- Service children (belief-verifier, session-writer, spec-manager, doctor)
  keep using `handle` — they are control-plane, not data processors.
- No replacement of Mother's event broker — it manages cursors, ack, backpressure.
- No removal of `handle` from `patina:child@0.1.0` world — this is additive.
- No `child.toml` manifest structure changes.

## WASI 0.3 alignment note

WASI 0.3 adds `stream<T>` and `future<T>` as first-class WIT types.
When available, `record-processor` naturally evolves from batch:

```wit
process: func(records: list<record-envelope>) -> ...
```

to streaming:

```wit
process: func(input: stream<record-envelope>) -> stream<record-envelope>;
```

Designing typed interfaces now creates the clean migration path. The types
are correct today; only the transport changes when async streams land.

## Verification

```bash
cargo check --workspace -q
cargo test -q --lib
patina spec check child-typed-exports --json
```

## Build Readiness

Ready. All prerequisite cleanup is done: children use canonical `Child`
trait, `MotherRuntimeStore` renamed, monolith retired, SDK consolidated.
The 6 canon children and the host linker are the implementation surface.
