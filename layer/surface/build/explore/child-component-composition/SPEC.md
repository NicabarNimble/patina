---
type: explore
id: child-component-composition
status: draft
created: 2026-04-08
sessions:
  origin: 20260408-120617-842723000
beliefs:
  - "[[wasi-is-foundation-not-option]]"
  - "[[children-have-agency-toys-are-capabilities]]"
related:
  - wit/child/child.wit
  - children/file-system-monitor/
  - children/content-extractor/
  - children/schema-enforcer/
  - children/dedup-filter/
  - children/record-writer/
  - children/lakehouse-catalog/
  - resources/pandos/folder-text-to-parquet/pando.toml
  - sdk/patina-sdk/
  - src/child/internal/child.rs
exit_criteria: []
---
# explore: Typed WIT child composition

> Migrate the 6 canon children from handle(string, string) + event
> broker to typed WIT interfaces + wac composition. Both the old
> (handle) and new (typed) paths must work during migration.

## Spike Result

Proven: two minimal components with typed WIT interfaces compose
via `wac plug`. The composed component merges shared imports (WASI)
and resolves internal typed links. Producer's export wires to
consumer's import. Composed component exposes only the outer export
and merged platform imports.

## Current State

6 canon children in folder-text-to-parquet pando. All export the
same `child` world. Data flows as JSON strings through Mother's
event broker.

```
file-system-monitor          [scan]
  toys: logging, messaging, measure, filesystem
  subscribes: nothing
  emits: file.found
       ↓ event broker (JSON)
content-extractor            [extract-found]
  toys: logging, events, messaging, filesystem
  subscribes: file.found
  emits: record.extracted
       ↓ event broker (JSON)
schema-enforcer              [enforce-schema]
  toys: logging, events, messaging, measure
  subscribes: record.extracted
  emits: record.validated, record.rejected
       ↓ event broker (JSON)
dedup-filter                 [filter-dedup]
  toys: logging, events, messaging, keyvalue, measure
  subscribes: record.validated
  emits: record.ready, record.duplicate
       ↓ event broker (JSON)
record-writer                [write-records]
  toys: logging, keyvalue, events, messaging, measure, filesystem
  subscribes: record.ready
  emits: file.written
       ↓ event broker (JSON)
lakehouse-catalog            [register-written]
  toys: logging, keyvalue, events, sql
  subscribes: file.written
  emits: nothing
```

Every child: same serde `Record` struct (duplicated 4 times),
same event subscribe/ack/emit boilerplate, same handle dispatch
pattern, same lifecycle exports (none use tick or drain).

## Target State

6 children with typed WIT exports. Composed into one component
via `wac plug`. Mother provides toys. Lifecycle is opt-in.

```
file-system-monitor
  imports: logging, measure, filesystem
  exports: patina:pipeline/file-source
       ↓ wac composition (typed WIT)
content-extractor
  imports: patina:pipeline/file-source, logging, filesystem
  exports: patina:pipeline/record-source
       ↓ wac composition (typed WIT)
schema-enforcer
  imports: patina:pipeline/record-source, logging, measure
  exports: patina:pipeline/record-source (validated)
       ↓ wac composition (typed WIT)
dedup-filter
  imports: patina:pipeline/record-source, logging, keyvalue, measure
  exports: patina:pipeline/record-source (deduplicated)
       ↓ wac composition (typed WIT)
record-writer
  imports: patina:pipeline/record-source, logging, keyvalue, measure, filesystem
  exports: patina:pipeline/file-sink
       ↓ wac composition (typed WIT)
lakehouse-catalog
  imports: patina:pipeline/file-sink, logging, keyvalue, sql
  exports: patina:pipeline/catalog-result
```

Composed component:
- imports: all toys (merged by wac)
- exports: entry point to run the pipeline
- lifecycle: optional — only if Mother needs to drive it

## What Changes

### 1. New WIT packages

Define shared types that replace duplicated serde structs.
Names below are working names — final names emerge from the build.

**`wit/toys/patina/pipeline.wit`** (or split into multiple files):

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

    record process-result {
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
}

interface file-source {
    use types.{file-found};
    scan: func(folder: string) -> result<list<file-found>, string>;
}

