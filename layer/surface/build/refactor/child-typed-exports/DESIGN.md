# Design: Typed WIT exports for canon children

## Why This Design

Children are modular WASM compute blocks that compose into pandos. The
component model's composition primitive is typed interfaces — components
export and import WIT interfaces, and tooling can verify compatibility.
Today all children export `handle(string, string)`, which hides data
contracts inside Rust source as JSON strings with hardcoded stream names.

This design restores component model type guarantees at the child boundary
while keeping Mother's event broker for runtime coordination. Standard
where standards exist, adapt where Patina's needs are outside.

## Build Target

### Phase 0: Define shared record type (cte1)

**New file:** `wit/record/record.wit`

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

    /// Processing result — typed output from record processors
    record process-result {
        accepted: list<record-envelope>,
        rejected: list<rejected-record>,
    }
}
```

This replaces the `Record`, `FileFoundEvent`, `FileWrittenEvent`, and
`RejectedRecord` structs currently duplicated across children. One source
of truth in WIT.

**Also add:** `wit/record/processor.wit`

```wit
package patina:record@0.1.0;

interface record-processor {
    use record-types.{record-envelope, process-result};

    /// Process a batch of records. Returns accepted and rejected.
    process: func(records: list<record-envelope>) -> result<process-result, string>;
}
```

**Mirror to SDK:** `sdk/patina-sdk/wit/record/` (same files, SDK must
mirror canonical WIT for bindgen).

### Phase 1: Extend child world (cte2, cte3)

**Modify:** `wit/child/child.wit`

```wit
world child {
    // ... existing imports unchanged ...

    // Existing lifecycle exports (kept)
    export init: func();
    export name: func() -> string;
    export on-load: func() -> result<_, string>;
    export on-unload: func();
    export health: func() -> child-health;
    export handle: func(action: string, payload: string) -> result<string, string>;
    export drain: func(limit: u32) -> result<list<pending-event>, string>;
    export tick: func() -> list<task-intent>;

    // NEW: optional typed data processing export
    export patina:record/record-processor;
}
```

**Open question:** WIT worlds require all exports to be satisfied. For
optional exports, we may need a separate world (`child-record-processor`)
that extends `child`, or use the component model's `include` mechanism.
Investigate wasmtime support for optional exports vs separate worlds.

**SDK trait:** `sdk/patina-sdk/src/record.rs` (new file)

```rust
pub trait RecordProcessor {
    fn process(&mut self, records: Vec<RecordEnvelope>) -> Result<ProcessResult, String>;
}
```

**SDK macro update:** `sdk/patina-sdk/src/child.rs`

The `register_child!` macro detects if the type implements `RecordProcessor`
and generates the typed WIT export shim alongside the existing `handle` shim.
If not implemented, only `handle` is exported.

### Phase 2: Host dispatch (cte4)

**Modify:** `src/child/internal/child.rs`

The `bindgen!` macro at line 124 generates typed bindings from the child
world. After adding `record-processor` export, it auto-generates
`call_process()` alongside `call_handle()`.

Dispatch in `WasmChild::handle()` (currently line ~1138):

```rust
fn handle(&self, request: &ChildRequest) -> Result<ChildResponse> {
    let inner = self.inner.lock()...;
    let WasmChildInner { store, instance } = &mut *inner;

    // Try typed dispatch first if child exports record-processor
    if self.has_record_processor {
        if let Some(records) = try_deserialize_records(&request.payload) {
            let result = instance.call_process(store, &records)?;
            return Ok(typed_result_to_response(result));
        }
    }

    // Fallback: string dispatch
    let payload_json = serde_json::to_string(&request.payload)?;
    match instance.call_handle(store, &request.action, &payload_json)? { ... }
}
```

The `has_record_processor` flag is set at instantiation by checking if
the component exports the `record-processor` interface.

### Phase 3: Migrate two children (cte5)

**Modify:** `children/schema-enforcer/src/lib.rs`

Before:
```rust
impl Child for SchemaEnforcerChild {
    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
        match action {
            "enforce-schema" => self.enforce_schema(payload),
            ...
        }
    }
}
```

After:
```rust
impl Child for SchemaEnforcerChild {
    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
        Err("use typed record-processor interface".to_string())
    }
}

