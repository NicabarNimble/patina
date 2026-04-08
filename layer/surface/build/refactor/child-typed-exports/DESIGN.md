# Design: Typed WIT exports for canon children

## Why We Are Doing This

Patina's mission is to provide modular, reusable WASM compute blocks that
agents compose into pandos — higher-level products. We chose WASM and the
component model because of the properties Luke Wagner and the Bytecode
Alliance have designed into it: sandboxing, portability, typed interfaces,
and composability without shared memory.

We are aligned with the Bytecode Alliance standards at every layer except
one. Our toys (capabilities) are typed WIT interfaces — five standard WASI
proposals plus Patina extensions that document the gaps they fill. Our
binary format is the W3C component model. Our sandbox is deny-by-default
with explicit grants via child.toml. All of this follows the standard.

But our children's data processing boundary does not:

```wit
export handle: func(action: string, payload: string) -> result<string, string>;
```

This single untyped export collapses everything the component model gives
us at the most important seam — where children connect to each other. The
`Record` struct that flows between schema-enforcer, dedup-filter, and
record-writer is defined identically in 4 separate Rust files as a serde
struct, serialized to JSON, passed through Mother's event broker as a
string, and deserialized on the other side. WIT never sees it. The
component model cannot verify that one child's output matches another's
input. Stream names are hardcoded in Rust source, making children
non-reusable without modification.

This is the gap between "uses WASM" and "is a component model citizen."
Closing it matters for three reasons:

1. **Composability** — pandos should compose children by matching typed
   interfaces, not by hoping JSON shapes align at runtime. The component
   model's toolchain (wasm-compose, WAC) can verify this at build time
   if we give it types to work with.

2. **Reusability** — the child-construction-canon's thesis is that children
   are reusable across pandos by configuration. Today, reuse requires
   reading source code to know what stream names a child hardcodes. With
   typed exports, a child's contract is visible from its WIT alone.

3. **Standards path** — WASI 0.3 adds `stream<T>` and `future<T>` as
   first-class WIT types. When that lands, `list<record-envelope>` becomes
   `stream<record-envelope>` — a natural evolution. But only if we have
   `record-envelope` as a WIT type in the first place. Building typed
   interfaces now creates the migration path to standard async streaming.

Patina aspires to be a Bytecode Alliance member and contributor. That
means our architecture should demonstrate the component model's value,
not work around it. This refactor closes the last major alignment gap.

## Design Philosophy

Follow WASM/WASI/Component Model standards for typed interfaces between
components. Adapt only where Patina's needs are genuinely outside:

- **Standard:** typed WIT interfaces for data contracts (this is what WIT is for)
- **Standard:** component-level import/export declarations
- **Adaptation:** Mother as runtime coordinator (dynamic composition, event brokering, cursor/ack)
- **Adaptation:** child lifecycle (on_load, tick, drain, health — no WASI equivalent for managed component lifecycle)

## Resolved Decisions

### 1. Two worlds, not optional exports

WIT worlds require all exports to be satisfied. The component model does
not support optional exports in a single world.

**Decision:** Define two worlds in `wit/child/`:

- `child` — existing world. Lifecycle + `handle`. Service children.
- `child-record-processor` — includes `child` plus exports `record-processor`.

Children declare their world in `child.toml` via `world` field. Host linker
reads this to select the correct bindgen bindings and dispatch path.

**Why not one world:** wasmtime's `bindgen!` generates Rust types for all
exports in a world. If `record-processor` is in the world, all children
must implement it or fail to instantiate. Separate worlds avoid this.

### 2. Separate registration macro

**Decision:** New `register_record_processor!` macro for children targeting
`child-record-processor` world. Existing `register_child!` unchanged.

**Why not auto-detect:** Rust macros expand before trait resolution. The
macro cannot know at expansion time whether a type implements
`RecordProcessor`. Explicit macro selection is clearer and matches the
explicit world targeting in child.toml.

### 3. Mother owns data-plane IO for record-processor children

**Decision:** When a child targets `child-record-processor`, Mother handles
subscribe/ack/emit. The child is a pure transform: `process(records) ->
process-result`.

Today (child manages own IO):
```
tick/handle called -> child subscribes -> child processes -> child emits -> child acks
```

After (Mother manages IO for record-processor children):
```
Mother subscribes -> Mother calls child.process(records) -> Mother emits accepted -> Mother acks
```

**Why this shift:** The typed interface `process(list<record-envelope>)`
means the child receives records as function arguments, not by pulling from
streams. Mother must be the one pulling from streams and passing records in.
This is Wagner's "virtual platform layering" — the platform owns IO, the
component owns compute.

**Impact:** Migrated children lose direct access to events-stream and
messaging toys for their data path. They may still use these toys for
non-data-path concerns (e.g., health reporting), but the primary record
flow is Mother-mediated.

### 4. Output contract: process-result

```wit
record process-result {
    accepted: list<record-envelope>,
    rejected: list<rejected-record>,
}

process: func(records: list<record-envelope>) -> result<process-result, string>;
```

