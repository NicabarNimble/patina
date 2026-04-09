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
  - id: pe1-compose-and-load
    text: "Mother builds a composed .wasm from the 6 canon children via wac-graph encode() and loads it in wasmtime's component API."
    checked: false
  - id: pe2-outside-toys-linked
    text: "Mother links outside toys (logging, keyvalue, filesystem, measure, sql) to the composed component. Children's outside toy imports are satisfied."
    checked: false
  - id: pe3-entry-point
    text: "Mother calls the composition's entry point (source.scan) with a folder path. Data flows through all 6 children: scan → extract → transform → transform → write → catalog."
    checked: false
  - id: pe4-output-exists
    text: "After execution, parquet files and catalog entries exist on disk at a Mother-controlled path."
    checked: false
  - id: pe5-handle-children-work
    text: "Handle-based service children (belief-verifier, session-writer, spec-manager, doctor) continue working unchanged alongside the composed pando."
    checked: false
  - id: pe6-proof
    text: "Integration test: ingest a test folder with known files through folder-text-to-parquet, verify parquet output contains expected record count. cargo check + cargo nextest run pass."
    checked: false
---
# feat: Pando Execution MVP

## Problem

All 6 canon children are compiled to typed WIT components. The pando manifest
exists. wac-graph composition validation works — Mother can build the graph,
check wiring, and encode composed bytes. But nobody has ever loaded the
composed component in wasmtime, linked outside toys, called the entry point,
or run data through the pipeline.

The gap: composition VALIDATION works. Composition EXECUTION does not.

voice-lake-mvp1 and multiproject-belief-share depend on the pipeline actually
running. This spec closes the execution gap for folder-text-to-parquet — our
only pando.

## Goal

Make folder-text-to-parquet run end-to-end: folder in, parquet + catalog out.
Mother composes 6 typed children via wac-graph, loads the result in wasmtime,
links outside toys, calls the entry point, and collects output.

## Status

Draft.

## Non-Goals

- Pando config injection (`[config]` section) — that's voice-lake-mvp1.
- Voice/namespace scoping — that's voice-lake-mvp1.
- Federation query integration — that's voice-lake-mvp1.
- Per-child metrics via patina:measure — desirable but not blocking execution.
- Security audit logging (grant decisions) — important but not blocking execution.
- child.toml inside toy grant validation — Mother currently validates composition
  graph shape; formal grant enforcement is a hardening step.
- Multi-instance reuse (same child, multiple instances) — proven in spike, not
  needed for this pando.
- Converting service children to typed — they stay handle-based.

## What Exists Today

| Component | Status |
|---|---|
| 6 canon children compiled to .wasm | Done — `~/.patina/children/*.wasm` |
| Pando manifest parser | Done — `mother/src/pando.rs` |
| Typed wiring parser (`PandoTypedWiring`) | Done |
| wac-graph validation (`validate_typed_composition`) | Done — `daemon.rs:188-318` |
| wac-graph `encode()` produces composed bytes | Done — `daemon.rs:309` |
| Load composed .wasm in wasmtime component API | **NOT DONE** |
| Link outside toys to composed component | **NOT DONE** |
| Call entry point (source.scan) | **NOT DONE** |
| Dispatch enum (HandleBased vs Composed) | **NOT DONE** |
| Integration test with real data | **NOT DONE** |

## What This Spec Builds

1. **Composed component loading** — After wac-graph produces encoded bytes,
   load them as a wasmtime Component. Generate or use bindgen for the
   composed world's exports.

2. **Outside toy linking** — The composed component imports outside toys
   (logging, keyvalue, filesystem, measure, sql) that were originally
   imported by individual children. Mother's existing link_*() functions
   satisfy these imports on the wasmtime Linker. The merged import set of
   the composed component should match the union of all children's outside
   toy imports.

3. **Entry point invocation** — The composition exports
   `patina:record/source@0.1.0` (from file-system-monitor). Mother calls
   `source.scan(folder)` on the composed component. Data flows through
   the pipeline internally.

4. **Output collection** — record-writer produces parquet files.
   lakehouse-catalog registers them. Both use filesystem and keyvalue
   toys. Mother provides these with paths pointing to a controlled
   output directory.

5. **Dispatch path** — Mother needs to know whether to use the existing
   handle-based dispatch or the new composed dispatch. Pando manifest
   already has `[composition]` section; presence of typed wiring
   selects the composed path.

## Implementation Order

1. Composed component loading — wasmtime Component::new from wac-graph bytes
2. Bindgen for composed world — generate or manual typed interface for entry point
3. Outside toy linker setup — reuse existing link_*() with composed component
4. Entry point call — source.scan(folder) invocation
5. Output path wiring — filesystem preopens for record-writer and lakehouse-catalog
6. Dispatch enum — HandleBased vs Composed selection in registry
7. Integration test — real folder, real execution, verify parquet output

## Resolved Decisions

- **wac-graph at runtime, not build time** — Mother composes. Children stay
  individual .wasm files. This is the resolved architecture from
  child-typed-composition.
- **Entry point is source.scan** — The composition's outermost export is
  file-system-monitor's source toy. Mother calls scan(folder), everything
  else flows internally.
- **Outside toys are the union** — The composed component imports the union
  of all 6 children's outside toys. Mother satisfies all of them.
- **Handle-based children unaffected** — Existing dispatch path stays. New
  path added alongside.
- **Output to Mother-controlled directory** — Mother decides where parquet
  and catalog land. For this spec, a reasonable default path. voice-lake-mvp1
  adds voice-scoped namespacing on top.

## Verification

```bash
cargo check --workspace -q
cargo nextest run
# Specific integration test:
cargo test --test pando_execution -- folder_text_to_parquet
# Expect: test creates temp folder with .txt files, runs composed pando,
# verifies parquet files exist, verifies catalog entries, verifies record count.
```

## Build Readiness

All prerequisites exist:
- 6 children compiled to .wasm
- wac-graph dependency in Cargo.toml
- Composition graph builds and encodes
- wasmtime component API available
- Existing outside toy linker code in daemon.rs

The work is: load the encoded bytes, link toys, call entry point, wire output.
