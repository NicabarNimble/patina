---
type: belief
id: wit-defines-pipe-contract-not-runtime
persona: architect
facets: [architecture, wit, pipes, contracts]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-05
revised: 2026-03-05
---

# wit-defines-pipe-contract-not-runtime

WIT defines the universal type contract for pipes (fact shapes, capability declarations, fetch interface) without mandating WASM as runtime — for plugins WASM is right (compute sandbox), for pipes native processes are right (I/O-bound, streaming), WIT is the interface definition for both

## Statement

WIT defines the universal type contract for pipes (fact shapes, capability declarations, fetch interface) without mandating WASM as runtime — for plugins WASM is right (compute sandbox), for pipes native processes are right (I/O-bound, streaming), WIT is the interface definition for both

## Evidence

- [[session-20260305-224446]]: [[session-20260305-224446]] - Strengthens [[wit-is-contract-wasm-is-one-runtime]]: WIT defines pipe interface (get-capabilities, check-health, fetch) whether implemented as Rust binary, Python script, or Cloudflare Worker. WASM is one runtime for plugins, native process is the runtime for pipes (weight: 0.9)

## Supports

- [[wit-is-contract-wasm-is-one-runtime]] — directly strengthens: pipes are the concrete case where WIT contract + non-WASM runtime is the right choice
- [[pipes-are-processes-not-wasm]] — WIT provides type safety for process-based pipes without requiring WASM
- [[pipe-protocol-is-transport-agnostic]] — WIT defines message types independent of transport

## Attacks

- Attacks "WIT is only useful with WASM" — WIT's value is type definitions and interface contracts, orthogonal to runtime

## Attacked-By

- "Just use JSON schema / protobuf instead of WIT" — WIT adds a dependency on the component-model toolchain. Counter: WIT is already in the codebase (4 world definitions), generates types for both WASM plugins and native code, and defines capabilities (not just data shapes)

## Applied-In

- `wit/mother-child/mother-child.wit` — existing WIT world for WASM plugins
- [[spec-pipe-architecture]] — proposed `wit/pipe/pipe.wit` for pipe interface (get-capabilities, check-health, fetch)

## Revision Log

- 2026-03-05: Created — metrics computed by `patina scrape`
