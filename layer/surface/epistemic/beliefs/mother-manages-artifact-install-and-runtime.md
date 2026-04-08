---
type: belief
id: mother-manages-artifact-install-and-runtime
persona: architect
facets: [architecture, mother, runtime]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-07
revised: 2026-04-07
---

# mother-manages-artifact-install-and-runtime

Mother owns artifact install/cache and runtime instantiation as separate concerns: install proves readiness, runtime health proves liveness.

## Statement

Mother owns artifact install/cache and runtime instantiation as separate concerns: install proves readiness, runtime health proves liveness.

## Evidence

- [[session-20260407-063359]]: Lifecycle redesign clarified the difference between ready and live and that Mother should evaluate installed artifacts separately from currently loaded instances. (weight: 0.97)
- [[src/commands/mother/daemon.rs]] and [[mother/src/pando.rs]]: implementation now separates installed-child resolution from live-child health to project `registered`/`ready`/`live`/`degraded`. (weight: 0.94)

## Supports

- [[children-are-portable-wasm-artifacts]] — install/cache ownership in Mother is required for portable artifact reuse.
- [[pandos-are-products-children-are-compute]] — Mother provides the OS-level runtime boundary between product manifests and compute artifacts.

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `pando-platform` PP5C1: protocol v2 lifecycle statuses and registry evaluation split between install readiness and runtime liveness.

## Revision Log

- 2026-04-07: Created — metrics computed by `patina scrape`
