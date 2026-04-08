---
type: belief
id: pandos-are-shareable-compositions
persona: architect
facets: [architecture, pando, p2p]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-07
revised: 2026-04-07
---

# pandos-are-shareable-compositions

Pandos are shareable product manifests that compose child artifacts and toy bindings; they should be portable across a future P2P Mother network.

## Statement

Pandos are shareable product manifests that compose child artifacts and toy bindings; they should be portable across a future P2P Mother network.

## Evidence

- [[session-20260407-063359]]: Design direction emphasized sharing pandos and child/toy capability metadata across a future Mother network while keeping children reusable across multiple pandos. (weight: 0.96)
- [[layer/surface/build/feat/pando-platform/SPEC.md]]: Added Phase C3 gate for shareable composition identity distinct from child artifact identity and runtime instances. (weight: 0.9)

## Supports

- [[pandos-are-products-children-are-compute]] — shareable product manifests are the user-facing distribution unit.
- [[children-are-portable-wasm-artifacts]] — compositional sharing depends on reusable artifact-level children.

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- Phase C planning now includes C3 as a dedicated artifact/composition model slice before P2P transport implementation.

## Revision Log

- 2026-04-07: Created — metrics computed by `patina scrape`
