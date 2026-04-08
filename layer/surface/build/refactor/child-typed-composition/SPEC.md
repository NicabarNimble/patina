---
type: refactor
id: child-typed-composition
status: draft
created: 2026-04-08
sessions:
  origin: 20260408-120617-842723000
beliefs:
  - "[[wasi-is-foundation-not-option]]"
  - "[[children-have-agency-toys-are-capabilities]]"
related:
  - wit/child/child.wit
  - children/
  - resources/pandos/folder-text-to-parquet/pando.toml
  - sdk/patina-sdk/
  - src/child/internal/child.rs
  - layer/surface/build/explore/child-component-composition/
  - layer/surface/build/explore/child-typed-exports/
exit_criteria:
  - id: ctc1-shared-types
    text: "Shared WIT type package exists with record-envelope, file-found, file-written, and pipeline result types. Used by all 6 canon children — no serde Record structs."
    checked: false
  - id: ctc2-typed-toys
    text: "WIT toys defined for pipeline stages: source toy (scan → file-found), transform toy (records in → results out), write toy (records → file-written), catalog toy (file-written → catalog-entry). Children export these toys instead of handle(string,string)."
    checked: false
  - id: ctc3-children-build
    text: "All 6 canon children build as typed WASM components targeting wasm32-wasip2. wasm-tools component wit shows typed exports and only the toys each child needs as imports."
    checked: false
  - id: ctc4-composition-works
    text: "wac plug composes the 6 children into one WASM component. Composed component imports only toys (merged). Internal child-to-child connections resolved."
    checked: false
  - id: ctc5-mother-loads
    text: "Mother loads the composed component, links toys, calls entry point. Pipeline runs end-to-end: folder in, parquet + catalog out."
    checked: false
  - id: ctc6-both-lanes
    text: "Handle-based children (service children) continue to work alongside typed children. Mother dispatches to both."
    checked: false
  - id: ctc7-observability
    text: "Each child inside the composition emits metrics via patina:measure toy. Mother sees per-child metrics from the composed component."
    checked: false
---
# refactor: Typed WIT child composition

> Migrate the 6 canon pipeline children from handle(string, string) +
> event broker to typed WIT toys + wac composition. Align Patina's MCT
> model with the Bytecode Alliance component model.

## Problem

All 6 canon children export the same generic `child` world:

```wit
export handle: func(action: string, payload: string) -> result<string, string>;
```

Data flows as JSON strings through Mother's event broker. This means:
- The same `Record` serde struct is duplicated in 4 children
- Data types are invisible — JSON strings hide the contracts
- Composition is runtime-only — Mother routes events, can't verify at build time
- Children can't be wired directly — Mother is always in the data path
- The child pool isn't searchable by typed capability

This contradicts the component model where components compose via typed
interfaces verified at build time.

## Goal

Children export typed WIT toys. Children compose into one component via
`wac plug`. Mother provides platform toys and lifecycle. Mother is NOT
in the data path between children.

```
Today:   child → Mother event broker → child → Mother event broker → child
After:   child → child → child (wac composition, typed, verified at build time)
         Mother provides toys to all children
```

## Vocabulary

| Term | Meaning in this spec |
|---|---|
| **Toy** | WIT interface — both imports (capabilities) and exports (contracts) |
| **Connection** | Child-to-child data link — resolved by wac at build time |
| **Platform toy** | Toy provided by Mother at runtime (logging, keyvalue, measure, filesystem, sql) |
| **Pipeline toy** | Toy that describes data flow between children (scan, transform, write) |
| **Source child** | Produces data from external world (needs platform toys for access) |
| **Transform child** | Takes data, processes, returns data (may need platform toys for state) |
| **Sink child** | Writes data to external world (needs platform toys for access) |

## Current State

