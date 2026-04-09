# Design: pando-execution-mvp

## Why This Design

Composition validation works. Execution doesn't. This spec closes the gap by
fixing 4 prerequisite issues (incomplete worlds, legacy pando wiring, hardcoded
output, missing bindgen) then wiring the execution path.

## Build Target

Single branch (`patina`). Commits grouped by concern: child world fixes,
pando manifest update, Mother composition execution, integration test.

## Resolved Decisions

- Bindgen: `wasmtime::component::bindgen!` against composed world — typed entry
- Linker: New setup for composed world, separate from handle-based child linker
- Output: WASI filesystem preopens, Mother sets directory
- Dispatch: typed wiring presence in pando selects composed path
- Package name: `patina:records@0.1.0` (plural, matches actual WIT)

## Commits

1. `fix(children): add upstream toy imports to content-extractor, schema-enforcer, lakehouse-catalog` — Update 3 world.wit files to import upstream toys per child-typed-composition SPEC. Rebuild .wasm. content-extractor imports records/source, schema-enforcer imports records/extract, lakehouse-catalog imports records/write.

2. `fix(record-writer): use filesystem preopen instead of hardcoded /tmp/patina/` — Replace PathBuf::from("/tmp/patina/...") with WASI preopen directory lookup. Rebuild .wasm.

3. `refactor(pando): replace legacy string wiring with typed wiring in folder-text-to-parquet` — Update pando.toml: replace `[composition].wiring` string list with `[[composition.wiring]]` typed rules (from/to/toy). Add `[composition].entry = { child = "file-system-monitor", toy = "patina:records/source" }`.

4. `feat(mother): compose and load pando via wac-graph + wasmtime` — Add component::bindgen! for composed world. After wac-graph encode(), load Component::new(). Set up linker with outside toy implementations. Call source.scan(folder). Add LoadedComponent::Composed dispatch path alongside existing HandleBased.

5. `test(pando): integration test for folder-text-to-parquet end-to-end` — Create temp dir with 3 .txt files, run composed pando, assert 3 records in parquet, assert catalog entry, assert output in Mother-controlled path.

## Direct Code Targets

### Commit 1: Child world fixes
- `children/content-extractor/wit/world.wit` — add `import patina:records/source@0.1.0;`
- `children/schema-enforcer/wit/world.wit` — add `import patina:records/extract@0.1.0;`
- `children/lakehouse-catalog/wit/world.wit` — add `import patina:records/write@0.1.0;`
- `children/content-extractor/src/lib.rs` — update to call upstream source.scan or accept data via import
- `children/schema-enforcer/src/lib.rs` — update to call upstream extract.extract or accept via import
- `children/lakehouse-catalog/src/lib.rs` — update to call upstream write.write or accept via import

### Commit 2: record-writer output path
- `children/record-writer/src/lib.rs:168` — replace hardcoded `/tmp/patina/` with WASI preopen directory

### Commit 3: Pando manifest
- `resources/pandos/folder-text-to-parquet/pando.toml` — replace entire `[composition]` section

### Commit 4: Mother composition execution
- `src/commands/mother/daemon.rs` — after validate_typed_composition, add:
  load composed component, bindgen, linker setup, entry point call
- New file or section: composed component bindgen (component::bindgen! macro)
- New file or section: composed linker setup (logging, keyvalue, measure, filesystem)
- Registry: LoadedComponent enum (HandleBased | Composed)

### Commit 5: Integration test
- `tests/pando_execution.rs` — new test file

## Verification Plan

After commits 1-2: `cargo build --target wasm32-wasip2` for all 4 updated children
After commit 3: `cargo test` pando manifest parsing tests
After commit 4: `cargo check --workspace`
After commit 5: `cargo nextest run`

## Build Readiness

Prerequisites exist. Gaps documented and scoped.

## Open Questions

1. **Upstream import pattern** — When content-extractor imports records/source,
   does it CALL source.scan() itself, or does wac-graph wire the data flow
   implicitly? In BA component model, importing an interface means the
   component can call it. In composition, Mother wires the export of one
   component to the import of another. So content-extractor would call
   `source::scan()` (its import), which at runtime is satisfied by
   file-system-monitor's export. Need to verify this is how wac-graph works.

2. **Composed world shape** — The composed component's outer world is the
   union of all unresolved imports (outside toys) plus the entry export
   (records/source from file-system-monitor). Need to verify wac-graph
   produces this shape and that bindgen can target it.

3. **Filesystem preopen scoping** — If multiple children need filesystem
   (file-system-monitor for input, record-writer for output), do they get
   the same preopen or separate ones? In composition, shared-nothing means
   separate stores per instance — verify with wasmtime.
