---
type: belief
id: pipes-are-processes-not-wasm
persona: architect
facets: [architecture, children, security, wasm, protocol]
entrenchment: medium
status: defeated
endorsed: true
extracted: 2026-03-05
revised: 2026-03-25
---

# pipes-are-processes-not-wasm

Pipe protocol supports multiple runtimes — WASM (proven by forge) and native processes (new). Runtime is a transport choice, not architectural. Children communicate the same protocol regardless of whether they run in wasmtime or as OS processes over stdio.

## Statement

Pipe protocol supports multiple runtimes — WASM (proven by forge) and native processes (new). Runtime is a transport choice, not architectural. Children communicate the same protocol regardless of whether they run in wasmtime or as OS processes over stdio.

## Evidence

- [[session-20260305-224446]]: Exploration session traced WASM overhead: all security is in host_support.rs (domain allowlist, credential injection, leak check), not in WASM sandbox. Chrome/Docker pattern: OS sandbox prevents bypass same as WASM prevents bypass. Proved native is viable. (weight: 0.9)
- [[session-20260306-123021]]: Architecture reframe: pipe = protocol, not process type. Children are managed services that speak pipe protocol over any transport. WASM and native are both first-class. (weight: 0.9)
- [[session-20260319-071818-503477000]]: Native child lane was explicitly retired as dead dual-system infrastructure (`spec-native-child-removal`, commit `a9c9f9ff`), re-centering child runtime on the WASM knowledge-child lane. (weight: 1.0)
- [[session-20260325-064204-876122000]]: Architecture decision locked as "Mother has internal services + external WASM children"; native service capabilities remain Mother-internal, not child-runtime doctrine. (weight: 1.0)

## Supports

- [[wit-is-contract-wasm-is-one-runtime]] — WIT defines the contract, WASM is one runtime; native is another. Both speak pipe protocol.
- [[unix-philosophy]] — children as single-purpose services, composed by Mother
- [[pipe-protocol-is-transport-agnostic]] — protocol-first design naturally supports multiple runtimes and transports

## Attacks

- Attacks the assumption that all external code must run in WASM for safety — OS sandboxing achieves the same isolation for I/O-bound workloads
- Attacks "pipe = process type" — pipe is the protocol, runtime is orthogonal

## Attacked-By

- "WASM uniformity is simpler" — one loading path, one testing path, one packaging format for everything. Counter: native children provide better dev ergonomics (cargo run, dbg!), streaming support, and direct HTTP. The protocol is the unifying layer, not the runtime.

## Applied-In

- [[spec-pipe-architecture]] — multi-runtime child model (WASM via patina-sdk, native via patina-pipe)
- `src/plugin/internal/host_support.rs` — host-side security logic reusable across runtimes
- `plugins/forge/src/github.rs` — first child, currently WASM, can migrate to native with same protocol

## Revision Log

- 2026-03-05: Created — pipes are processes, not WASM
- 2026-03-06: Revised — pipe = protocol, children can be WASM or native. Runtime is transport choice, not architectural.
- 2026-03-25: Defeated — native child doctrine retired after dual-system cleanup; children are now WASM-only in active architecture.
