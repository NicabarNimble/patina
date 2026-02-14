---
type: belief
id: gate-exports-on-target-arch
persona: architect
facets: [rust, wasm, architecture, cargo]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-14
revised: 2026-02-14
---

# gate-exports-on-target-arch

Gate WASM export machinery on target_arch, not feature flags — when multiple workspace consumers need different features of an SDK crate, export\!() and its bridge code must be #[cfg(target_arch = "wasm32")] to prevent ABI symbol conflicts from Cargo's feature unification.

## Statement

Gate WASM export machinery on target_arch, not feature flags — when multiple workspace consumers need different features of an SDK crate, export\!() and its bridge code must be #[cfg(target_arch = "wasm32")] to prevent ABI symbol conflicts from Cargo's feature unification.

## Evidence

- [[session-20260214-154148]]: [[session-20260214-154148]] - Discovered during patina-sdk build: workspace unifies command+mother-child features, causing duplicate cabi_post_name symbols from export\!(Component). Gating on wasm32 lets native builds unify harmlessly while wasm32 builds enforce single-world. (weight: 0.95)

## Supports

- [[compiler-enforced-safety]] — enforces single-world at compile time where it matters
- [[separate-worlds-for-isolation]] — preserves world isolation in the SDK consolidation

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `patina-sdk/src/task.rs` — `#[cfg(target_arch = "wasm32")] mod __wasm { ... export!(Component); }`
- `patina-sdk/src/command.rs` — same pattern
- `patina-sdk/src/mother_child.rs` — same pattern
- `patina-sdk/src/pipeline.rs` — same pattern
- `patina-sdk/src/lib.rs` — `compile_error!` mutual exclusion also wasm32-gated

## Revision Log

- 2026-02-14: Created — metrics computed by `patina scrape`
