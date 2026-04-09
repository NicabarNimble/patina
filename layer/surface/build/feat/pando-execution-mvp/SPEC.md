---
type: feat
id: pando-execution-mvp
status: draft
created: 2026-04-09
sessions:
  origin: 20260409-090238-892748000
beliefs:
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[wasi-is-foundation-not-option]]"
  - "[[projects-are-sovereign-mother-coordinates]]"
related:
  - refactor/child-typed-composition
  - feat/voice-lake-mvp1
  - feat/child-construction-canon
  - resources/pandos/folder-text-to-parquet/pando.toml
  - src/commands/mother/daemon.rs
  - mother/src/pando.rs
exit_criteria:
  - id: pe1-worlds-complete
    text: "All 6 children's world.wit files declare upstream toy imports so wac-graph can wire the full chain: content-extractor imports records/source, schema-enforcer imports records/extract, lakehouse-catalog imports records/write."
    checked: false
  - id: pe2-pando-typed-wiring
    text: "folder-text-to-parquet/pando.toml uses typed wiring format ([[composition.wiring]] with from/to/toy) and declares composition.entry. Legacy string wiring removed."
    checked: false
  - id: pe3-compose-and-load
    text: "Mother builds a composed .wasm from the 6 canon children via wac-graph encode() and loads it in wasmtime's component API."
    checked: false
  - id: pe4-outside-toys-linked
    text: "Mother links outside toys (logging, keyvalue, measure) to the composed component using wasmtime component linker. Bindgen strategy is explicit (component::bindgen! for composed world)."
    checked: false
  - id: pe5-entry-point
    text: "Mother calls the composition's entry point (records/source.scan) with a folder path. Data flows through all 6 children."
    checked: false
  - id: pe6-output-exists
    text: "After execution, output files exist at a Mother-controlled path (not hardcoded /tmp/patina/). record-writer uses filesystem preopen, not hardcoded path."
    checked: false
  - id: pe7-handle-children-work
    text: "Handle-based service children (belief-verifier, session-writer, spec-manager, doctor) continue working unchanged alongside the composed pando."
    checked: false
  - id: pe8-proof
    text: "Integration test: ingest a temp folder with 3 .txt files through folder-text-to-parquet, verify output parquet contains 3 records (no dedup collisions), verify catalog has entry. cargo check + cargo nextest run pass."
    checked: false
---
# feat: Pando Execution MVP

## Problem

All 6 canon children are compiled to typed WIT components. The pando manifest
exists. wac-graph composition validation works. But nobody has ever loaded the
composed component, linked outside toys, called the entry point, or run data
through the pipeline.

Additionally, an audit reveals the children's worlds are INCOMPLETE for full
composition — 4 of 6 are missing upstream toy imports. And the shipped pando.toml
still uses legacy string wiring with no typed wiring or entry point.

The gap is wider than "just load and call." It's: fix the worlds, update the
pando, compose, load, link, call, and get output.

## What's Wrong Today

### 1. Incomplete child worlds

The child-typed-composition SPEC describes worlds with upstream imports, but
the actual world.wit files are missing them:

| Child | Missing import | Has |
|---|---|---|
| content-extractor | `patina:records/source@0.1.0` | only logging |
| schema-enforcer | `patina:records/extract@0.1.0` | only logging + measure |
| lakehouse-catalog | `patina:records/write@0.1.0` | only logging + keyvalue |

dedup-filter and record-writer DO have upstream imports (both import
`patina:records/transform@0.1.0`). file-system-monitor is the source child
and correctly has no upstream import.

Without these imports, wac-graph cannot wire the full chain. Composition
breaks at 3 points.

### 2. Pando uses legacy wiring

`resources/pandos/folder-text-to-parquet/pando.toml` uses legacy string
wiring (`"file-system-monitor.file.found -> content-extractor"`) with no
`[composition].entry` and no typed wiring rules. The typed composition
validation code in daemon.rs returns early when no typed rules are found.

### 3. Package name mismatch in docs

