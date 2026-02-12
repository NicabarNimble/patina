---
type: belief
id: graceful-extraction
persona: architect
facets: [architecture, plugins, migration]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-12
revised: 2026-02-12
---

# graceful-extraction

When moving functionality from core to plugin, keep the compiled version as a feature-gated fallback — plugin-first dispatch with compiled fallback means the system works regardless of plugin availability, and the compiled path is only removed after the plugin path is proven stable.

## Statement

When moving functionality from core to plugin, keep the compiled version as a feature-gated fallback — plugin-first dispatch with compiled fallback means the system works regardless of plugin availability, and the compiled path is only removed after the plugin path is proven stable.

## Evidence

- [[session-20260212-113744]]: [[plugin-system]] Phase 2 — doctor extraction uses bundled-doctor feature gate with WASM-first dispatch and compiled fallback (weight: 0.8)

## Supports

- [[compiler-enforced-safety]] — feature gates are compiler-checked; dead code is provably absent
- [[separate-worlds-for-isolation]] — each extracted module gets its own plugin boundary

## Attacked-By

- Complexity cost: two code paths (plugin + compiled) doubles the surface area during transition
  - Status: acknowledged — cost is bounded and temporary by design

## Applied-In

- `src/commands/doctor.rs` gated behind `#[cfg(feature = "bundled-doctor")]`, `src/main.rs` dispatches to WASM first with compiled fallback

## Revision Log

- 2026-02-12: Created — metrics computed by `patina scrape`
