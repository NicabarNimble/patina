# Design: refactor: mother-child-toy-beliefs layout and debt cleanup

## Why This Design

Patina now has a stable doctrine: beliefs are the system center; Mother owns
authority and continuity; Children execute bounded workflows; Toys are granted
capabilities. The repository and module layout should make this doctrine obvious
without code archaeology.

This design prioritizes:

- **Beliefs-first core**: internal components should improve beliefs, not merely consume them.
- **Explicit ownership**: Mother/Child/Toy boundaries are physically represented in source tree.
- **Debt removal**: legacy native DuckLake path is removed rather than quarantined.
- **Core-tools clarity**: spec/scrape-code internals are grouped as core tooling.

## Build Target

### Canonical Source Layout (target)

- `src/beliefs/**` - belief engine/query/mutation/verification/materialization
- `src/mother/**` - authority, grant policy, orchestration, continuity
- `src/child/**` - child runtime contracts/registry/lifecycle glue
- `src/toys/**` - toy host implementations + toy catalog
- `src/core_tools/**` - core system tools (spec lifecycle, scrape-code internals)

### Canonical Child Project Layout (target)

- `child/ducklake/**` (knowledge-child app)
- `child/belief-verifier/**` (knowledge-child app)
- `child/github-connector/**` (until explicitly retired by separate spec)
- no `legacy/ducklake*` retention

### Layer Output Contract (preserved)

- `layer/core/**`
- `layer/surface/**`
- `layer/dust/**`

## Resolved Decisions

- Layout doctrine follows the same trust model as runtime doctrine.
- DuckLake native legacy path is deleted when cutover gates are met (no legacy folder fallback).
- `plugins/*` is not canonical for doctrine runtime surfaces.
- `spec` and `scrape-code` are treated as core tools, not incidental command internals.

## Commits

1. `refactor: establish beliefs/mother/child/toys/core_tools roots` — introduce canonical module roots and compile plumbing.
2. `refactor: move toy host logic into src/toys` — centralize capabilities and add toy catalog.
3. `refactor: move child runtime glue into src/child` — isolate child contracts/registry/lifecycle.
4. `refactor: move belief internals into src/beliefs` — make beliefs-first center explicit.
5. `refactor: extract spec and scrape-code internals to src/core_tools` — clarify core tooling ownership.
6. `refactor: rename child projects to child/* and remove ducklake native legacy` — finalize child layout and remove dead ducklake path.
7. `chore: add CI drift guards for doctrine layout` — prevent regression.

## Direct Code Targets

### Phase A - Root layout and wiring

- `src/lib.rs` - add and re-export canonical roots (`beliefs`, `mother`, `child`, `toys`, `core_tools`).
- `Cargo.toml` - update workspace members once `child/*` move lands.

### Phase B - Toys centralization

- `src/plugin/internal/knowledge_child.rs` -> split toy host implementations into `src/toys/*` modules.
- `src/mother/lake_host.rs` -> move under `src/toys/lake/*` (or wrap with toy module facade).
- `src/toys/catalog.rs` (new) - registry: toy id, owning module, grant requirements, metric names.

### Phase C - Child runtime boundary

- `src/plugin/internal/mod.rs` and child-loading support -> `src/child/runtime/*` + `src/child/registry/*`.
- `src/broker/mod.rs` child loading helpers -> call through `src/child` interfaces.

### Phase D - Beliefs core boundary

- belief query/mutation/materialization internals currently spread across scrape/mother/retrieval flows -> `src/beliefs/*` modules with stable public API back into commands.

### Phase E - Core tools extraction

- `src/commands/spec/internal/*` -> `src/core_tools/spec/*`.
- scrape-code internals (`src/commands/scrape/**` code-focused pieces) -> `src/core_tools/scrape_code/*`.
- command modules become thin routing adapters over `core_tools` APIs.

### Phase F - Child project paths + legacy removal

- `children/ducklake-wasm/**` -> `child/ducklake/**`
- `children/belief-verifier/**` -> `child/belief-verifier/**`
- `children/github-connector/**` -> `child/github-connector/**`
- remove `children/ducklake/**` native legacy path (and references) once parity criteria are met.
- update all path references in runtime/tests/spec docs.

## Verification Plan

Run after each phase (not only at the end):

- `cargo check --workspace`
- `cargo build --target wasm32-wasip2 -p patina-ai-child-ducklake`
- `cargo build --target wasm32-wasip2 -p patina-ai-child-belief-verifier`
- `cargo test -q -p patina-ai -- src/plugin/internal/tests.rs`
- `cargo test -q -p patina-ai -- src/commands/spec/internal/tests.rs`
- `cargo test -q -p patina-ai -- src/commands/scrape/internal/tests.rs`
- `bash resources/scripts/check-single-sdk-surface.sh`
- `bash resources/scripts/check-crate-names.sh`
- `patina spec check mother-child-toy-beliefs-layout --json`

Drift probes:

- `rg "children/ducklake-wasm|children/ducklake" src child children sdk tests Cargo.toml`
- `rg "src/(beliefs|mother|child|toys|core_tools)" layer/surface/build/refactor/mother-child-toy-beliefs-layout/DESIGN.md`

## Build Readiness

- [ ] Canonical root modules compile with no behavioral regression.
- [ ] Toy catalog exists and all toys are listed once.
- [ ] Spec/scrape-code internals routed through `core_tools` APIs.
- [ ] Child project paths moved to `child/*`.
- [ ] Native legacy ducklake removed.
- [ ] CI drift guards merged.

## Open Questions

- Should `core_tools` remain under `src/` or become a dedicated workspace crate in a follow-up?
- Should `child/` path migration happen in this spec or be split into a short dependent spec to reduce blast radius?
