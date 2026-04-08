---
type: belief
id: children-are-portable-wasm-artifacts
persona: architect
facets: [architecture, children, distribution]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-07
revised: 2026-04-07
---

# children-are-portable-wasm-artifacts

Children are portable, versioned WASM artifacts that can be reused by many pandos; source trees in the monorepo are a development forge, not the long-term distribution boundary.

## Statement

Children are portable, versioned WASM artifacts that can be reused by many pandos; source trees in the monorepo are a development forge, not the long-term distribution boundary.

## Evidence

- [[session-20260407-063359]]: Architecture discussion established that children should live outside the repo long-term, be reusable Lego blocks, and be pulled as compiled artifacts for runtime use. (weight: 0.98)
- [[layer/surface/build/feat/pando-platform/SPEC.md]]: Phase C2/C3 direction now distinguishes monorepo source forge from runtime compiled artifact install and shareable composition identity. (weight: 0.92)

## Supports

- [[pandos-are-products-children-are-compute]] — strengthens the separation between user-facing products and reusable compute units.
- [[pandos-are-shareable-compositions]] — reusable child artifacts are the substrate that makes composition sharing practical.

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `folder-text-to-parquet` lifecycle work: moved status logic to installed/runtime artifact evaluation instead of treating missing runtime artifacts as manifest failure.

## Revision Log

- 2026-04-07: Created — metrics computed by `patina scrape`
