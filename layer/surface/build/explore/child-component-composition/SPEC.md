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
  - children/schema-enforcer/
  - children/dedup-filter/
  - layer/surface/build/explore/child-typed-exports/
exit_criteria: []
---
# explore: Compose children via typed WIT interfaces

> Can two Patina children be composed into a single component using
> typed WIT interfaces and `wasm-tools compose`, with Mother providing
> only toys?

## Why This Matters

Patina children are WASM components. The component model's composition
mechanism (`wasm-tools compose`) can wire component exports to component
imports, producing a new composed component. This is how the Bytecode
Alliance intends components to be assembled — not through runtime message
brokers, but through typed interface linking.

Today, Patina children all export `handle(string, string)` and pass
data as JSON through Mother's event broker. This means:
- Data types are invisible (JSON strings)
- Composition is runtime-only (Mother routes events)
- Reuse is discoverable only by reading source code
- Children can't be verified to fit together at build time

If children export typed WIT interfaces and compose via `wasm-tools
compose`, then:
- Data types are visible in WIT
- Composition is verifiable at build time
- The child pool becomes a registry of typed contracts
- Pandos become component composition blueprints
- Mother provides toys and lifecycle only — not data routing

## What We're Testing

Take two canon children from the folder-text-to-parquet pipeline:

**schema-enforcer** → validates records against schema
**dedup-filter** → deduplicates records by content hash

Today they're connected by Mother's event broker:
```
schema-enforcer
  subscribes to "record.extracted"
  validates records
  emits to "record.validated"
      ↓ (Mother's event broker, JSON strings)
dedup-filter
  subscribes to "record.validated"
  checks content_hash against state
  emits to "record.ready"
```

We want to compose them directly:
```
schema-enforcer
  exports: record-processor (typed WIT)
      ↓ (wasm-tools compose, typed linking)
dedup-filter
  imports: record-source (typed WIT)
  exports: record-processor (typed WIT)
```

The composed component is one unit that Mother loads. Mother provides
toys (logging, keyvalue, measure). Data flows between children through
typed WIT — no JSON, no event broker for the data path.

## Concrete Steps

### 1. Define shared record types

New WIT package `patina:record@0.1.0`:

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
    process: func(records: list<record-envelope>) -> result<process-result, string>;
}
```

This type already exists as a duplicated Rust `Record` struct in both
`schema-enforcer/src/lib.rs` and `dedup-filter/src/lib.rs`. The WIT
version replaces both copies.

### 2. Define per-child worlds

**schema-enforcer world:**
```wit
world schema-enforcer {
    // Toys from Mother
    import wasi:logging/logging@0.1.0;
    import patina:measure/measure@0.1.0;

    // Typed data export
    export patina:record/record-processor;
}
```

**dedup-filter world:**
```wit
world dedup-filter {
    // Toys from Mother
    import wasi:logging/logging@0.1.0;
    import wasi:keyvalue/store@0.2.0;
    import patina:measure/measure@0.1.0;

    // Typed data — in from schema-enforcer, out to next child
    import patina:record/record-processor;  // gets records from upstream
    export patina:record/record-processor;  // passes records downstream
}
```

Note: both children import only the toys they actually need, not the
full child world. This is the component model way — declare exactly
what you need.

### 3. Build both as WASM components

Modify each child's `src/lib.rs` to:
- Remove serde `Record` struct (use WIT-generated type)
- Remove event subscribe/emit (data comes through typed interface)
- Implement the `record-processor` export
- Keep toy usage (logging, measure, keyvalue)

Build with: `cargo build --target wasm32-wasip2`

### 4. Compose with wasm-tools

```bash
wasm-tools compose \
    schema-enforcer.wasm \
    --definitions dedup-filter.wasm \
    -o composed-pipeline.wasm
```

This should wire schema-enforcer's `record-processor` export to
dedup-filter's `record-processor` import. Toy imports (logging,
keyvalue, measure) stay unsatisfied — Mother fills those at runtime.

### 5. Load in Mother

Mother loads `composed-pipeline.wasm` as a single component. Links
toys. Calls the outermost `record-processor` export to push records
through the pipeline. Both children execute in sequence, typed,
verified.

## What We're Proving

1. **Typed WIT works for child data** — the duplicated `Record` serde
   struct becomes a shared WIT type.
2. **wasm-tools compose works for children** — two children can be
   wired together at build time.
3. **Mother only provides toys** — she's the platform, not the data
   router.
4. **The pando blueprint maps to component composition** — wiring
   rules become composition instructions.

## What We're NOT Doing

- Not migrating all children — just schema-enforcer + dedup-filter
- Not changing Mother's existing dispatch — composed component runs
  alongside existing handle-based children
- Not removing event broker — it stays for non-composed children
- Not building the full pipeline — just two children as proof

## Known Unknowns

1. **Does wasm-tools compose handle shared toy imports?** Both
   children import logging. Does the composition merge those imports
   or conflict?
2. **Can Mother load a composed component with its current bindgen?**
   The composed component has a different world than `child`. Mother's
   dispatch needs to handle this.
3. **Does the same interface work for both import and export?** If
   dedup-filter both imports and exports `record-processor`, does
   wasm-tools compose wire the right direction?
4. **Resource usage** — does the composed component get one linear
   memory or two? (Should be two — shared nothing between children.)
5. **How does keyvalue state scope work?** dedup-filter uses keyvalue
   for hash state. In a composed component, Mother links it once — does
   it scope correctly?

## Verification

```bash
# Build both children
cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2
cargo build -p patina-ai-child-dedup-filter --target wasm32-wasip2

# Compose
wasm-tools compose schema-enforcer.wasm -d dedup-filter.wasm -o composed.wasm

# Inspect the result
wasm-tools component wit composed.wasm

# Verify it has the right shape
# - imports: logging, keyvalue, measure (toys)
# - exports: record-processor (data contract)
```

## Relationship to child-typed-exports

The `child-typed-exports` explore designed much of the WIT type system
(record-envelope, process-result, etc.) and identified key decisions
(two worlds, separate macros, data-plane ownership). That work informs
this explore.

The key difference: child-typed-exports assumed Mother mediates typed
data (path 1). This explore tests direct child-to-child composition
(path 2) — the component model way. If this works, path 1 is
unnecessary.
