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
  - id: ctc1-toy-package
    text: "Shared toy package `patina:record@0.1.0` exists with types (record-envelope, file-found, file-written, etc.) and toy interfaces. All 6 canon children reference it — no duplicated serde Record structs."
    checked: false
  - id: ctc2-typed-children
    text: "All 6 canon children export typed toys instead of handle(string,string). Each child's toybox (world) declares only the outside toys it needs and the inside toys it provides."
    checked: false
  - id: ctc3-mother-composes
    text: "Mother uses wac-graph at runtime to compose children per the pando blueprint. Children stay as individual .wasm files. Mother registers, instantiates, wires, validates, and logs."
    checked: false
  - id: ctc4-security
    text: "child.toml declares inside toy grants. Mother validates every grant at load time. Unauthorized connections rejected. All decisions logged."
    checked: false
  - id: ctc5-end-to-end
    text: "Composed folder-text-to-parquet runs end-to-end: folder in, parquet + catalog out. Mother provides outside toys, calls entry point."
    checked: false
  - id: ctc6-handle-children-work
    text: "Handle-based service children (belief-verifier, session-writer, spec-manager, doctor) continue working unchanged alongside typed children."
    checked: false
  - id: ctc7-observability
    text: "Each child inside the composition emits metrics via patina:measure. Mother sees per-child metrics."
    checked: false
  - id: ctc8-reuse
    text: "Same child package instantiated multiple times in a composition via instance IDs in pando wiring. Proven with test."
    checked: false
---
# refactor: Typed WIT child composition

> Migrate the 6 canon children from handle(string, string) + event
> broker to typed WIT toys composed by Mother at runtime using
> wac-graph. Align Patina's MCT model with the BA component model.

## Vocabulary

These terms are used throughout this spec. No invented terms. Where
a BA/component model term exists, we use it. Where Patina has its
own term, we use ours.

| Term | Definition |
|---|---|
| **Toy** | A WIT interface. A capability. Both imports and exports are toys. Follows `wasi:http` pattern — one package, types + interfaces together. |
| **Outside toy** | A toy provided by Mother at runtime. Logging, keyvalue, filesystem, measure, sql. Mother implements these. |
| **Inside toy** | A toy provided by another child through composition. The child doesn't know it comes from another child — Mother arranged it. |
| **Toybox** | A child's world. The complete set of toys it needs (imports) and provides (exports). Each child has its own toybox. |
| **Package** | `patina:DOMAIN` — where toy types and interfaces live. Named by domain, like `wasi:http`. Contains types + interfaces. Not separate. |
| **Child** | A WASM component. Built to wasm32-wasip2. An individual .wasm file. |
| **Pando** | A composition blueprint. Says which children, wired how. Mother reads it. |
| **Composition** | Wiring children's exports to other children's imports. BA term. Mother does this at runtime using wac-graph. |
| **Instance** | A running copy of a child. One child package can have many instances. Shared-nothing between instances. |

## Problem

All 6 canon children export the same generic `child` world with
`handle(string, string)`. The same `Record` serde struct is duplicated
in 4 children. Data flows as JSON strings through Mother's event broker.
Mother sits in every data path. Children can't be verified to fit
together at build time. The child pool isn't searchable by capability.

## Goal

Children export typed WIT toys. Mother composes them at runtime using
wac-graph per the pando blueprint. Children stay as individual .wasm
files — reusable, replaceable, independently optimizable. Mother
provides outside toys and validates all grants. Inside toys are
wired by Mother, logged, and auditable.

## How Mother Composes (wac-graph)

Mother reads the pando. For each child:

```
1. register_package  — load child .wasm from pool, inspect its types
2. validate grants   — check child.toml: are these toys authorized?
3. log decision      — "granting schema-enforcer: logging, measure (outside),
                        record/transform from content-extractor (inside)"
4. instantiate       — create instance (shared-nothing)
5. wire inside toys  — set_instantiation_argument: export → import
6. export            — declare what the composition exposes to Mother
7. encode            — produce composed component bytes
8. load in wasmtime  — link outside toys, call entry point
```

