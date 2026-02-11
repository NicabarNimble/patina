---
type: belief
id: coupling-is-complexity
persona: architect
facets: [architecture, plugins, risk-management, complexity]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-11
revised: 2026-02-11
---

# coupling-is-complexity

When de-risking a new runtime, 'simplest payload' means lowest coupling to existing infrastructure, not fewest interface requirements. A module with zero imports but deep build-system entanglement (ABI versioning, vendored C compilation, hot-path processors) is harder to migrate than a module with host imports but clean trait boundaries.

## Statement

When de-risking a new runtime, 'simplest payload' means lowest coupling to existing infrastructure, not fewest interface requirements. A module with zero imports but deep build-system entanglement (ABI versioning, vendored C compilation, hot-path processors) is harder to migrate than a module with host imports but clean trait boundaries.

## Evidence

- [[session-20260211-133159]]: Grammar WASM looked simplest (pure computation, no host imports) but was deeply coupled to tree-sitter ABI versioning, patina-metal cc::Build, and 8 language processors on the scrape hot path. MotherChild has host imports and WIT but clean trait boundaries and zero existing consumers — actually simpler to WASM. (weight: 0.9)
- [[session-20250901-135830]]: tree-sitter 0.24 expects ABI 13-14, C/C++ grammars generate ABI 15. This version mismatch forced vendoring grammar C sources and compiling via cc::Build — the coupling that makes grammar WASM hard. The problem doesn't go away with WASM; it just moves. (weight: 0.8)
- [[session-20250824-113703]]: Implemented vendor-and-pin architecture specifically to work around grammar version hell. The build complexity exists because of coupling, not because of interface requirements. (weight: 0.7)

## Supports

- [[de-risk-runtime-with-simplest-payload]] — refines what "simplest" means: not fewest imports, but lowest coupling
- [[dependable-rust]] — clean trait boundaries (mod.rs + internal.rs) are what make a module easy to migrate; entangled build systems are what make it hard
- [[transparent-complexity]] — coupling is hidden complexity; interface requirements are visible complexity. Visible is safer.

## Attacks

<!-- No beliefs defeated yet -->

## Attacked-By

- Interface complexity is real cost: WIT definitions, host function implementations, and capability grants add code and design surface. A module with zero imports genuinely requires less new infrastructure to WASM — if the coupling cost is low.
  - Status: acknowledged — the belief is "measure coupling first, then interface complexity." When coupling is low (new module, no existing consumers), interface complexity becomes the dominant cost and the original [[de-risk-runtime-with-simplest-payload]] ordering holds.

## Applied-In

- [[plugin-system]] spec amendment: reordered Phase 1 from grammar WASM (zero imports, high coupling) to MotherChild WASM (host imports, clean boundaries). Grammars moved to Phase 5.

## Revision Log

- 2026-02-11: Created — metrics computed by `patina scrape`
