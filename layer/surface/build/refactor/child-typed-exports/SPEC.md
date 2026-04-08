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

  # Phase A: foundation (types + world + SDK + host)
  - id: cte1-record-wit
    text: "A shared WIT interface package `patina:record@0.1.0` defines typed records: `record-envelope` (record-id, source-path, source-hash, source-modified-at, source-size-bytes, content, content-hash, content-type, encoding, line-count, ingested-at, batch-id, schema-version), `file-found`, `file-written`, `rejected-record`, and `process-result` (accepted + rejected lists). WIT files placed in `wit/record/` with deps mirrored to `wit/child/deps/` and `sdk/patina-sdk/wit/child/deps/` per existing convention."
    checked: false

  - id: cte2-separate-world
    text: "A new `child-record-processor` world defined in `wit/child/` that includes all of `child` plus exports `patina:record/record-processor`. Children that process records target this world. Children that don't target the existing `child` world. Two worlds, not optional exports."
    checked: false

  - id: cte3-sdk-trait
    text: "SDK provides a `RecordProcessor` trait in `sdk/patina-sdk/src/record.rs` alongside the existing `Child` trait. A separate `register_record_processor!` macro generates both the Child lifecycle exports and the record-processor typed export. The existing `register_child!` macro is unchanged."
    checked: false

  - id: cte4-host-dispatch
    text: "Host linker in `src/child/internal/child.rs` supports both worlds: `child` (existing) and `child-record-processor` (new). World selection is declared in `child.toml` via a `world` field. When a child targets `child-record-processor`, Mother calls `process()` directly with typed records from the event stream — Mother owns subscribe/ack/emit for these children."
    checked: false

  # Phase B: migrate two canon children as proof
  - id: cte5a-schema-enforcer
    text: "schema-enforcer targets `child-record-processor` world, implements `RecordProcessor`, uses `register_record_processor!`. No hardcoded stream names in source. `child.toml` declares `world = \"child-record-processor\"`. `cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2` succeeds."
    checked: false

  - id: cte5b-dedup-filter
    text: "dedup-filter targets `child-record-processor` world, implements `RecordProcessor`, uses `register_record_processor!`. No hardcoded stream names in source. `child.toml` declares `world = \"child-record-processor\"`. `cargo build -p patina-ai-child-dedup-filter --target wasm32-wasip2` succeeds."
    checked: false

  # Phase C: pando wiring + backward compat
  - id: cte6-pando-type-validation
    text: "Pando wiring is parsed into structured form (not just `Vec<String>`). At pando load time, Mother validates: source child exports `record-processor`, target child can receive those records. Type mismatches are rejected at load time with a clear error."
    checked: false

  - id: cte7-backward-compat
    text: "All service children (belief-verifier, session-writer, spec-manager, doctor) continue targeting the `child` world with `handle` and no changes. `cargo check --workspace -q`, `cargo test -q --lib`, and `cargo test --test wasm_integration` pass."
    checked: false

  # Phase D: remaining canon children (future, not gated)
  # Remaining 4 canon children (file-system-monitor, content-extractor,
  # record-writer, lakehouse-catalog) migrate in a follow-up spec after
  # Phase A-C proves the pattern.

---
# refactor: Typed WIT exports for canon children

> Replace handle(string, string) data path with typed WIT interfaces on
> canon children. Keep handle for control-plane. Align child exports
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

## Key Design Decisions

### Two worlds, not optional exports

WIT worlds require all exports to be satisfied. The component model does
not support optional exports. Rather than fight this, we follow the
standard: define two worlds.

- `child` — existing world. Lifecycle + `handle`. Service children use this.
- `child-record-processor` — includes `child` plus exports `record-processor`.
  Canon pipeline children that process records use this.

Children declare their world in `child.toml` via `world = "child"` (default)
or `world = "child-record-processor"`. This is explicit, not auto-detected.

### Data-plane ownership shift

This is the biggest runtime change. Today, children own the full data flow:

```
Child subscribes -> Child processes -> Child emits -> Child acks
```

After migration, Mother owns the data flow for record-processor children:

```
Mother subscribes -> Mother calls child.process(records) -> Mother emits -> Mother acks
```

The child becomes a pure transform: records in, results out. Mother handles
stream wiring, cursor management, ack semantics, and backpressure. This is
Wagner's "virtual platform layering" — the platform (Mother) owns IO, the
component (child) owns compute.

Children targeting the `child` world (service children) continue to manage
their own stream IO via the events-stream and messaging toys.

### Separate registration macro

The `register_child!` macro stays unchanged — it generates exports for
the `child` world. A new `register_record_processor!` macro generates
exports for the `child-record-processor` world (lifecycle + typed export).

Rust's macro system cannot detect trait implementations at expansion time.
Auto-detection was proposed but is not feasible. Explicit macro selection
is clearer and follows the pattern of explicit world targeting.

### Output contract: process-result with accepted + rejected

```wit
record process-result {
    accepted: list<record-envelope>,
    rejected: list<rejected-record>,
}

process: func(records: list<record-envelope>) -> result<process-result, string>;
```

The `result` error case is for infrastructure failures (child trapped, etc).
Business-level rejections (schema violations, duplicates) are returned in
`rejected` — they are normal output, not errors.

### child.toml changes required

The spec originally said "no child.toml changes." This was wrong. Migrated
children need:

```toml
[child]
world = "child-record-processor"
```

This replaces `[needs.scopes.events].subscribe` for record-processor
children — they no longer subscribe to streams directly. Mother reads the
world declaration and handles stream IO on their behalf.

### Structured pando wiring

Pando wiring is currently `Vec<String>`. For type validation, wiring must
be parsed into structured form with source child, event type, and target
child as distinct fields. This enables Mother to check interface
compatibility at pando load time.

## Non-Goals

- Service children (belief-verifier, session-writer, spec-manager, doctor)
  keep using `handle` — they are control-plane, not data processors.
- No replacement of Mother's event broker — it manages cursors, ack, backpressure.
- No removal of `handle` from `child` world — this is additive.
- Remaining 4 canon children migrate in a follow-up spec after Phase A-C
  proves the pattern.

## WASI 0.3 alignment note

WASI 0.3 adds `stream<T>` and `future<T>` as first-class WIT types.
When available, `record-processor` naturally evolves from batch:

```wit
process: func(records: list<record-envelope>) -> result<process-result, string>;
```

to streaming:

```wit
process: func(input: stream<record-envelope>) -> stream<record-envelope>;
```

Designing typed interfaces now creates the clean migration path. The types
are correct today; only the transport changes when async streams land.

## Verification

```bash
# Host compilation
cargo check --workspace -q
# Host tests
cargo test -q --lib
# WASM export generation (critical — host tests do not verify this)
cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2
cargo build -p patina-ai-child-dedup-filter --target wasm32-wasip2
# Integration tests
cargo test --test wasm_integration
# Spec check
patina spec check child-typed-exports --json
```

## Build Readiness

**Not ready.** Prerequisites are complete (shims removed, naming clean,
SDK consolidated, 728 tests passing). But three design decisions needed
investigation and are now resolved in this spec:

1. ~~Optional exports vs separate worlds~~ → **Two worlds** (resolved)
2. ~~Macro auto-detection~~ → **Separate `register_record_processor!`** (resolved)
3. ~~Data-plane ownership~~ → **Mother owns IO for record-processor children** (resolved)

Remaining before build: review this spec with audit agents, then promote
to active.