impl RecordProcessor for SchemaEnforcerChild {
    fn process(&mut self, records: Vec<RecordEnvelope>) -> Result<ProcessResult, String> {
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();
        for record in records {
            match validate_record(&record) {
                Ok(()) => accepted.push(record),
                Err(reason) => rejected.push(RejectedRecord { reason, envelope: record }),
            }
        }
        Ok(ProcessResult { accepted, rejected })
    }
}
```

No more `subscribe("record.extracted")` or `emit("record.validated")`
inside the child. The child processes records — Mother handles the stream
wiring based on pando.toml.

**Same pattern for:** `children/dedup-filter/src/lib.rs`

### Phase 4: Pando type validation (cte6)

**Modify:** `mother/src/pando.rs`

At pando load time, after parsing wiring rules, Mother checks:
- Does the source child export `record-processor`?
- Does the target child import (or can process) the output type?
- Are the WIT types compatible in the wiring chain?

This is validation only — runtime dispatch still uses Mother's event
broker. But type mismatches are caught at load time, not at runtime.

## Direct Code Targets

### Phase 0 (new files)
- `wit/record/record.wit` — new, shared record types
- `wit/record/processor.wit` — new, record-processor interface
- `sdk/patina-sdk/wit/record/` — mirror of canonical WIT

### Phase 1 (additive changes)
- `wit/child/child.wit:41-67` — add optional record-processor export
- `sdk/patina-sdk/src/record.rs` — new RecordProcessor trait
- `sdk/patina-sdk/src/child.rs:1027-1033` — register_child! macro expansion
- `sdk/patina-sdk/src/lib.rs` — re-export record module

### Phase 2 (host dispatch)
- `src/child/internal/child.rs:124-127` — bindgen picks up new exports
- `src/child/internal/child.rs:1078-1086` — WasmChild struct gets has_record_processor flag
- `src/child/internal/child.rs:1138-1148` — dispatch with typed fallback

### Phase 3 (child migration)
- `children/schema-enforcer/src/lib.rs` — implement RecordProcessor, remove stream hardcoding
- `children/dedup-filter/src/lib.rs` — implement RecordProcessor, remove stream hardcoding

### Phase 4 (pando validation)
- `mother/src/pando.rs` — add type compatibility check at load time

## Resolved Decisions

1. **Batch, not streaming** — use `list<record-envelope>` for now. WASI 0.3
   `stream<T>` is the future, but not yet available in wasmtime. Batch
   is correct and migrates cleanly.

2. **Separate world vs optional export** — needs investigation. If wasmtime
   supports optional exports, use one world. If not, define a
   `child-record-processor` world that includes `child` plus the typed export.

3. **handle stays** — it's the control-plane dispatch for service children
   and backward compatibility. Not removed.

4. **record-envelope is the first typed contract** — other domains (beliefs,
   sessions) get typed interfaces when they need them, not preemptively.

## Open Questions

1. Does wasmtime's `bindgen!` support optional exports in a single world,
   or do we need separate worlds for children with/without record-processor?

2. Should `file-found` and `file-written` be separate WIT types (as shown)
   or variant cases of a union type? They serve different pipeline stages.

3. How does Mother discover at instantiation whether a child exports
   `record-processor`? Check the component's export list, or declare it
   in `child.toml`?

## Commits

1. `feat(wit): define patina:record@0.1.0 shared record types — CTE1`
2. `feat(wit): add record-processor export to child world — CTE2`
3. `feat(sdk): add RecordProcessor trait and macro support — CTE3`
4. `feat(host): typed dispatch with handle fallback — CTE4`
5. `refactor(children): migrate schema-enforcer to typed exports — CTE5`
6. `refactor(children): migrate dedup-filter to typed exports — CTE5`
7. `feat(pando): type-aware wiring validation at load time — CTE6`
8. `test: verify backward compat for service children — CTE7`

## Verification Plan

- `cargo check --workspace -q` after each phase
- `cargo test -q --lib` after each phase
- Integration test: load pando with typed children, verify records flow
- Backward compat test: service children unchanged, handle still works
- Type mismatch test: pando with incompatible wiring rejected at load

## Build Readiness

Ready. Prerequisites complete:
- Children use canonical `Child` trait (knowledge_child shims removed)
- `MotherRuntimeStore` renamed (no legacy naming confusion)
- Monolith and stub retired (clean children inventory)
- SDK consolidated with accurate README
- 728 tests passing, pre-push checks green
