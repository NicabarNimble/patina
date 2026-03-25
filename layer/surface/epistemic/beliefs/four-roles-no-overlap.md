---
type: belief
id: four-roles-no-overlap
persona: architect
facets: [architecture, identity, sdk, children]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-24
revised: 2026-03-24
---

# four-roles-no-overlap

Patina is the belief layer, Mother is the infrastructure, children are knowledge workers, projects are the dev zone — four concepts, four roles, no overlap

## Statement

Patina is the belief layer, Mother is the infrastructure, children are knowledge workers, projects are the dev zone — four concepts, four roles, no overlap

## Evidence

- [[session-20260324-101606-299953000]] - Distilled through extended toybox/architecture discussion: Patina owns the knowledge protocol (five verbs, layer is truth), Mother owns the runtime (daemon, sandbox, toys), children do the work (WASM, sandboxed, use SDK), projects are where you build children without modifying the platform (weight: 0.95)

## Supports

- [[patina-identity]] — refines the protocol identity into four distinct architectural roles
- [[core-primitives-are-not-children]] — children are workers, not the protocol itself
- [[children-have-agency-toys-are-capabilities]] — children use toys granted by Mother
- [[core-baseline-child-strategy-extensions]] — children extend core, never replace it

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[sdk-toybox-definition]] — toybox defined as Mother's controlled surface, children consume via SDK
- [[fix-grammar-pipeline]] — grammar children use pipeline toys, embedded fallback uses same WASM path
- [[greenfield-mother-patina-rebuild]] — M6 crate architecture separates core (patina) from runtime (mother)

## Revision Log

- 2026-03-24: Created — metrics computed by `patina scrape`