- `Ok(process-result)` — normal operation. Accepted records flow downstream,
  rejected records are logged/routed separately.
- `Err(string)` — infrastructure failure (child trapped, resource exhaustion).
  Mother handles retry/dead-letter.

**Why not just `list<record-envelope>`:** schema-enforcer produces both
accepted and rejected records. Rejected records need a reason string.
Collapsing this into a single list would lose the rejection reason or
require a variant type, which is more complex than a flat result struct.

### 5. Batch, not streaming (for now)

**Decision:** `list<record-envelope>` in, `process-result` out.

**Why not `stream<T>`:** WASI 0.3 is not yet shipped in wasmtime. When it
lands, `list<record-envelope>` migrates naturally to `stream<record-envelope>`.
The types are correct now; only the transport changes.

### 6. child.toml declares world

Migrated children add:

```toml
[child]
world = "child-record-processor"
```

This replaces `[needs.scopes.events].subscribe` for these children — they
no longer subscribe to streams directly. Mother reads the world declaration
and manages stream IO. Default is `world = "child"` (backward compatible).

### 7. Structured pando wiring

`pando.toml` wiring is currently `Vec<String>`:

```toml
wiring = ["schema-enforcer.record.validated -> dedup-filter"]
```

For type validation, Mother must parse this into structured form:

```rust
struct WiringRule {
    source_child: String,
    event_type: String,
    target_child: String,
}
```

At pando load time, Mother checks:
- Source child's world — does it export `record-processor`?
- Target child's world — does it accept records?
- Are the WIT types compatible?

The wiring syntax stays the same in pando.toml; parsing becomes structured.

## Build Target

### Phase A: Foundation

#### A1: Define `patina:record@0.1.0` (cte1)

**New files:**

`wit/record/record.wit`:
```wit
package patina:record@0.1.0;

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

    record file-found {
        source-path: string,
        source-hash: string,
        source-size-bytes: u64,
        discovered-at: string,
    }

    record file-written {
        file-path: string,
        record-count: u64,
        written-at: string,
    }

    record rejected-record {
        reason: string,
        envelope: record-envelope,
    }

    record process-result {
        accepted: list<record-envelope>,
        rejected: list<rejected-record>,
    }
}

interface record-processor {
    use record-types.{record-envelope, process-result};

    /// Process a batch of records. Returns accepted and rejected.
    process: func(records: list<record-envelope>) -> result<process-result, string>;
}
```

**WIT placement:** Following existing convention, the canonical source is
`wit/record/`. Deps are mirrored to `wit/child/deps/patina-record.wit`
and `sdk/patina-sdk/wit/child/deps/patina-record.wit` so that bindgen
for both host and guest can resolve the package. The pre-push WIT
consistency check (`resources/scripts/check-wit-consistency.sh`) must
pass after mirroring.

#### A2: Define `child-record-processor` world (cte2)

**Modify:** `wit/child/child.wit`

Add after the existing `child` world:

```wit
world child-record-processor {
    include child;
    export patina:record/record-processor;
}
```

This uses the component model's `include` mechanism — the new world
inherits all imports and exports from `child`, then adds the typed export.
Children targeting this world must satisfy all `child` exports PLUS
`record-processor`.

#### A3: SDK RecordProcessor trait + macro (cte3)

**New file:** `sdk/patina-sdk/src/record.rs`

```rust
use crate::child::host::GuestHost;

pub use crate::child::patina::record::record_types::{
    RecordEnvelope, FileFound, FileWritten, RejectedRecord, ProcessResult,
};

pub trait RecordProcessor {
    fn process(&mut self, records: Vec<RecordEnvelope>) -> Result<ProcessResult, String>;
}
```

**New macro:** `register_record_processor!` in `sdk/patina-sdk/src/child.rs`

This macro generates:
1. All `child` world exports (same as `register_child!`)
2. The `record-processor` export shim that delegates to `RecordProcessor::process`

The macro takes a type that implements both `Child` and `RecordProcessor`.

#### A4: Host dispatch for both worlds (cte4)

**Modify:** `src/child/internal/child.rs`

Two bindgen invocations:
```rust
mod bindings {
    wasmtime::component::bindgen!({ path: "wit/child/", world: "child" });
}
mod record_bindings {
    wasmtime::component::bindgen!({ path: "wit/child/", world: "child-record-processor" });
}
```

`ChildEngine` reads `child.toml` world field to select which bindings
to use at instantiation. `WasmChild` struct gains an enum:

```rust
enum WasmChildInstance {
    Standard { instance: bindings::Child },
    RecordProcessor { instance: record_bindings::ChildRecordProcessor },
}
```

When Mother's event broker has records for a `RecordProcessor` child,
it calls `instance.call_process()` directly with typed records. No JSON
serialization.

**Data-plane ownership:** For `RecordProcessor` children, Mother:
1. Subscribes to the input stream (per pando wiring)
2. Deserializes events into `RecordEnvelope` (from JSON in event broker)
3. Calls `child.process(records)` with typed data
4. Emits `accepted` records to the output stream
5. Logs/routes `rejected` records
6. Acks the input stream offset

