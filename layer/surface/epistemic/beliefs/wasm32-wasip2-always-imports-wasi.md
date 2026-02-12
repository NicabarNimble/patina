---
type: belief
id: wasm32-wasip2-always-imports-wasi
persona: architect
facets: [wasm, plugin-system, wasmtime]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-12
revised: 2026-02-12
---

# wasm32-wasip2-always-imports-wasi

wasm32-wasip2 components always import basic WASI interfaces (cli, filesystem, streams, clocks) via the std library, even for pure-computation code — any wasmtime host must provide wasmtime-wasi regardless of plugin capabilities

## Statement

wasm32-wasip2 components always import basic WASI interfaces (cli, filesystem, streams, clocks) via the std library, even for pure-computation code — any wasmtime host must provide wasmtime-wasi regardless of plugin capabilities

## Evidence

- [[session-20260212-061458]]: [[commit-5001ecb2]] - models child with zero WASI usage still imported 10 WASI interfaces in wasm-tools component wit output (weight: 0.95)

## Supports

- [[de-risk-runtime-with-simplest-payload]] — understanding WASI baseline is part of de-risking

## Attacks

- Invalidates assumption in [[plugin-system]] spec that Phase 1 needs no wasmtime-wasi

## Attacked-By

- Future: `#![no_std]` plugins could avoid WASI imports but lose std library access

## Applied-In

- `src/plugin/internal.rs` — HostState includes WasiCtx + ResourceTable, `add_to_linker_sync()` in PluginEngine::new()
- `Cargo.toml` — `wasmtime-wasi = { version = "=41", features = ["p2"] }` added for Phase 1

## Revision Log

- 2026-02-12: Created — metrics computed by `patina scrape`
