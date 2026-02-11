---
type: belief
id: de-risk-runtime-with-simplest-payload
persona: architect
facets: [architecture, plugins, risk-management, wasmtime]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-11
revised: 2026-02-11
---

# de-risk-runtime-with-simplest-payload

When introducing a new runtime (wasmtime), start with the simplest payload (pure computation, no host imports) before tackling the hard problem (host-plugin communication) — validate the foundation before building on it.

## Statement

When introducing a new runtime (wasmtime), start with the simplest payload (pure computation, no host imports) before tackling the hard problem (host-plugin communication) — validate the foundation before building on it.

## Evidence

- [[session-20260211-125648]]: [[plugin-system]] spec orders grammars (Phase 1) before MotherChild WASM (Phase 2). Grammars are pure computation — no WIT host interfaces, no capability grants, no plugin manifest. If wasmtime integration fails at the grammar level, we know before investing in host↔plugin WIT interfaces. (weight: 0.9)
- [[session-20260211-100430]]: Walkthrough-driven discovery found 4 bugs that abstract design missed (zero safeguards, tag ordering, no major path, no versioning check). Same principle: test the simple case before building the complex one. (weight: 0.7)

## Supports

- [[compiler-enforced-safety]] — validate at the simplest level first; if the compiler can't enforce safety for grammars, it won't for plugins
- [[transparent-complexity]] — phasing reveals complexity incrementally instead of hiding it behind a big-bang integration
- [[ablate-before-optimizing]] — same principle applied to integration: prove each layer works in isolation

## Attacks

<!-- No beliefs defeated yet -->

## Attacked-By

- Speed of delivery: doing grammars first delays the high-value MotherChild plugins by one phase. The simplest payload isn't the most valuable payload.
  - Status: acknowledged — mitigated by grammar WASM being a small, bounded phase (5-8 build steps). The delay is weeks, not months. And a failed wasmtime integration discovered at Phase 2 would cost more than the Phase 1 investment.

## Applied-In

- [[plugin-system]] spec Phase 1: tree-sitter grammar WASM (pure computation, no host imports) before Phase 2: MotherChild WASM (WIT interfaces, host functions, capability grants)

## Revision Log

- 2026-02-11: Created — metrics computed by `patina scrape`
