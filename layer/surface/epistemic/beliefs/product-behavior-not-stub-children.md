---
type: belief
id: product-behavior-not-stub-children
persona: architect
facets: [architecture, children, mother, pando]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-05-27
revised: 2026-05-27
---

# product-behavior-not-stub-children

Product-shaped behavior should not live as first-party stub children; it should be native infrastructure or external child/Pando packages.

## Statement

Product-shaped behavior should not live as first-party stub children; it should be native infrastructure or external child/Pando packages.

## Evidence

- Session update captured removal of product-shadow children and PR merge evidence: [[session-20260512-171843-557794000]], [[commit-b1bb746b]], [[commit-c63f1e67]]. This generalizes cleanup of `children/spec-manager`, `children/doctor`, `children/belief-verifier`, and `children/session-writer` while preserving native spec/session/doctor/belief infrastructure or external package boundaries.

## Supports

- [[children-are-wasm]] — children are runtime units, not shadows for native Mother/Patina capabilities.
- [[core-primitives-are-not-children]] — core primitives remain native infrastructure; children feed or extend core rather than replacing it with stubs.
- [[pandos-are-products-children-are-compute]] — product behavior belongs at the Pando/product layer, while children are compute units.
- [[code-is-not-core]] — domain/product behavior should move to extension/package surfaces instead of bloating core.

## Attacks

- The pattern of keeping first-party stub children solely because their names mirror real product surfaces.

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[children-storage-cleanup]] — product-shadow child cleanup removed `children/spec-manager`, `children/doctor`, `children/belief-verifier`, and `children/session-writer` from the workspace while preserving Mother loader fixtures.
- [[commit-b1bb746b]] — removed obsolete product-shadow child source crates.
- [[commit-c63f1e67]] — merged the cleanup to main via PR #131.

## Revision Log

- 2026-05-27: Created — metrics computed by `patina scrape`