Children stay as individual .wasm files on disk. The pool is the
inventory. The pando is the blueprint. Mother is the composer.

**Proven:** wac-graph spike successfully composed 2 components at
runtime, loaded result in wasmtime. Second spike proved same package
can be instantiated multiple times (reuse).

## Toy Package: `patina:record`

One package for this pando's data domain. Types + interfaces together,
like `wasi:http` has types + handlers. If types need to split out
later (like `wasi:io` split from `wasi:http`), that happens naturally.

```wit
package patina:record@0.1.0;

/// Shared types — the data vocabulary for this domain
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
}

/// Source toy — scan external world, produce file-found list
interface source {
    use types.{file-found};
    scan: func(folder: string) -> result<list<file-found>, string>;
}

/// Extract toy — file metadata to records
interface extract {
    use types.{file-found, record-envelope};
    extract: func(files: list<file-found>) -> result<list<record-envelope>, string>;
}

/// Transform toy — records in, results out.
/// Same toy used by schema-enforcer AND dedup-filter.
/// Different implementations, same contract.
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

## Per-Child Toyboxes (Worlds)

Each child declares its own toybox. Outside toys (from Mother) as
imports. Inside toy (what it provides) as export. Inside toy (from
upstream child) as import where needed.

**file-system-monitor** — source child. Reads external world.
```wit
world file-system-monitor {
    // Outside toys (Mother provides)
    import wasi:logging/logging@0.1.0;
    import patina:measure/measure@0.1.0;
    import wasi:filesystem/types@0.2.8;
    import wasi:filesystem/preopens@0.2.8;
    // Inside toy (what this child provides)
    export patina:record/source@0.1.0;
}
```

**content-extractor** — takes file-found, reads files, produces records.
```wit
world content-extractor {
    // Outside toys
    import wasi:logging/logging@0.1.0;
    import wasi:filesystem/types@0.2.8;
    import wasi:filesystem/preopens@0.2.8;
    // Inside toy (from upstream child)
    import patina:record/source@0.1.0;
    // Inside toy (what this child provides)
    export patina:record/extract@0.1.0;
}
```

**schema-enforcer** — validates records. Uses the transform toy.
```wit
world schema-enforcer {
    import wasi:logging/logging@0.1.0;
    import patina:measure/measure@0.1.0;
    import patina:record/extract@0.1.0;
    export patina:record/transform@0.1.0;
}
```

**dedup-filter** — deduplicates records. Same transform toy, different implementation.
```wit
world dedup-filter {
    import wasi:logging/logging@0.1.0;
    import wasi:keyvalue/store@0.2.0;
    import patina:measure/measure@0.1.0;
    import patina:record/transform@0.1.0;
    export patina:record/transform@0.1.0;
}
```

**record-writer** — writes records to parquet files.
```wit
world record-writer {
    import wasi:logging/logging@0.1.0;
    import wasi:keyvalue/store@0.2.0;
    import patina:measure/measure@0.1.0;
    import wasi:filesystem/types@0.2.8;
    import wasi:filesystem/preopens@0.2.8;
    import patina:record/transform@0.1.0;
    export patina:record/write@0.1.0;
}
```

**lakehouse-catalog** — registers files in catalog.
```wit
world lakehouse-catalog {
    import wasi:logging/logging@0.1.0;
    import wasi:keyvalue/store@0.2.0;
    import wasi:sql/readwrite@0.1.0;
    import patina:record/write@0.1.0;
    export patina:record/catalog@0.1.0;
}
```

## Child Code Changes

For each child, mechanical:

**Remove:** serde Record struct, parse_payload(), emit() boilerplate,
events_stream subscribe/ack, handle() dispatch, Child trait impl,
register_child! macro, serde/serde_json deps.

**Replace with:** wit_bindgen::generate! pointing to per-child world,
export!() macro, typed toy implementation.

**Keep:** Business logic (validation, dedup hash, parquet write,
catalog SQL), outside toy usage (log, measure, keyvalue, sql,
filesystem) through wit-bindgen generated bindings.

**WIT file location:** Each child carries its own WIT, following the
wasmtime pattern (each crate owns its wit/). The child's Cargo.toml
points wit-bindgen at its local `wit/` directory. Mother never reads
.wit files — she loads the compiled .wasm which has types baked in.

**Example — schema-enforcer after:**
```rust
wit_bindgen::generate!({
    path: "wit",  // children/schema-enforcer/wit/
    world: "schema-enforcer",
    generate_all,
});