The actual WIT package is `patina:records@0.1.0` (plural). The
child-typed-composition SPEC says `patina:record@0.1.0` (singular).
This spec uses the ACTUAL name: `patina:records`.

### 4. record-writer hardcodes output path

`children/record-writer/src/lib.rs:168` hardcodes `/tmp/patina/records-*.parquet`.
Must be changed to use filesystem preopens for Mother-controlled output.

### 5. Linker bindgen is world-coupled

`src/child/internal/child.rs` uses `bindgen!({ world: "child" })`. The link_*()
methods are generic, but the generated bindings are coupled to the handle-based
child world. Composed components need their own bindgen — must use
`component::bindgen!` or equivalent for the composed world.

## Goal

Make folder-text-to-parquet run end-to-end: folder in, parquet + catalog out.
Fix the prerequisite gaps (worlds, pando, output paths), then compose, load,
link, call.

## Non-Goals

- Pando config injection (`[config]` section) — that's voice-lake-mvp1.
- Voice/namespace scoping — that's voice-lake-mvp1.
- Federation query integration — that's voice-lake-mvp1.
- Per-child metrics via patina:measure — link the toy but don't require
  metrics verification.
- Security audit logging (grant decisions) — important but not blocking.
- child.toml inside toy grant validation — future hardening.
- Multi-instance reuse — not needed for this pando.

## Implementation Order

1. **Fix 3 child worlds** — Add upstream imports to content-extractor,
   schema-enforcer, lakehouse-catalog. Rebuild .wasm for all 3.
2. **Fix record-writer output** — Replace hardcoded `/tmp/patina/` with
   filesystem preopen path. Rebuild .wasm.
3. **Update pando.toml** — Replace legacy string wiring with typed wiring
   (`[[composition.wiring]]` format). Add `[composition].entry`.
4. **Bindgen for composed world** — Use `wasmtime::component::bindgen!`
   targeting the composed component's exported interface.
5. **Compose and load** — wac-graph encode() → Component::new().
6. **Link outside toys** — Add logging, keyvalue, measure implementations
   to the composed component's linker.
7. **Call entry point** — source.scan(folder), collect results.
8. **Dispatch enum** — HandleBased vs Composed selection in Mother registry.
9. **Integration test** — temp folder with 3 .txt files, verify end-to-end.

## Resolved Decisions

- **Package name**: `patina:records@0.1.0` (plural) — matches actual WIT.
- **Bindgen strategy**: `wasmtime::component::bindgen!` macro against the
  composed component's WIT. This gives typed Rust entry points. Not manual
  `func_new_typed` — too fragile for 6-child composition surface.
- **Output paths**: record-writer uses WASI filesystem preopens. Mother sets
  the preopen directory. No more `/tmp/patina/` hardcode.
- **Linker**: New linker setup for composed world alongside existing child
  world linker. They don't share bindgen — different worlds, different types.
- **Dispatch**: Pando with `[composition]` typed wiring → composed path.
  Pando without (or with only legacy wiring) → existing handle-based path.
- **Entry point**: The composition exports `patina:records/source@0.1.0`.
  Mother calls `scan(folder)`. Everything else flows internally via wac-graph
  wiring.

## Verification

```bash
cargo check --workspace -q
cargo nextest run

# Verify children rebuild with updated worlds:
cargo build -p patina-ai-child-content-extractor --target wasm32-wasip2
cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2
cargo build -p patina-ai-child-lakehouse-catalog --target wasm32-wasip2
cargo build -p patina-ai-child-record-writer --target wasm32-wasip2

# Integration test:
cargo test --test pando_execution -- folder_text_to_parquet
# Creates temp dir with 3 .txt files
# Runs composed pando
# Asserts: 3 records in parquet output
# Asserts: catalog entry exists
# Asserts: output NOT in /tmp/patina/ (Mother-controlled path)
```

## Build Readiness

All prerequisites exist except the gaps documented above. The work is:
fix worlds (3 children), fix output path (1 child), update pando manifest,
bindgen + compose + load + link + call, integration test.
