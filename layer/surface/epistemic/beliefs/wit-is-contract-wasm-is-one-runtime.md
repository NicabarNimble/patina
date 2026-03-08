---
type: belief
id: wit-is-contract-wasm-is-one-runtime
persona: architect
facets: [architecture, plugins, wit, wasm, portability]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-04
revised: 2026-03-04
---

# wit-is-contract-wasm-is-one-runtime

WIT defines the universal interface contracts for everything outside Patina core. WASM is one runtime that implements those contracts — wasmtime locally, Cloudflare Workers at the edge, native binaries for performance-critical paths. The interface standard (WIT types, capability grants) is the architecture. The execution environment is an implementation detail. A connector's interface is defined in WIT whether it runs as WASM, a native process, or a cloud function.

## Statement

WIT defines the universal interface contracts for everything outside Patina core. WASM is one runtime that implements those contracts — wasmtime locally, Cloudflare Workers at the edge, native binaries for performance-critical paths. The interface standard (WIT types, capability grants) is the architecture. The execution environment is an implementation detail. A connector's interface is defined in WIT whether it runs as WASM, a native process, or a cloud function.

## Evidence

- [[session-20260304-120702]]: Realized WASM is awkward for some roles — connectors are mostly I/O (wrapping HTTP in WASM host boundary is overhead), apps are their own deployments (not plugins), embeddings can't run in Workers (ONNX is 30-100MB, Workers limit ~10MB). WIT as contract language decouples interface from runtime. (weight: 0.9)
- [[session-20260304-120702]]: Cloudflare analysis showed the protocol maps well to edge (D1=SQLite, Workers=WASM, Vectorize=embeddings) but oxidize needs Workers AI, not local ONNX. Same WIT interface, different backend. (weight: 0.85)

## Supports

- [[patina-is-knowledge-protocol]] — The protocol is the architecture. WIT contracts are how the protocol extends. If the contracts are right, the runtime doesn't matter.
- [[reads-via-host-writes-via-intents]] — The read/write pattern is defined at the WIT level. Whether the host is wasmtime or Workers, the contract is the same.
- [[compiler-enforced-safety]] — WIT provides compile-time type safety at the boundary regardless of runtime.

## Attacks

- "WASM all the way down" — Defeated: WASM is great for grammars and extensions (pure compute, sandboxed). It's overhead for I/O-heavy connectors and impossible for large ML models. WIT contracts without WASM mandate is the right design.

## Attacked-By

- "Multiple runtimes means multiple implementations of host functions" — Valid. Each runtime (wasmtime, Workers, native) needs its own host implementation. Counter: the host functions are thin wrappers around stores (SQLite, git). The WIT contract ensures they behave identically.
- "WIT without WASM is just an IDL" — Valid tension. WIT's power comes from WASM's sandbox guarantees. A native binary speaking WIT types has no sandbox. Counter: capability grants in the manifest are the security model, not the sandbox alone.

## Applied-In

- Current 4 plugin worlds: all defined in WIT, all run in wasmtime locally. The WIT definitions would work unchanged in any WASM runtime.
- Forge schema: defined in WIT types (issue, pull-request records). These types are runtime-independent.

## Revision Log

- 2026-03-04: Created — metrics computed by `patina scrape`