```
file-system-monitor   → content-extractor   → schema-enforcer
  handle("scan")        handle("extract")      handle("enforce")
  toys: log,measure,    toys: log,events,      toys: log,events,
        filesystem,           messaging,             messaging,
        messaging             filesystem             measure
  emits: file.found     sub: file.found        sub: record.extracted
                         emits: record.extracted emits: record.validated

→ dedup-filter         → record-writer        → lakehouse-catalog
  handle("filter")       handle("write")        handle("register")
  toys: log,events,      toys: log,keyvalue,    toys: log,keyvalue,
        messaging,              events,                events,sql
        keyvalue,               messaging,
        measure                 measure,
                                filesystem
  sub: record.validated  sub: record.ready      sub: file.written
  emits: record.ready    emits: file.written
```

Every child: same Child trait, same handle dispatch, same event
subscribe/ack/emit boilerplate, duplicated Record struct.

## Target State

```
file-system-monitor   → content-extractor   → schema-enforcer
  exports: source toy    imports: source toy    imports: transform toy
  platform toys:         exports: extract toy   exports: transform toy
    log, measure,        platform toys:         platform toys:
    filesystem             log, filesystem        log, measure

→ dedup-filter         → record-writer        → lakehouse-catalog
  imports: transform toy imports: transform toy imports: sink toy
  exports: transform toy exports: sink toy      exports: catalog toy
  platform toys:         platform toys:         platform toys:
    log, keyvalue,         log, keyvalue,         log, keyvalue,
    measure                measure, filesystem    sql
```

Connections (→) resolved by wac. Platform toys provided by Mother.
Composed component: one .wasm, imports only platform toys, exports
pipeline entry point.

## Shared WIT Types

One package. These types replace the duplicated serde structs.

```wit
package patina:pipeline@0.1.0;

interface types {
    record file-found {
        source-path: string,
        source-hash: string,
        source-size-bytes: u64,
        discovered-at: string,
    }

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

    record rejected-record {
        reason: string,
        envelope: record-envelope,
    }

    record transform-result {
        accepted: list<record-envelope>,
        rejected: list<rejected-record>,
    }

    record file-written {
        file-path: string,
        record-count: u64,
        written-at: string,
    }

    record catalog-entry {
        file-path: string,
        record-count: u64,
        written-at: string,
        registered-at: string,
        schema-version: u32,
    }

    record pipeline-result {
        files-found: u32,
        records-extracted: u32,
        records-accepted: u32,
        records-rejected: u32,
        files-written: u32,
        catalog-entries: u32,
    }
}
```

## Pipeline Toys

These are the toys children export and import to connect to each other.
The **same transform toy** is used by schema-enforcer AND dedup-filter —
same shape, different implementation.

```wit
package patina:pipeline@0.1.0;

/// Source toy — scan external world, produce file-found list
interface source {
    use types.{file-found};
    scan: func(folder: string) -> result<list<file-found>, string>;
}

/// Extract toy — turn file metadata into records
interface extract {
    use types.{file-found, record-envelope};
    extract: func(files: list<file-found>) -> result<list<record-envelope>, string>;
}

/// Transform toy — records in, results out (validate, dedup, etc.)
/// Same toy, multiple children. Schema-enforcer and dedup-filter
/// both export this. wac chains them.
interface transform {
    use types.{record-envelope, transform-result};
    transform: func(records: list<record-envelope>) -> result<transform-result, string>;
}

/// Write toy — records to files
interface write {
    use types.{record-envelope, file-written};
    write: func(records: list<record-envelope>) -> result<list<file-written>, string>;
}

/// Catalog toy — register written files
interface catalog {
    use types.{file-written, catalog-entry};
    register: func(files: list<file-written>) -> result<list<catalog-entry>, string>;
}
```

## Per-Child Worlds

Each child declares only what it needs (platform toys) and what it
does (pipeline toys). No more shared `child` world.

**file-system-monitor:**
```wit
world file-system-monitor {
    import wasi:logging/logging@0.1.0;
    import patina:measure/measure@0.1.0;
    import wasi:filesystem/types@0.2.8;
    import wasi:filesystem/preopens@0.2.8;
    export patina:pipeline/source@0.1.0;
}
```

**content-extractor:**
```wit
world content-extractor {
    import wasi:logging/logging@0.1.0;
    import wasi:filesystem/types@0.2.8;
    import wasi:filesystem/preopens@0.2.8;
    import patina:pipeline/source@0.1.0;
    export patina:pipeline/extract@0.1.0;
}
```

