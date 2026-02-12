---
type: belief
id: hoststate-cohabits-with-bindgen
persona: architect
facets: [wasmtime, rust, plugin-system]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-11
revised: 2026-02-11
---

# hoststate-cohabits-with-bindgen

In wasmtime v41, the HostState struct and Host trait impl must live in the same module as the bindgen\! macro invocation, or trait coherence fails due to competing HasData/HostWithStore impl generation

## Statement

In wasmtime v41, the HostState struct and Host trait impl must live in the same module as the bindgen\! macro invocation, or trait coherence fails due to competing HasData/HostWithStore impl generation

## Evidence

- [[session-20260211-203416]]: [[session-20260211-203416]] - Discovered through iterative compilation: placing Host impl outside mod bindings caused 'multiple impls satisfying HasData' errors. Moving HostState + impl inside the bindings module resolved it. Confirmed via cargo expand showing generated HostWithStore blanket impl (weight: 0.95)

## Supports

- [[compiler-enforced-safety]] — the compiler catches this mistake immediately; no runtime surprise
- [[dependable-rust]] — the bindings module IS the internal.rs pattern: generated types stay private

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `src/plugin/internal.rs` — mod bindings contains HostState, bindgen!, and Host impl together

## Revision Log

- 2026-02-11: Created — metrics computed by `patina scrape`
