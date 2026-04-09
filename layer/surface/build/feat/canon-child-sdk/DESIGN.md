# Design: canon-child-sdk

## Why This Design

Push-pure children use wit_bindgen directly today. This works but produces
duplication (error helpers, toy call patterns) and gives developers zero
guidance. patina-canon sits alongside wit_bindgen — it doesn't replace it.
Children still declare their world. The SDK provides shared types and
ergonomic toy access.

## Build Target

Single branch. Commits grouped: crate scaffold → toy helpers → config toy →
template → child migration → docs.

## Resolved Decisions

- patina-canon is a NEW crate, not a patina-sdk feature flag
- wit_bindgen::generate! stays in children (SDK doesn't wrap world declaration)
- Type re-exports use WIT-generated types, not hand-written duplicates
- Toy helpers are thin wrappers — one function call maps to one WIT call
- Config toy is new WIT (patina:config@0.1.0), minimal read-only interface

## Commits

1. `feat(sdk): scaffold patina-canon crate with type re-exports` — New sdk/patina-canon/ with Cargo.toml + src/lib.rs. Re-export patina:records types (RecordEnvelope, TransformResult, FileFound, FileWritten, CatalogEntry, RejectedRecord) via prelude module.

2. `feat(sdk): add outside toy helpers to patina-canon` — toys::log (info/warn/error), toys::keyvalue (open → Bucket with get/set/exists/drop, error mapping), toys::measure (counter/gauge). Each wraps raw WIT bindings with ergonomic Rust API.

3. `feat(wit): define patina:config@0.1.0 toy + Mother implementation` — New wit/toys/deps/patina-config.wit. Mother host impl in daemon.rs (reads from pando config HashMap). SDK adds toys::config::get(key).

4. `feat(sdk): create template-canon with cargo-generate` — sdk/template-canon/ with Cargo.toml, child.toml, wit/ structure, skeleton lib.rs using patina-canon prelude + toys.

5. `refactor(children): migrate 6 canon children to patina-canon` — Replace raw WIT toy calls with toys::* helpers. Remove duplicated keyvalue_error_to_string from dedup-filter and record-writer. Import types from patina_canon::prelude. Verify all 6 build to wasm32-wasip2.

6. `docs(sdk): mark patina-sdk as legacy, add decision tree` — Update patina-sdk README. Add decision tree to AGENTS.md or SDK README: canon (patina-canon) vs legacy service (patina-sdk child) vs grammar (patina-sdk pipeline).

## Direct Code Targets

### Commit 1: Crate scaffold
- New: `sdk/patina-canon/Cargo.toml`
- New: `sdk/patina-canon/src/lib.rs` (prelude module with type re-exports)
- New: `sdk/patina-canon/src/types.rs` (WIT-generated type wrappers)
- Update: root `Cargo.toml` workspace members

### Commit 2: Toy helpers
- New: `sdk/patina-canon/src/toys/mod.rs`
- New: `sdk/patina-canon/src/toys/log.rs` (~20 lines)
- New: `sdk/patina-canon/src/toys/keyvalue.rs` (~60 lines, Bucket struct + error mapping)
- New: `sdk/patina-canon/src/toys/measure.rs` (~15 lines)

### Commit 3: Config toy
- New: `wit/toys/deps/patina-config.wit` (~6 lines)
- Update: `src/commands/mother/daemon.rs` composed_bindings — already has config impl
- New: `sdk/patina-canon/src/toys/config.rs` (~10 lines)

### Commit 4: Template
- New: `sdk/template-canon/cargo-generate.toml`
- New: `sdk/template-canon/Cargo.toml`
- New: `sdk/template-canon/child.toml`
- New: `sdk/template-canon/wit/world.wit`
- New: `sdk/template-canon/wit/deps/` (patina:records types)
- New: `sdk/template-canon/src/lib.rs`

### Commit 5: Migrate children
- Update: `children/dedup-filter/Cargo.toml` — add patina-canon dependency
- Update: `children/dedup-filter/src/lib.rs` — use toys::keyvalue, remove local helper
- Update: `children/record-writer/Cargo.toml` + `src/lib.rs` — same
- Update: `children/schema-enforcer/src/lib.rs` — use toys::log, toys::measure
- Update: remaining 3 children similarly
- Rebuild all 6 to wasm32-wasip2

### Commit 6: Documentation
- Update: `sdk/patina-sdk/README.md` — add "Legacy Service Lane" header
- Update: `AGENTS.md` or new `sdk/README.md` — decision tree

## Verification Plan

Per commit: `cargo check --workspace`
After commit 5: `cargo build --target wasm32-wasip2` for all 6 children
After all: `cargo nextest run`
Template: `cargo generate` + build in /tmp

## Open Questions

None.