struct SchemaEnforcer;

impl exports::patina::record::transform::Guest for SchemaEnforcer {
    fn transform(
        records: Vec<patina::record::types::RecordEnvelope>,
    ) -> Result<patina::record::types::TransformResult, String> {
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();

        for record in records {
            match validate_record(&record) {
                Ok(()) => accepted.push(record),
                Err(reason) => rejected.push(
                    patina::record::types::RejectedRecord { reason, envelope: record }
                ),
            }
        }

        // Metrics via outside toy (Mother provides)
        patina::measure::measure::counter("validated_records", accepted.len() as f64)?;

        Ok(patina::record::types::TransformResult { accepted, rejected })
    }
}

fn validate_record(r: &patina::record::types::RecordEnvelope) -> Result<(), String> {
    if r.record_id.is_empty() { return Err("missing record_id".into()); }
    if r.source_path.is_empty() { return Err("missing source_path".into()); }
    // ... same validation logic as today
    Ok(())
}

export!(SchemaEnforcer);
```

~60 lines. Business logic unchanged. Boilerplate gone. wit-bindgen
is the SDK — Luke Wagner's "SDKs for free."

## Security and Audit

Mother is the authority. Children only know Mother. wac-graph is
Mother's tool — children don't know about composition.

**child.toml gains inside toy acceptance:**

The child declares what toy SHAPES it accepts from inside — not who
provides them. The pando knows the wiring. Mother validates the match.

```toml
[child]
name = "dedup-filter"

[needs]
toys = ["logging", "keyvalue", "measure"]

[needs.inside]
accepts = ["patina:record/transform"]
```

The child says "I accept a transform toy from inside." The pando says
"wire schema-enforcer's transform to dedup-filter." Mother validates:
schema-enforcer exports transform, dedup-filter accepts transform,
pando wiring matches — grant approved.

**Mother validates at load time:**
1. Load each child .wasm — `register_package`
2. Read child.toml — check outside toy grants
3. Read child.toml — check inside toy acceptance list
4. Read pando — check wiring matches accepted toy shapes
5. Reject if unauthorized — child accepts a toy shape the pando doesn't
   wire, or pando wires a toy the child doesn't accept
6. Log every decision:
   ```
   [GRANT] dedup-filter: logging (outside, Mother)
   [GRANT] dedup-filter: keyvalue (outside, Mother)
   [GRANT] dedup-filter: measure (outside, Mother)
   [GRANT] dedup-filter: patina:record/transform (inside, wired from schema-enforcer per pando)
   [DENY]  dedup-filter: sql (not declared in child.toml)
   ```
7. Wire inside toys — `set_instantiation_argument`
8. Encode and load

**Tamper verification:**
- Composed bytes are produced by Mother from individual .wasm files
- Mother can hash each child .wasm before composition
- Pando declares expected children + versions
- If a .wasm doesn't match expected hash, reject

**Traceability:**
- Every grant logged with timestamp, child name, toy name, source
- Every composition logged: which children, which connections
- Mother's audit log shows full history of what was granted to whom

## Mother Changes

**Add wac-graph dependency.** Mother uses `CompositionGraph` at runtime.

**New composition path alongside existing dispatch:**
```rust
enum LoadedComponent {
    /// Existing handle-based child
    HandleBased { instance: child_bindings::Child },
    /// Composed pando — Mother built this via wac-graph
    Composed { instance: /* typed bindgen for composed world */ },
}
```

**Follow wasmtime model:** separate bindgen per world type. The
composed component has a different world than `child`. Mother needs
bindgen for both. Like wasmtime has wasmtime-wasi and wasmtime-wasi-http
as separate crates with separate linker setup.

**Outside toy linking unchanged:** Mother's existing link_log(),
link_state(), link_store() etc. work the same. They add toy
implementations to the linker. The composed component's merged
outside toy imports get satisfied the same way individual children's
imports do today.

## Pando Changes

The pando becomes a composition blueprint that Mother reads and
executes via wac-graph:

```toml
[pando]
name = "folder-text-to-parquet"
version = "0.2.0"