interface record-extract {
    use types.{file-found, record-envelope};
    extract: func(files: list<file-found>) -> result<list<record-envelope>, string>;
}

interface record-validate {
    use types.{record-envelope, process-result};
    validate: func(records: list<record-envelope>) -> result<process-result, string>;
}

interface record-dedup {
    use types.{record-envelope, process-result};
    dedup: func(records: list<record-envelope>) -> result<process-result, string>;
}

interface record-write {
    use types.{record-envelope, file-written};
    write: func(records: list<record-envelope>) -> result<list<file-written>, string>;
}

interface catalog-register {
    use types.{file-written, catalog-entry};
    register: func(files: list<file-written>) -> result<list<catalog-entry>, string>;
}
```

### 2. Per-child worlds

Each child gets its own world. No more shared `child` world for
pipeline children.

```wit
// wit/worlds/file-system-monitor.wit
world file-system-monitor {
    import wasi:logging/logging@0.1.0;
    import patina:measure/measure@0.1.0;
    import wasi:filesystem/types@0.2.8;
    import wasi:filesystem/preopens@0.2.8;
    export patina:pipeline/file-source@0.1.0;
}
```

```wit
// wit/worlds/content-extractor.wit
world content-extractor {
    import wasi:logging/logging@0.1.0;
    import wasi:filesystem/types@0.2.8;
    import wasi:filesystem/preopens@0.2.8;
    import patina:pipeline/file-source@0.1.0;
    export patina:pipeline/record-extract@0.1.0;
}
```

```wit
// wit/worlds/schema-enforcer.wit
world schema-enforcer {
    import wasi:logging/logging@0.1.0;
    import patina:measure/measure@0.1.0;
    import patina:pipeline/record-extract@0.1.0;
    export patina:pipeline/record-validate@0.1.0;
}
```

```wit
// wit/worlds/dedup-filter.wit
world dedup-filter {
    import wasi:logging/logging@0.1.0;
    import wasi:keyvalue/store@0.2.0;
    import patina:measure/measure@0.1.0;
    import patina:pipeline/record-validate@0.1.0;
    export patina:pipeline/record-dedup@0.1.0;
}
```

```wit
// wit/worlds/record-writer.wit
world record-writer {
    import wasi:logging/logging@0.1.0;
    import wasi:keyvalue/store@0.2.0;
    import patina:measure/measure@0.1.0;
    import wasi:filesystem/types@0.2.8;
    import wasi:filesystem/preopens@0.2.8;
    import patina:pipeline/record-dedup@0.1.0;
    export patina:pipeline/record-write@0.1.0;
}
```

```wit
// wit/worlds/lakehouse-catalog.wit
world lakehouse-catalog {
    import wasi:logging/logging@0.1.0;
    import wasi:keyvalue/store@0.2.0;
    import wasi:sql/readwrite@0.1.0;
    import patina:pipeline/record-write@0.1.0;
    export patina:pipeline/catalog-register@0.1.0;
}
```

### 3. Child code changes

For each child, the changes are mechanical:

**Remove:**
- serde `Record`/`FileFoundEvent`/etc. structs (use WIT-generated types)
- `parse_payload()` JSON parsing
- `emit()` messaging boilerplate
- `events_stream::subscribe()` / `events_stream::ack()` boilerplate
- `handle()` action dispatch
- `Child` trait impl (lifecycle: name, on_load, health)
- `register_child!` macro

**Replace with:**
- WIT-generated types from `patina:pipeline`
- Typed export implementation (e.g., `validate(records) -> process-result`)
- Direct function — data comes in as arguments, goes out as return value
- New `register_*!` macro or `export!()` macro from wit-bindgen

**Keep:**
- Business logic (validate_record, provenance check, dedup hash check, parquet write, catalog SQL)
- Toy usage (logging, measure, keyvalue, sql, filesystem)
- Metric emission

Schema-enforcer before: 228 lines with boilerplate.
Schema-enforcer after: ~80 lines of validation logic + toy calls.

### 4. Composition

Build each child:
```bash
cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2
# ... for each child
```

Compose with wac:
```bash
wac plug --plug file-system-monitor.wasm content-extractor.wasm
wac plug --plug <previous>.wasm schema-enforcer.wasm
wac plug --plug <previous>.wasm dedup-filter.wasm
wac plug --plug <previous>.wasm record-writer.wasm
wac plug --plug <previous>.wasm lakehouse-catalog.wasm
# Or a wac compose file that does all at once
```

Result: one composed .wasm component that:
- imports: all toys (merged)
- exports: the pipeline entry point
- internally: 6 children wired by typed interfaces

### 5. Pando changes

Today `pando.toml`:
```toml
[composition]
wiring = [
  "file-system-monitor.file.found -> content-extractor",
  "content-extractor.record.extracted -> schema-enforcer",
  ...
]
```

After: pando.toml becomes a composition manifest:
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

# ... etc

[composition]
tool = "wac"
# wac resolves the wiring from typed interfaces
# or explicit plug order if needed
```

