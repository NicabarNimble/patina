---
type: belief
id: compat-seam-before-rewire
persona: architect
facets: [architecture, refactor, phase-gates, compatibility]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-26
revised: 2026-03-26T17
---

# compat-seam-before-rewire

For large refactors, introduce a thin compatibility seam first, then rewire targets behind it with phase-scoped gate checks so behavior stays stable while architecture evolves.

## Statement

For large refactors, introduce a thin compatibility seam first, then rewire targets behind it with phase-scoped gate checks so behavior stays stable while architecture evolves.

## Evidence

- [[session-20260326-063911]] Phase 2 of [[toy-collapse-wasi-alignment]] was executed as dispatch-only compat slices with no child logic or WIT contract churn (weight: 0.95)
- [[commit-db4d5b17]] First compat seam slice established ingress reroute pattern through `compat::*` while preserving behavior (weight: 0.90)
- [[commit-15169e0c]] Connector dispatch moved behind compat seam with targeted tests passing (weight: 0.90)
- [[commit-3614a99f]] Lake dispatch moved behind compat seam with parity preserved (weight: 0.90)
- [[commit-21ebc96b]] Events dispatch and peer event pull path moved to compat seam without behavior change (weight: 0.90)
- [[commit-84a6402f]] Finalized broad query/http/ingress compat routing with workspace checks still green (weight: 0.90)

## Supports

- [[phased-development-with-measurement]]
- [[process-checkpoints-over-tooling]]
- [[spec-first]]
- [[dependable-rust]]

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `src/child/toy_host/v2.rs` — the compat seam introduced in Phase 2 was fully rewired and collapsed into v2.rs as the single dispatch layer during later phases of toy-collapse-wasi-alignment
- `src/child/internal/knowledge_child.rs` — host dispatch now routes directly through `toy_host::v2` after compat indirection was removed post-collapse
- [[toy-collapse-wasi-alignment]] Phase 2 sequence executed as small parity-gated commits (the pattern's canonical proof; compat.rs was the transitional artifact, v2.rs is the permanent result)

## Revision Log

- 2026-03-26: Created — metrics computed by `patina scrape`
