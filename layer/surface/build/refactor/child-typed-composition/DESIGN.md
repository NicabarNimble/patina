# Design: Typed WIT child composition

## Why This Design

Patina children are WASM components. The Bytecode Alliance component
model defines how components compose: typed exports wire to typed
imports. Patina currently bypasses this — all data flows as JSON
strings through `handle(string, string)` and Mother's event broker.

This design aligns Patina's MCT model with the BA component model:
- Children export typed toys (WIT interfaces)
- Mother composes children at runtime using wac-graph
- Inside toys connect children. Outside toys come from Mother.
- Children stay individual .wasm files — reusable, replaceable
- Security is Mother's: deny by default, explicit grants, auditable

The design follows BA conventions where they exist:
- Package naming: `patina:record` like `wasi:http`
- Types + interfaces in one package (like `wasi:http` has types + handlers)
- Same toy exported by multiple children (like `wasi:http` has incoming + outgoing handlers)
- wit-bindgen as SDK (Luke Wagner's "SDKs for free")
- wac-graph for programmatic composition (BA's official composition library)

## Resolved Decisions

### 1. Mother composes at runtime, not build time

**Decision:** Mother uses wac-graph at runtime to compose children.
No pre-built giant .wasm files.

**Why:** Mother must see each child individually to validate grants,
log decisions, and maintain authority. Pre-built composition bypasses
Mother's security model. Children stay individual .wasm files so they
can be reused, replaced, and hot-swapped.

**Proven:** wac-graph spike composed two components at runtime, loaded
result in wasmtime. Second spike proved same package instantiates
multiple times.

### 2. Inside toys and outside toys are both just toys

**Decision:** A child's imports are toys. Some come from Mother
(outside). Some come from other children via composition (inside).
The child doesn't know the difference.

**Why:** Follows the component model — a component doesn't know who
satisfies its imports. Mother arranges everything. This preserves
Mother's authority and keeps children simple.

### 3. Same transform toy for multiple children

**Decision:** schema-enforcer and dedup-filter both export
`patina:record/transform`. Same interface, different implementations.

**Why:** They do the same shaped thing: records in, results out. The
interface IS the contract. wac-graph chains them: the composed result
passes through both transforms in sequence. Proven in spike.

### 4. One toy package per data domain

**Decision:** `patina:record@0.1.0` contains all types and interfaces
for the record-processing domain. Types and interfaces together, not
separate.

**Why:** Follows `wasi:http` pattern. HTTP types (request, response,
error-code) live with HTTP handlers (incoming-handler, outgoing-handler)
in one package. If types need to split later (like `wasi:io` split
from `wasi:http`), that happens naturally.

### 5. wit-bindgen is the SDK for typed children

**Decision:** Typed children use `wit_bindgen::generate!` + `export!`
directly. No patina-sdk wrapper needed.

**Why:** Luke Wagner's "SDKs for free" — define WIT, generate bindings
automatically. The WIT IS the SDK. patina-sdk stays for handle-based
children only.

### 6. child.toml declares inside toy grants

**Decision:** child.toml gains `[needs.inside]` section declaring which
inside toys a child accepts and from whom.

**Why:** Mother validates every grant at load time. Without declared
inside toy grants, Mother can't enforce deny-by-default for child-to-
child connections. Same security model as outside toys, extended.

### 7. Service children stay handle-based

**Decision:** belief-verifier, session-writer, spec-manager, doctor
keep `handle(string, string)`. Out of scope.

**Why:** They're control-plane, not data-flow. Request-response where
the action IS the interface. They may get typed toys later when their
contracts stabilize. Separate spec.

## Build Target

### Phase 1: Toy package + schema-enforcer

**New files:**
- `wit/toys/patina/record.wit` — `patina:record@0.1.0` types + interfaces
- `wit/worlds/schema-enforcer.wit` — schema-enforcer toybox
- Deps copies in `children/schema-enforcer/wit/` (per self-contained parts pattern)

**Modified files:**
- `children/schema-enforcer/src/lib.rs` — rewrite with wit-bindgen
- `children/schema-enforcer/Cargo.toml` — replace patina-sdk with wit-bindgen, update component metadata

**Removed from schema-enforcer:**
- serde `Record` struct (use WIT-generated type)
- `parse_payload()`, `emit()` boilerplate
- `events_stream::subscribe/ack` calls
- `Child` trait impl, `register_child!` macro
- `serde`, `serde_json` dependencies

**Verify:**
```bash
cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2
wasm-tools component wit target/.../schema_enforcer.wasm
```
Expect: exports `patina:record/transform@0.1.0`, imports only outside toys.

### Phase 2: dedup-filter + runtime composition

**New files:**
- `wit/worlds/dedup-filter.wit` — dedup-filter toybox

**Modified files:**
- `children/dedup-filter/src/lib.rs` — rewrite with wit-bindgen
- `children/dedup-filter/Cargo.toml` — same changes as schema-enforcer

**New test:**
- `tests/composition_spike.rs` — Mother composes schema-enforcer +
  dedup-filter via wac-graph, loads in wasmtime, verifies shape

**Verify:**
```bash
cargo build -p patina-ai-child-dedup-filter --target wasm32-wasip2
cargo test --test composition_spike
```

### Phase 3: Remaining 4 children

**New files:**
- `wit/worlds/file-system-monitor.wit`
- `wit/worlds/content-extractor.wit`
- `wit/worlds/record-writer.wit`
- `wit/worlds/lakehouse-catalog.wit`

**Modified files:**
- `children/file-system-monitor/src/lib.rs`
- `children/content-extractor/src/lib.rs`
- `children/record-writer/src/lib.rs`
- `children/lakehouse-catalog/src/lib.rs`
- Corresponding Cargo.toml files

### Phase 4: Mother composition path

**Modified files:**
- `Cargo.toml` — add wac-graph dependency
- `src/child/internal/child.rs` — add composed component bindgen + dispatch
- `mother/src/pando.rs` — read new pando format, build wac-graph
- `mother/src/registry.rs` — support LoadedComponent enum

**New in Mother:**
```rust
// Second bindgen for composed pipeline world
mod composed_bindings {
    wasmtime::component::bindgen!({
        path: "wit/worlds/composed-pipeline",
        world: "composed-pipeline",
    });
}

enum LoadedComponent {
    HandleBased { instance: child_bindings::Child },
    Composed { instance: composed_bindings::ComposedPipeline },
}
```

### Phase 5: Security + both lanes

**Modified files:**
- child.toml for each canon child — add `[needs.inside]` section
- `src/child/internal/mod.rs` — parse inside toy grants from child.toml
- Mother composition code — validate grants, log decisions

## Direct Code Targets

### Phase 1
- `wit/toys/patina/record.wit` — new, toy package
- `children/schema-enforcer/src/lib.rs` — full rewrite
- `children/schema-enforcer/Cargo.toml` — deps change

### Phase 2
- `children/dedup-filter/src/lib.rs` — full rewrite
- `children/dedup-filter/Cargo.toml` — deps change
- `tests/composition_spike.rs` — new integration test

### Phase 3
- `children/file-system-monitor/src/lib.rs` — full rewrite
- `children/content-extractor/src/lib.rs` — full rewrite
- `children/record-writer/src/lib.rs` — full rewrite
- `children/lakehouse-catalog/src/lib.rs` — full rewrite

### Phase 4
- `Cargo.toml` — add wac-graph
- `src/child/internal/child.rs:124` — add second bindgen
- `src/child/internal/child.rs:837` — build_linker dispatch
- `mother/src/pando.rs` — wac-graph composition logic
- `mother/src/registry.rs` — LoadedComponent enum

### Phase 5
- `children/*/child.toml` — add [needs.inside] sections
- `src/child/internal/mod.rs` — parse inside grants

## Commits

Phase 1:
1. `wit(record): define patina:record@0.1.0 toy package` — types + interfaces
2. `refactor(schema-enforcer): typed WIT export via wit-bindgen` — rewrite child

Phase 2:
3. `refactor(dedup-filter): typed WIT export, same transform toy` — rewrite child
4. `test: Mother runtime composition via wac-graph` — integration test

Phase 3:
5. `refactor(file-system-monitor): typed source export` — rewrite child
6. `refactor(content-extractor): typed extract export` — rewrite child
7. `refactor(record-writer): typed write export` — rewrite child
8. `refactor(lakehouse-catalog): typed catalog export` — rewrite child

Phase 4:
9. `feat(mother): wac-graph runtime composition` — composition path
10. `feat(mother): dual dispatch for handle and composed worlds` — both lanes

Phase 5:
11. `feat(child.toml): inside toy grant declarations` — security model
12. `feat(mother): validate and log all toy grants at load time` — audit

## Verification Plan

Each phase has its own verification before proceeding.

Phase 1: `wasm-tools component wit` shows typed export
Phase 2: Integration test proves wac-graph composition + wasmtime load
Phase 3: All 6 children build as typed components
Phase 4: End-to-end pipeline: folder → parquet → catalog
Phase 5: Grant validation + handle-based children still work

Full suite at end:
```bash
cargo check --workspace -q
cargo test -q --workspace
cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2
cargo build -p patina-ai-child-dedup-filter --target wasm32-wasip2
cargo build -p patina-ai-child-content-extractor --target wasm32-wasip2
cargo build -p patina-ai-child-file-system-monitor --target wasm32-wasip2
cargo build -p patina-ai-child-record-writer --target wasm32-wasip2
cargo build -p patina-ai-child-lakehouse-catalog --target wasm32-wasip2
```

## Build Readiness

**Ready for Phase 1.** Prerequisites:
- wac-graph API proven (spike successful)
- wit-bindgen 0.41 works with typed exports (spike successful)
- wasm-tools compose validates composed components (spike successful)
- wasmtime loads composed components (spike successful)
- Same package instantiates multiple times (spike successful)

**Open questions to resolve during build:**
- Composition entry point (open question 1 in SPEC)
- State scoping for keyvalue in composition (open question 2)
- Exact wac-graph API for 6-child chain (proven for 2 and 3)