**schema-enforcer:**
```wit
world schema-enforcer {
    import wasi:logging/logging@0.1.0;
    import patina:measure/measure@0.1.0;
    import patina:pipeline/extract@0.1.0;
    export patina:pipeline/transform@0.1.0;
}
```

**dedup-filter:**
```wit
world dedup-filter {
    import wasi:logging/logging@0.1.0;
    import wasi:keyvalue/store@0.2.0;
    import patina:measure/measure@0.1.0;
    import patina:pipeline/transform@0.1.0;
    export patina:pipeline/transform@0.1.0;
}
```

Note: dedup-filter imports AND exports the same transform toy.
It receives records from schema-enforcer's transform output and
passes them downstream. wac chains same-shaped toys naturally —
proven in the composition spike.

**record-writer:**
```wit
world record-writer {
    import wasi:logging/logging@0.1.0;
    import wasi:keyvalue/store@0.2.0;
    import patina:measure/measure@0.1.0;
    import wasi:filesystem/types@0.2.8;
    import wasi:filesystem/preopens@0.2.8;
    import patina:pipeline/transform@0.1.0;
    export patina:pipeline/write@0.1.0;
}
```

**lakehouse-catalog:**
```wit
world lakehouse-catalog {
    import wasi:logging/logging@0.1.0;
    import wasi:keyvalue/store@0.2.0;
    import wasi:sql/readwrite@0.1.0;
    import patina:pipeline/write@0.1.0;
    export patina:pipeline/catalog@0.1.0;
}
```

## Composition

Build each child, then compose with wac:

```bash
# Build all 6
for child in file-system-monitor content-extractor schema-enforcer \
             dedup-filter record-writer lakehouse-catalog; do
    cargo build -p "patina-ai-child-$child" --target wasm32-wasip2
done

# Compose: plug each child into the next
# wac resolves: export of A matches import of B
wac plug --plug file-system-monitor.wasm content-extractor.wasm -o step1.wasm
wac plug --plug step1.wasm schema-enforcer.wasm -o step2.wasm
wac plug --plug step2.wasm dedup-filter.wasm -o step3.wasm
wac plug --plug step3.wasm record-writer.wasm -o step4.wasm
wac plug --plug step4.wasm lakehouse-catalog.wasm -o pipeline.wasm
```