# Each child entry creates one instance. For multi-instance reuse,
# add an `id` to disambiguate. Wiring references the id.
# Example: { name = "dedup-filter", id = "dedup-1" }
[[children]]
name = "file-system-monitor"

[[children]]
name = "content-extractor"

[[children]]
name = "schema-enforcer"

[[children]]
name = "dedup-filter"

[[children]]
name = "record-writer"

[[children]]
name = "lakehouse-catalog"

[composition]
entry = { child = "file-system-monitor", toy = "patina:record/source" }

[[composition.wiring]]
from = "file-system-monitor"
to = "content-extractor"
toy = "patina:record/source"

[[composition.wiring]]
from = "content-extractor"
to = "schema-enforcer"
toy = "patina:record/extract"

[[composition.wiring]]
from = "schema-enforcer"
to = "dedup-filter"
toy = "patina:record/transform"

[[composition.wiring]]
from = "dedup-filter"
to = "record-writer"
toy = "patina:record/transform"

[[composition.wiring]]
from = "record-writer"
to = "lakehouse-catalog"
toy = "patina:record/write"
```

Typed wiring replaces string wiring. Each rule names the children
and the toy that connects them. Mother reads this, loads each child,
builds the wac-graph, validates grants against child.toml, wires,
composes, loads.

**Parser compatibility:** `PandoManifest` currently uses
`deny_unknown_fields`. New fields (`[composition.wiring]`,
`[composition].entry`) must be added as `Option<T>` to the parser
in Phase 1 so old pandos still parse. The `[composition].wiring`
string list format stays for handle-based pandos.

## SDK Changes

**Typed children:** use wit-bindgen directly. `wit_bindgen::generate!`
+ `export!()`. No patina-sdk dependency. The WIT IS the SDK.

**Handle-based children:** patina-sdk with Child trait +
register_child! + granted::* stays. Untouched.

The SDK naturally splits: typed children use BA tools directly
(SDKs for free), handle children use Patina SDK. Over time, as
children get typed toys, patina-sdk shrinks.

## Observability

Every child inside the composition imports `patina:measure` and
`wasi:logging`. These are outside toys — Mother provides one
implementation. When schema-enforcer emits
`counter("validated_records", 1.0)`, that call goes to Mother's
measure implementation.

Mother sees per-child metrics from inside the composition because
each child identifies itself in metric names and log contexts.
The measure toy IS the observability channel.

Connection-level observability (latency between children, throughput)
is future work — a composable observer child that passes data through
and emits metrics. Not in scope.

## Service Children — Out of Scope

Service children (belief-verifier, session-writer, spec-manager,
doctor) stay handle-based. They are control-plane, not data-flow.
They respond to actions like "verify this belief" — request-response
where the action IS the interface. handle(string, string) is the
right shape for them today.

They may get typed toys later when their contracts stabilize. But
that's a separate spec. This spec is about the 6 canon children
in the folder-text-to-parquet pando.

## Decision History

The `child-typed-exports` explore (session 20260408-064526) deferred
typed exports until a second domain (beliefs) provided a second
example alongside records. That gate assumed we'd DESIGN typed
interfaces from domain analysis.

This spec overrides that gate because the approach changed:
- We're not designing interfaces from theory — we're using the BA
  component model's own composition mechanism (wac-graph)
- The toy contracts come from WIT, not from domain analysis
- The composition spike proved the mechanism works
- The insight: composition IS the typing. Children declare what toys
  they export and import. wac-graph wires them. The types emerge from
  the toys, not from studying two domains.

The explore's design work (record-envelope types, process-result
shape, etc.) is still valid and informs this spec. What changed is
HOW we get there, not WHAT we're building.

## Compatibility

Handle-based children must continue working. Mother's current loader
checks child kind at `src/child/internal/child.rs:867`
(`check_capabilities`). The new composition path must not break this.

**Explicit compatibility contract:**
- `child` world children: load via existing path. No changes.
- Composed components: load via new composition path. New dispatch.
- Mother checks manifest/pando to decide which path.
- Integration tests cover both paths in the same test suite.

## Open Questions

1. **Composition entry point** — The pando declares
   `entry = { child = "file-system-monitor", toy = "patina:record/source" }`.
   Mother needs the composed component to expose this as its outer
   export. wac-graph's `graph.export()` can control what gets exported.
   Resolve during Phase 2: verify that wac-graph can expose an
   intermediate child's toy as the composition's outer export, not
   just the final child's.

2. **State scoping** — dedup-filter and record-writer both import
   keyvalue. Shared-nothing inside the composition should keep them
   separate. Need to verify with wasmtime that each instance gets
   its own keyvalue scope.

3. **Rejected records** — transform toy returns accepted + rejected.
   Downstream gets only accepted. Child logs rejections via logging
   toy. If routing rejected records is needed later, that's a
   composable child — a seam.

4. **WASI plumbing** — Compiled children pull in wasi:cli, wasi:clocks,
   wasi:filesystem even if not needed (from wasm32-wasip2 target).
   These merge in composition. Shouldn't break but adds unused imports.

## Implementation Order

### Phase 1: Toy package + schema-enforcer (one child builds typed)
1. Update `PandoManifest` and `ChildManifest` parsers — add new
   fields as `Option<T>` so old manifests still parse
2. Create `patina:record@0.1.0` WIT package
3. Create schema-enforcer toybox (world) in `children/schema-enforcer/wit/`
4. Rewrite schema-enforcer with wit-bindgen
5. Build, inspect with `wasm-tools component wit`

### Phase 2: dedup-filter + wac-graph composition
5. Create dedup-filter toybox (same transform toy, different impl)
6. Rewrite dedup-filter with wit-bindgen
7. Mother composes both via wac-graph at runtime
8. Verify: inside toy resolved, outside toys listed

### Phase 3: All 6 children
9. Convert remaining 4 children
10. Mother composes all 6 via wac-graph
11. Verify composed component shape

### Phase 4: Mother end-to-end
12. Add wac-graph to Mother's dependencies
13. Add composition path + dispatch enum
14. Load composed component, link outside toys, call entry point
15. Run end-to-end: folder → parquet → catalog

### Phase 5: Security + both lanes
16. child.toml inside toy grants
17. Mother validates and logs all grants
18. Verify handle-based service children still work
19. Full test suite

## Verification

```bash
# Phase 1
cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2
wasm-tools component wit target/.../schema_enforcer.wasm
# Expect: exports patina:record/transform, imports outside toys only

# Phase 2 — Mother composes at runtime
cargo test --test composition_spike
# Expect: wac-graph composes, wasmtime loads, outside toys listed

# Phase 4
cargo test --test wasm_integration
# Expect: end-to-end pipeline runs

# Phase 5
cargo test -q --workspace
cargo check --workspace -q
```

## Non-Goals

- Not migrating service children — they keep handle(string, string)
- Not building lifecycle toy — future work
- Not building observer children — future work
- Not pre-building giant .wasm files — Mother composes at runtime
- Not changing Mother's core (runtime store, registry) — only adding
  composition path
