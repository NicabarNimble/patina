---
type: feat
id: watch-null-sink-child
status: active
created: 2026-04-13
sessions:
  origin: 20260413-200000-000000000
beliefs:
  - "[[wasi-is-foundation-not-option]]"
  - "[[children-have-agency-toys-are-capabilities]]"
related:
  - children/watch-null-sink/
  - children/folder-watch-actor/
  - wit/pando/deps/
exit_criteria:
  - id: wnsc1-typed-sink-export
    text: "New child exports `patina:watch/events@0.1.0` as a typed sink contract."
    checked: true
  - id: wnsc2-ephemeral-behavior
    text: "Sink performs no durable writes (no keyvalue/sql/filesystem persistence) and drops payloads after observability emit."
    checked: true
  - id: wnsc3-minimal-toys
    text: "Child manifest grants only `logging` + `measure` toys."
    checked: true
  - id: wnsc4-build-proof
    text: "Child compiles to wasm32-wasip2 and WIT inspection shows `export patina:watch/events@0.1.0`."
    checked: true
---
# feat: watch null sink child

> Add an ephemeral typed sink child so watch/event connections can be tested without persisting data.

## Problem

We need a strict WIT-native way to test child-to-child emission paths where downstream should intentionally discard data.

## Goal

Provide a reusable `watch-null-sink` child that accepts typed watch events and does:
- measure counter increment
- log/print signal
- no storage side effects

## Non-Goals

- No Mother business logic for watch domain.
- No persistence.
- No conversion to records domain in this child.

## Verification

```bash
cargo build --manifest-path children/watch-null-sink/Cargo.toml --target wasm32-wasip2
wasm-tools component wit children/watch-null-sink/target/wasm32-wasip2/debug/patina_ai_child_watch_null_sink.wasm
```

Expected WIT export:
- `export patina:watch/events@0.1.0`