This means the subscribe/ack/emit code currently in each child moves to
Mother's pando execution path. The JSON boundary moves from child↔child
to Mother↔event-broker (where it already exists).

### Phase B: Migrate two children

#### B1: schema-enforcer (cte5a)

**Modify:** `children/schema-enforcer/src/lib.rs`

- Remove: `subscribe("record.extracted")`, `emit("record.validated")`, `ack()`
- Remove: local `Record` struct (use WIT-generated `RecordEnvelope`)
- Add: `impl RecordProcessor for SchemaEnforcerChild`
- Change: `register_child!` → `register_record_processor!`

**Modify:** `children/schema-enforcer/child.toml`

```toml
[child]
world = "child-record-processor"
```

Remove `[needs.scopes.events].subscribe` and `[needs].toys` entries for
events and messaging (data-plane IO is now Mother-managed).

**Modify:** `children/schema-enforcer/Cargo.toml`

Update `[package.metadata.component.target].world` to `child-record-processor`.

**Verify:** `cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2`

#### B2: dedup-filter (cte5b)

Same pattern as schema-enforcer. Additionally, dedup-filter uses
`wasi:keyvalue` for content-hash state — this toy stays (it's not
data-plane IO, it's computation state).

**Verify:** `cargo build -p patina-ai-child-dedup-filter --target wasm32-wasip2`

### Phase C: Pando validation + backward compat

#### C1: Structured pando wiring (cte6)

**Modify:** `mother/src/pando.rs`

Parse `wiring` strings into `WiringRule` structs. At pando load time,
for each wiring rule:
- Look up source child's world from its `child.toml`
- Look up target child's world from its `child.toml`
- If target is `child-record-processor`, verify source emits compatible type
- Reject incompatible wiring with descriptive error

#### C2: Backward compatibility verification (cte7)

- Service children unchanged (child world, handle dispatch)
- All existing tests pass
- Integration tests verify end-to-end pando execution with mixed worlds

## Direct Code Targets

### Phase A (foundation)
- `wit/record/record.wit` — new, shared record types + processor interface
- `wit/child/deps/patina-record.wit` — mirror for child world bindgen
- `sdk/patina-sdk/wit/child/deps/patina-record.wit` — mirror for SDK bindgen
- `wit/child/child.wit` — add `child-record-processor` world
- `sdk/patina-sdk/src/record.rs` — new RecordProcessor trait + type re-exports
- `sdk/patina-sdk/src/child.rs` — new `register_record_processor!` macro
- `sdk/patina-sdk/src/lib.rs` — re-export record module
- `src/child/internal/child.rs:124` — second bindgen for new world
- `src/child/internal/child.rs:1078` — WasmChildInstance enum
- `src/child/internal/child.rs:1138` — dispatch split by world
- `src/child/internal/mod.rs` — manifest parsing for world field

### Phase B (child migration)
- `children/schema-enforcer/src/lib.rs` — RecordProcessor impl
- `children/schema-enforcer/child.toml` — world declaration, remove event scopes
- `children/schema-enforcer/Cargo.toml` — world target
- `children/dedup-filter/src/lib.rs` — RecordProcessor impl
- `children/dedup-filter/child.toml` — world declaration, remove event scopes
- `children/dedup-filter/Cargo.toml` — world target

### Phase C (pando + compat)
- `mother/src/pando.rs` — structured WiringRule, type validation
- `tests/wasm_integration.rs` — add typed dispatch tests

## Commits

1. `feat(wit): define patina:record@0.1.0 shared record types — CTE1`
2. `feat(wit): add child-record-processor world — CTE2`
3. `feat(sdk): add RecordProcessor trait and register_record_processor macro — CTE3`
4. `feat(host): dual-world dispatch with typed record-processor path — CTE4`
5. `refactor(children): migrate schema-enforcer to typed exports — CTE5a`
6. `refactor(children): migrate dedup-filter to typed exports — CTE5b`
7. `feat(pando): structured wiring rules with type validation — CTE6`
8. `test: verify backward compat and wasm32 export generation — CTE7`

## Verification Plan

- `cargo check --workspace -q` after each commit
- `cargo test -q --lib` after each commit (728+ tests)
- `cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2` (Phase B)
- `cargo build -p patina-ai-child-dedup-filter --target wasm32-wasip2` (Phase B)
- `cargo test --test wasm_integration` (Phase C — proves wasm export wiring)
- Pre-push WIT consistency check passes
- Type mismatch test: pando with incompatible wiring rejected at load

## Build Readiness

**Ready after this review.** Prerequisites are complete (shims removed,
naming clean, SDK consolidated, 728 tests passing). All three architectural
forks are resolved:

1. ~~Optional exports vs separate worlds~~ → **Two worlds** (resolved)
2. ~~Macro auto-detection~~ → **Separate `register_record_processor!`** (resolved)
3. ~~Data-plane ownership~~ → **Mother owns IO for record-processor children** (resolved)

Promote to active after audit review confirms no remaining ambiguity.
