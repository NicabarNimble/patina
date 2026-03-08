---
type: belief
id: steenberg-lens-immutable-core
persona: architect
facets: [architecture, durability, design-philosophy]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-03
revised: 2026-03-03
---

# steenberg-lens-immutable-core

Modules are black boxes with stable public APIs. Architecture should not require periodic redesign. If a change fragments architectural coherence or introduces a parallel pattern instead of extending the core one, redesign before merging. The test: would this module survive unchanged for years?

## Statement

Modules are black boxes with stable public APIs. Architecture should not require periodic redesign. If a change fragments architectural coherence or introduces a parallel pattern instead of extending the core one, redesign before merging. The test: would this module survive unchanged for years?

## Evidence

- [[session-20260303-101839]]: Formalized from Eskil Steenberg's philosophy as applied across Patina sessions — dependable-rust captures the module pattern, this belief captures the architectural durability test. Applied as decision filter in data-fast-incremental audit (session-20260303-070328) where Steenberg's concern about shell hooks outside module boundary led to patina hook post-commit subcommand (weight: 0.9)

## Supports

- [[dependable-rust]] — the module pattern is the implementation of this lens
- [[dispatch-is-two-levels]] — layered dispatch prevents parallel patterns (one dispatch mechanism extended, not a second one created)

## Attacks

## Attacked-By

- [[yegge-lens-spec-code-proportionality]] — tension: immutable core requires upfront design = more spec work. Resolution: the spec investment pays off in avoided redesign.

## Applied-In

- `src/plugin/internal/mod.rs` — PluginWorld enum stable since v0.17.0, 4 worlds added incrementally without redesign
- `wit/pipeline/pipeline.wit` — pipeline interface unchanged since creation, 9 grammar plugins extend without modifying
- [[session-20260303-070328]]: Steenberg's concern about shell hooks outside module boundary led to `patina hook post-commit` subcommand — logic in testable Rust, not shell scripts

## Revision Log

- 2026-03-03: Created — metrics computed by `patina scrape`
