# Design: patina-sdk-rebuild

## Why This Design

Push-pure children use wit_bindgen directly today. This works but produces
duplication (error helpers, toy call patterns) and gives developers zero
guidance. patina-sdk sits alongside wit_bindgen — it doesn't replace it.
Children still declare their world. The SDK provides shared types and
ergonomic toy access.

## Build Target

Single branch. Commits grouped: crate scaffold → toy helpers → config toy →
template → child migration → docs.

## Resolved Decisions

- patina-sdk takes the name. Old crate renamed to patina-sdk-legacy.
- Legacy children use Cargo `package` alias: `patina-sdk = { package = "patina-sdk-legacy", ... }` so Rust code keeps importing `patina_sdk::*` with no code changes.
- wit_bindgen::generate! stays in children (SDK doesn't wrap world declaration)
- Type re-exports use WIT-generated types, not hand-written duplicates
- Toy helpers are thin wrappers — one function call maps to one WIT call
- Config toy already exists (patina:config@0.1.0 from pando-execution-mvp). SDK wraps it — no new WIT or Mother code.

## Commits

0. `refactor(sdk): rename patina-sdk to patina-sdk-legacy` — Move sdk/patina-sdk/ to sdk/patina-sdk-legacy/. Update crate name in Cargo.toml. Update all 4 legacy children (belief-verifier, session-writer, spec-manager, doctor) to use `package = "patina-sdk-legacy"` alias so Rust imports stay as `patina_sdk::*`. Update workspace members in root Cargo.toml. Verify cargo check.

1. `feat(sdk): scaffold patina-sdk crate with type re-exports` — New sdk/patina-sdk/ with Cargo.toml + src/lib.rs. Re-export patina:records types (RecordEnvelope, TransformResult, FileFound, FileWritten, CatalogEntry, RejectedRecord) via prelude module.

2. `feat(sdk): add outside toy helpers to patina-sdk` — toys::log (info/warn/error), toys::keyvalue (open → Bucket with get/set/exists/drop, error mapping), toys::measure (counter/gauge). Each wraps raw WIT bindings with ergonomic Rust API.

3. `feat(sdk): wrap existing patina:config toy in SDK` — patina:config@0.1.0 WIT and Mother host impl already exist (pando-execution-mvp). SDK adds toys::config::get(key) wrapper. No new WIT or daemon code.

4. `feat(sdk): create child template with cargo-generate` — sdk/template/ with Cargo.toml, child.toml, wit/ structure, skeleton lib.rs using patina-sdk prelude + toys.

5. `refactor(children): migrate 6 children to patina-sdk` — Replace raw WIT toy calls with toys::* helpers. Remove duplicated keyvalue_error_to_string from dedup-filter, record-writer, and lakehouse-catalog. Import types from patina_sdk::prelude. Verify all 6 build to wasm32-wasip2.

6. `docs(sdk): update SDK README, add decision tree` — Update patina-sdk-legacy README to mark as legacy. New patina-sdk README for child developers. Add decision tree to AGENTS.md or SDK README: child (patina-sdk) vs legacy service child (patina-sdk-legacy) vs grammar pipeline (patina-sdk-legacy pipeline feature).

## Direct Code Targets

### Commit 1: Crate scaffold
- New: `sdk/patina-sdk/Cargo.toml`
- New: `sdk/patina-sdk/src/lib.rs` (prelude module with type re-exports)
- New: `sdk/patina-sdk/src/types.rs` (WIT-generated type wrappers)
- Update: root `Cargo.toml` workspace members

### Commit 2: Toy helpers
- New: `sdk/patina-sdk/src/toys/mod.rs`
- New: `sdk/patina-sdk/src/toys/log.rs` (~20 lines)
- New: `sdk/patina-sdk/src/toys/keyvalue.rs` (~60 lines, Bucket struct + error mapping)
- New: `sdk/patina-sdk/src/toys/measure.rs` (~15 lines)

### Commit 3: Config toy wrapper
- New: `sdk/patina-sdk/src/toys/config.rs` (~10 lines, wraps existing patina:config@0.1.0)
- No WIT or daemon changes — both already exist from pando-execution-mvp

### Commit 4: Template
- New: `sdk/template/cargo-generate.toml`
- New: `sdk/template/Cargo.toml`
- New: `sdk/template/child.toml`
- New: `sdk/template/wit/world.wit`
- New: `sdk/template/wit/deps/` (patina:records types)
- New: `sdk/template/src/lib.rs`

### Commit 5: Migrate children
- Update: `children/dedup-filter/Cargo.toml` — add patina-sdk dependency
- Update: `children/dedup-filter/src/lib.rs` — use toys::keyvalue, remove keyvalue_error_to_string
- Update: `children/record-writer/Cargo.toml` + `src/lib.rs` — same (remove keyvalue_error_to_string)
- Update: `children/lakehouse-catalog/Cargo.toml` + `src/lib.rs` — same (remove keyvalue_error_to_string)
- Update: `children/schema-enforcer/src/lib.rs` — use toys::log, toys::measure
- Update: `children/file-system-monitor/src/lib.rs` — use toys::log
- Update: `children/content-extractor/src/lib.rs` — use toys::log
- Rebuild all 6 to wasm32-wasip2

### Commit 6: Documentation
- Update: `sdk/patina-sdk-legacy/README.md` — mark as legacy
- New: `sdk/patina-sdk/README.md` — child developer guide
- Update: `AGENTS.md` or new `sdk/README.md` — decision tree

## Verification Plan

Per commit: `cargo check --workspace`
After commit 5: `cargo build --target wasm32-wasip2` for all 6 children
After all: `cargo nextest run`
Template: `cargo generate` + build in /tmp

## Open Questions

None.
