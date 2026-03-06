---
type: belief
id: pipes-are-processes-not-wasm
persona: architect
facets: [architecture, pipes, security, wasm]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-05
revised: 2026-03-05
---

# pipes-are-processes-not-wasm

Pipes are native processes communicating over stdio, not WASM components — WASM is for plugins (domain logic, community extensions), pipes are I/O-bound data drivers where safety comes from host-proxied I/O, capability manifests, and OS sandboxing, not compute sandboxing

## Statement

Pipes are native processes communicating over stdio, not WASM components — WASM is for plugins (domain logic, community extensions), pipes are I/O-bound data drivers where safety comes from host-proxied I/O, capability manifests, and OS sandboxing, not compute sandboxing

## Evidence

- [[session-20260305-224446]]: [[session-20260305-224446]] - Exploration session traced WASM overhead for pipes: all security is in host_support.rs (domain allowlist, credential injection, leak check), not in WASM sandbox. Chrome/Docker pattern: OS sandbox prevents bypass same as WASM prevents bypass (weight: 0.9)

## Supports

- [[wit-is-contract-wasm-is-one-runtime]] — reinforces that WIT is the contract, WASM is one runtime; pipes choose native process as their runtime
- [[unix-philosophy]] — pipes as single-purpose transform binaries, composed by Mother
- [[host-proxied-io-is-the-security-model]] — the security model that makes non-WASM pipes safe
- [[pipe-protocol-is-transport-agnostic]] — process-based pipes naturally support multiple transports

## Attacks

- Attacks the assumption that all external code must run in WASM for safety — OS sandboxing achieves the same isolation for I/O-bound workloads

## Attacked-By

- "WASM uniformity is simpler" — one loading path, one testing path, one packaging format for everything. Counter: the development and streaming costs outweigh uniformity benefits for I/O-bound pipes

## Applied-In

- [[spec-pipe-architecture]] — pipe architecture spec to be updated with process-based model
- `src/plugin/internal/host_support.rs` — all host-side security logic (domain allowlist, credential injection, leak check) is reusable for process-based pipes

## Revision Log

- 2026-03-05: Created — metrics computed by `patina scrape`
