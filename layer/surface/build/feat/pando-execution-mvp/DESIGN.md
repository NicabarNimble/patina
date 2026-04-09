# Design: pando-execution-mvp

## Why This Design

Fix 2 (dual surface with pando adapters) was chosen over:
- Fix 1 (parameterless pull) — loses standalone testability
- Fix 3 (Mother orchestrates type boundaries) — Mother is inside the data flow
- Two-worlds-per-child — couples children to composition concerns

Pando adapters keep children push-pure and composition-ignorant. Adapters are
thin glue components that handle the chain. Mother controls outside, adapters
control inside.

## Build Target

Single branch (`patina`). Commits grouped by concern.

## Resolved Decisions

- **patina:pando** for composed interfaces (not patina:pipeline — reserved for grammar engine)
- **Children stay push-pure** — no upstream composed imports, ever
- **Adapters are glue-only** — pull upstream, call push child, return. Zero logic.
- **Source adapter is special** — no upstream pando import, config injection for folder path
- **Explicit WAC instance aliases** for repeated transform stages
- **Bindgen**: `wasmtime::component::bindgen!` against composed component's WIT
- **Folder injection**: config capability (preferred), WASI env as fallback
- **Package name**: `patina:records@0.1.0` (plural, matches actual WIT)

## Commits

1. `refactor(children): make dedup-filter and record-writer push-pure` — Remove upstream `patina:records/transform` import from both children's world.wit. Update implementations: dedup-filter processes records directly (no upstream call), record-writer writes records directly. Rebuild both .wasm.

2. `fix(record-writer): use filesystem preopen instead of /tmp/patina/` — Replace hardcoded PathBuf::from("/tmp/patina/...") with WASI preopen directory lookup. Rebuild .wasm.

3. `feat(wit): create patina:pando@0.1.0 composed interfaces` — New wit/pando/ directory with run() interfaces per stage (source, extract, transform, write, catalog). Each returns stage-specific type from patina:records. No domain types in this package.

4. `feat(adapters): build 6 pando adapter components` — Each adapter: imports upstream pando/<prev-stage> + push child records/<stage>, exports pando/<stage>::run(). Source adapter imports config + records/source only. Each is ~10-15 lines of Rust. Build all 6 to .wasm.

5. `refactor(pando): update folder-text-to-parquet to typed wiring with 12 components` — Replace legacy string wiring with [[composition.wiring]] typed rules for 12 components (6 push + 6 adapters). Add composition.entry pointing to lc-pando (pando/catalog). Add explicit instance aliases for schema-transform and dedup-transform.

6. `feat(mother): compose 12 components via wac-graph and load in wasmtime` — After wac-graph encode(), Component::new(). Bindgen for composed world. Linker with outside toys. Call pando/catalog::run(). Add LoadedComponent::Composed dispatch path.

7. `test(pando): parity tests per stage` — For each of 6 stages: push(fixture) vs composed.run() with mocked upstream returning same fixture. Assert identical output.

8. `test(pando): e2e folder-text-to-parquet integration` — 3 unique .txt files, Mother calls pando/catalog::run(), assert 3 records, 1 parquet at controlled path, 1 catalog entry.

9. `chore: record artifact size baselines` — Track .wasm size per child and adapter before/after.

## Direct Code Targets

### Commit 1: Push-pure children
- `children/dedup-filter/wit/world.wit:7` — remove `import patina:records/transform@0.1.0;`
- `children/dedup-filter/src/lib.rs:21` — remove `patina::records::transform::transform(&records)?` call, process records directly
- `children/record-writer/wit/world.wit:7` — remove `import patina:records/transform@0.1.0;`
- `children/record-writer/src/lib.rs:165` — remove upstream transform call, process records directly

### Commit 2: record-writer output
- `children/record-writer/src/lib.rs:168-171` — replace `/tmp/patina/` with preopen path

### Commit 3: patina:pando WIT
- New: `wit/pando/pando.wit` — package declaration + 5 interfaces (source, extract, transform, write, catalog), each with `run() → result<T, string>`, using types from patina:records

### Commit 4: Adapter components
- New: `adapters/fsm-pando/` — source adapter (config + records/source → pando/source)
- New: `adapters/ce-pando/` — extract adapter (pando/source + records/extract → pando/extract)
- New: `adapters/se-pando/` — transform adapter (pando/extract + records/transform → pando/transform)
- New: `adapters/df-pando/` — transform adapter (pando/transform + records/transform → pando/transform)
- New: `adapters/rw-pando/` — write adapter (pando/transform + records/write → pando/write)
- New: `adapters/lc-pando/` — catalog adapter (pando/write + records/catalog → pando/catalog)

### Commit 5: Pando manifest
- `resources/pandos/folder-text-to-parquet/pando.toml` — full rewrite with 12 components + typed wiring

### Commit 6: Mother composition execution
- `src/commands/mother/daemon.rs` — compose path: wac-graph encode → Component::new → linker → call
- New bindgen section for composed world
- Registry: LoadedComponent enum

### Commits 7-8: Tests
- New: `tests/pando_parity.rs`
- New: `tests/pando_execution.rs`

## Verification Plan

Per commit: `cargo check`
After all: `cargo nextest run`
Parity: per-stage push vs composed equivalence
E2E: single pando/catalog::run() drives full chain
Regression: `patina spec list` (handle-based child)
Size: wasm-tools print-size per artifact

## Open Questions

None. All resolved.

## Resolved (formerly open)

1. **Config capability**: New minimal `patina:config` toy (read-only key/value).
   Not raw env as primary. Env is local-dev fallback only. Matches voice/pando
   `[config]` direction. Explicit authority, testable.

2. **Adapter crate structure**: One crate per stage (6 crates) + optional
   shared `adapters-common` for any shared helper types. Cleaner WIT/world
   isolation, simpler builds, simpler size attribution.

3. **Adapter artifact location**: Separate namespace from children.
   Children in `~/.patina/children/`, adapters in `~/.patina/pando-adapters/`.
   Keeps "child" semantics clean, avoids muddying registries/lifecycle.
