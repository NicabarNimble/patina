---
type: belief
id: json-contract-over-shared-types
persona: architect
facets: [architecture, wasm, plugins]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-14
revised: 2026-02-14
---

# json-contract-over-shared-types

Plugin projects should define their own serialization types matching the host JSON contract, rather than sharing types across the WASM boundary — the contract is JSON, types are just serialization convenience.

## Statement

Plugin projects should define their own serialization types matching the host JSON contract, rather than sharing types across the WASM boundary — the contract is JSON, types are just serialization convenience.

## Evidence

- [[session-20260214-180546]] — grammar-cairo plugin defines local ExtractedData types (serialize-only) that match host JSON schema. Cleaner than depending on patina-ai internals across the WASM boundary. (weight: 0.9)
- Version independence: grammar-cairo uses cairo-lang-parser 2.15 while patina-metal uses 2.12. Shared types would force version alignment — JSON contract doesn't care. (weight: 0.7)

## Supports

- [[separate-worlds-for-isolation]] — JSON boundary is the ultimate isolation; no shared Rust types leak across worlds
- [[parser-agnostic-interfaces]] — the host doesn't care what parser tech produced the JSON, reinforced by no shared parser types

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Type drift risk: if host changes ExtractedData schema, plugins silently break at runtime instead of compile time. Mitigated by versioned envelope (`"version": "1"`) and graceful fallback.

## Applied-In

- `grammar-cairo/src/lib.rs` — local ExtractedData, CodeSymbol, FunctionFact, TypeFact, ImportFact, ConstantFact, MemberFact structs with `serde::Serialize` only (no Deserialize needed)

## Revision Log

- 2026-02-14: Created — metrics computed by `patina scrape`