Or even simpler: a wac composition file (`pando.wac`) that
replaces string wiring with typed composition instructions.

### 6. Mother changes

**Dual dispatch during migration:**
Mother reads the child manifest. If world is `child` (old), dispatch
through handle. If world is a typed world, call the typed export.

**For composed components:**
Mother loads the single composed .wasm. Links toys. Calls the
entry point. Doesn't need to know about the internal children.

**Lifecycle:**
Composed components that need tick (because file-system-monitor
needs it) export `patina:lifecycle/manageable`. Mother calls tick
on the composed component, which triggers the pipeline internally.

Components without lifecycle needs don't export it. Mother doesn't
call tick on pure request-response compositions.

### 7. SDK changes

**New macro:** `export!()` from wit-bindgen replaces `register_child!`.
Or a Patina wrapper macro that generates the export shims.

**New traits:** generated by wit-bindgen from the per-child worlds.
No more single `Child` trait for everyone.

**Keep old SDK:** `Child` trait + `register_child!` stay for
handle-based children. Service children (belief-verifier,
session-writer, spec-manager, doctor) keep using handle until
they have typed interfaces too.

## Build Order

### Phase 1: WIT types + one child (prove it builds)
1. Create `wit/toys/patina/pipeline.wit` with shared types
2. Create world for schema-enforcer
3. Modify schema-enforcer to implement typed export
4. Build, inspect with `wasm-tools component wit`

### Phase 2: Second child + composition (prove wac works)
5. Create world for dedup-filter
6. Modify dedup-filter to implement typed export + import
7. Build both, compose with `wac plug`
8. Inspect composed component

### Phase 3: All 6 + pando (prove the pipeline)
9. Convert remaining 4 children
10. Compose full pipeline
11. Update pando.toml format

### Phase 4: Mother loads composed component
12. Add dual dispatch to Mother (handle world vs typed world)
13. Load composed component, link toys
14. Run the pipeline end-to-end

### Phase 5: Lifecycle + observability
15. Define `patina:lifecycle/manageable` interface
16. Add lifecycle wrapper for composed component
17. Verify metrics flow through from internal children

## What DOESN'T Change

- **Service children** (belief-verifier, session-writer, spec-manager,
  doctor) — keep handle(string, string). They're control-plane, not
  data-pipeline.
- **Toys** — WIT interfaces and Mother implementations unchanged.
- **Mother's core** — runtime store, registry, engine. Only dispatch
  code changes.
- **Tests** — existing wasm_integration tests stay. New tests for
  typed dispatch added alongside.

## Risks

1. **wac composition order matters** — unclear if wac can resolve a
   full 6-child chain in one pass or needs sequential plugging.
2. **Lifecycle for composed components** — need to design how tick
   reaches file-system-monitor inside a composition.
3. **Error propagation** — typed results need to carry errors through
   the chain. Each child returns `result<T, string>` but composition
   needs the chain to stop on error.
4. **State scoping** — dedup-filter and record-writer both use keyvalue.
   In a composed component, do they share state or get separate scopes?
5. **Observable counters** — metrics from internal children need to
   flow out through the composed component's merged measure import.

## Verification

```bash
# Phase 1
cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2
wasm-tools component wit target/.../schema_enforcer.wasm

# Phase 2
wac plug --plug schema-enforcer.wasm dedup-filter.wasm -o composed.wasm
wasm-tools component wit composed.wasm

# Phase 3
# Full pipeline composition
wasm-tools component wit full-pipeline.wasm

# Phase 4
cargo test --test wasm_integration
cargo check --workspace -q
```
