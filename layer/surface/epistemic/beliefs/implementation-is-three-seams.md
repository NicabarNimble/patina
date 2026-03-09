---
type: belief
id: implementation-is-three-seams
persona: architect
facets: [architecture, planning, pipe-protocol]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-08
revised: 2026-03-08
---

# implementation-is-three-seams

Remaining implementation work is three seams (multi-destination broker, schema contract layer, lake control plane) not per-feature tasks — Seam 1→3 dependency, Seam 2 parallel.

## Statement

Remaining implementation work is three seams (multi-destination broker, schema contract layer, lake control plane) not per-feature tasks — Seam 1→3 dependency, Seam 2 parallel.

## Evidence

- [[session-20260308-210134]] - Gap analysis grounded in code: broker is single-destination (`src/broker/mod.rs`), schema parser behind authoring surface (`src/commands/schema/internal.rs` vs `children/github-connector/schema.toml`), Mother has no lake_registry (`src/mother/graph.rs`). Three seams organize the delta. (weight: 0.9)
- [[spec-pipe-architecture]] §Implementation Gap Analysis — seam decomposition with file references and dependency arrows (weight: 0.8)

## Supports

- [[unix-philosophy]] — each seam is one job: routing, contracts, control plane
- [[pipe-protocol-is-transport-agnostic]] — seams cut along protocol boundaries, not transport
- [[connectors-own-tables-schemas-are-contracts]] — Seam 2 implements this belief

## Attacks

<!-- None identified -->

## Attacked-By

<!-- Seam boundaries could shift if broker refactor proves inseparable from schema expansion -->

## Applied-In

- [[spec-pipe-architecture]] SPEC.md §Implementation Gap Analysis — three-seam framing with dependency graph and file mapping
- [[spec-pipe-architecture]] DESIGN.md §Key Files — reclassified existing infrastructure vs remaining gaps by seam

## Revision Log

- 2026-03-08: Created — metrics computed by `patina scrape`