Result (`pipeline.wasm`):
- **Imports:** logging, measure, keyvalue, filesystem, sql (merged platform toys)
- **Exports:** `patina:pipeline/catalog` (the outermost child's export)
- **Internal:** 6 children wired by typed connections, shared-nothing

Mother calls: run the pipeline by calling the composed entry point.

Open question: the composed component exports `catalog/register`
but what Mother really wants to call is `source/scan` to trigger
the pipeline. The composition wires source→extract→transform→...
but the ENTRY POINT is the source, not the final sink. Need to
verify: does wac expose intermediate interfaces, or only the
outermost child's export? If only the outermost, we may need a
thin wrapper child or a pipeline-level world.

## Child Code Changes

For each child, the change is mechanical:

**Remove:**
- `use patina_sdk::child::{Child, ChildHealth, ...};`
- `register_child!` macro
- `impl Child for ... { handle(), name(), health(), on_load() }`
- serde `Record`, `FileFoundEvent`, etc. structs
- `parse_payload()` JSON parsing
- `emit()` messaging boilerplate
- `events_stream::subscribe()` / `ack()` boilerplate
- `serde`, `serde_json` dependencies

**Replace with:**
- `wit_bindgen::generate!` pointing to per-child world
- `export!()` macro from wit-bindgen
- Typed export implementation (e.g., `transform(records) -> result`)

**Keep:**
- Business logic (validate_record, provenance_complete, dedup hash check,
  parquet write, catalog SQL)
- Platform toy usage (log, measure, keyvalue, sql, filesystem) — but
  through wit-bindgen generated bindings instead of patina-sdk wrappers
- Metric emission via measure toy

**Example: schema-enforcer after**

```rust
wit_bindgen::generate!({
    path: "../wit/worlds/schema-enforcer",
    world: "schema-enforcer",
    generate_all,
});

struct SchemaEnforcer;

impl exports::patina::pipeline::transform::Guest for SchemaEnforcer {
    fn transform(
        records: Vec<patina::pipeline::types::RecordEnvelope>,
    ) -> Result<patina::pipeline::types::TransformResult, String> {
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();

        for record in records {
            match validate_record(&record) {
                Ok(()) => accepted.push(record),
                Err(reason) => rejected.push(
                    patina::pipeline::types::RejectedRecord {
                        reason,
                        envelope: record,
                    }
                ),
            }
        }

        // Metrics via platform toy
        wasi::logging::logging::log(
            wasi::logging::logging::Level::Info,
            "",
            &format!("validated: {} accepted, {} rejected",
                accepted.len(), rejected.len()),
        );

        Ok(patina::pipeline::types::TransformResult { accepted, rejected })
    }
}

fn validate_record(record: &patina::pipeline::types::RecordEnvelope) -> Result<(), String> {
    // Same validation logic as today — unchanged
    if record.record_id.is_empty() { return Err("missing record_id".into()); }
    if record.source_path.is_empty() { return Err("missing source_path".into()); }
    // ... etc
    Ok(())
}

export!(SchemaEnforcer);
```

~60 lines vs 228 today. Business logic unchanged. Boilerplate gone.

## Mother Changes

Follow the wasmtime model: separate bindgen per world, separate
linker setup, same runtime.

**Add second bindgen:**

```rust
// Existing — for handle-based children
mod child_bindings {
    wasmtime::component::bindgen!({
        path: "wit/child/",
        world: "child",
    });
}

// New — for composed pipeline
mod pipeline_bindings {
    wasmtime::component::bindgen!({
        path: "wit/worlds/pipeline/",
        world: "pipeline",
    });
}
```

**Dispatch by world:**

Mother reads child.toml (or the pando manifest). If the component
targets the `child` world, dispatch through handle. If it targets
a pipeline world, call the typed export.

```rust
enum LoadedComponent {
    HandleBased { instance: child_bindings::Child },
    Pipeline { instance: pipeline_bindings::Pipeline },
}
```

Service children (belief-verifier, session-writer, spec-manager,
doctor) stay handle-based. The composed pipeline component uses
the new dispatch path.

**Toy linking unchanged:**

Mother's `link_wasi()`, `link_log()`, `link_state()`, etc. work
the same — they add toy implementations to the linker. The composed
component's merged platform toy imports get satisfied the same way
individual children's imports do today.

## Pando Changes

Today:
```toml
[pando]
name = "folder-text-to-parquet"
version = "0.1.0"

[[children]]
name = "file-system-monitor"
# ... 6 children

[composition]
wiring = [
  "file-system-monitor.file.found -> content-extractor",
  "content-extractor.record.extracted -> schema-enforcer",
  # ...
]
```

After: the wiring is done by wac at build time, not Mother at runtime.
The pando becomes a build manifest:

```toml
[pando]
name = "folder-text-to-parquet"
version = "0.2.0"

[[children]]
name = "file-system-monitor"
world = "file-system-monitor"

[[children]]
name = "content-extractor"
world = "content-extractor"

[[children]]
name = "schema-enforcer"
world = "schema-enforcer"

[[children]]
name = "dedup-filter"
world = "dedup-filter"

[[children]]
name = "record-writer"
world = "record-writer"

[[children]]
name = "lakehouse-catalog"
world = "lakehouse-catalog"

[composition]
tool = "wac"
output = "folder-text-to-parquet.wasm"
```

Mother reads the composed .wasm — she doesn't need to know about
individual children or wiring. The pando told wac how to build it.
Mother loads the result and provides toys.

## SDK Changes

**wit-bindgen is the SDK for typed children.** Luke Wagner's "SDKs
for free" — define WIT, generate bindings automatically.

For typed pipeline children:
- `wit_bindgen::generate!` generates types and export stubs
- `export!()` macro wires the implementation
- Platform toy calls use generated bindings directly
- No patina-sdk dependency needed

For handle-based service children:
- `patina-sdk` with `Child` trait + `register_child!` stays
- `granted::*` convenience wrappers stay
- No changes

The SDK splits into two paths:
1. **Typed children** — use wit-bindgen directly. SDK not needed.
2. **Handle children** — use patina-sdk as today. Untouched.

Over time, as service children get typed interfaces, patina-sdk
shrinks. The WIT definitions ARE the SDK.

## Observability

Each child inside the composition imports `patina:measure` and
`wasi:logging`. wac merges these imports — Mother provides one
implementation. Every child emits metrics and logs through the
platform toy.

Mother sees per-child metrics from inside the composition because
each child identifies itself in metric names/log contexts. The
measure toy is the observability channel.

Connection-level observability (latency between children, data
throughput) is a future enhancement — composable observer child
that sits between pipeline stages. Not in scope for this spec.

## Risks and Open Questions

1. **Composition entry point** — The composed component's outermost
   export is the last child's toy (catalog). But Mother wants to
   trigger the pipeline from the first child (source/scan). Need to
   verify how wac exposes the entry point or whether a thin wrapper
   world is needed.

2. **wac chain of 6** — Spike proved 2. Need to verify 6 works via
   sequential `wac plug` or a compose file.

3. **State scoping** — dedup-filter and record-writer both import
   keyvalue. Shared-nothing should keep them separate inside the
   composition. Need to verify with wasmtime.

4. **Rejected records** — transform toy returns accepted + rejected.
   Downstream child only gets accepted. Rejected records are logged
   by the child via logging toy. If we need rejected record routing,
   that's a future composable child.

5. **WASI plumbing imports** — Compiled components pull in wasi:cli,
   wasi:clocks, wasi:filesystem even if not needed (from the WASM
   target). These merge in composition. Shouldn't cause issues but
   adds unused imports.

## Implementation Order

### Phase 1: WIT types + schema-enforcer (prove one child builds)
1. Create `patina:pipeline@0.1.0` types and toy interfaces
2. Create schema-enforcer world
3. Rewrite schema-enforcer with wit-bindgen
4. Build, inspect with `wasm-tools component wit`
5. Verify: typed export visible, platform toy imports only

### Phase 2: dedup-filter + composition (prove wac chains same toy)
6. Create dedup-filter world (imports + exports transform toy)
7. Rewrite dedup-filter with wit-bindgen
8. Build both, compose with `wac plug`
9. Verify: composed component has merged imports, resolved connection

### Phase 3: All 6 children + full composition
10. Create remaining 4 worlds and rewrite children
11. Compose full 6-child pipeline
12. Verify composed component shape

### Phase 4: Mother loads composed component
13. Add pipeline bindgen to Mother
14. Add dispatch enum (HandleBased vs Pipeline)
15. Load composed component, link toys, call entry point
16. Run end-to-end: folder → parquet → catalog

### Phase 5: Pando + both lanes
17. Update pando.toml format for composition
18. Verify handle-based service children still work
19. Full test suite passes

## Verification

```bash
# Phase 1
cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2
wasm-tools component wit target/.../schema_enforcer.wasm
# Expect: exports patina:pipeline/transform, imports log+measure only

# Phase 2
wac plug --plug schema-enforcer.wasm dedup-filter.wasm -o composed.wasm
wasm-tools component wit composed.wasm
# Expect: exports transform, imports log+measure+keyvalue, no internal connection visible

# Phase 3
# Full 6-child composition
wasm-tools component wit pipeline.wasm
# Expect: imports all platform toys (merged), exports pipeline entry

# Phase 4
cargo check --workspace -q
cargo test -q --lib
cargo test --test wasm_integration

# Phase 5
cargo test -q --workspace
```

## Non-Goals

- Not migrating service children (belief-verifier, session-writer,
  spec-manager, doctor) — they keep handle(string, string)
- Not building lifecycle toy yet — future work
- Not building observer children — future work
- Not changing Mother's core (runtime store, registry) — only dispatch
- Not implementing wac compose files — sequential plug is sufficient
