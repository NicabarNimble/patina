---
type: belief
id: wasi-is-foundation-not-option
persona: architect
facets: [architecture, wasi, component-model, sdk]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-27
revised: 2026-03-27
---

# wasi-is-foundation-not-option

Patina builds with the WASI ecosystem, never parallel to it — standard interfaces are used directly, custom toys cover only the delta, and new interfaces follow WASI conventions to enable upstream contribution.

## Statement

Patina builds with the WASI ecosystem, never parallel to it — standard interfaces are used directly, custom toys cover only the delta, and new interfaces follow WASI conventions to enable upstream contribution.

## Evidence

- [[session-20260327-104954-066673000]] - Audit of 15 custom toys found 6 overlapping WASI proposals; discussion crystallized principle that Patina should compose on standards not duplicate them, and contribute to growing the Component Model ecosystem (weight: 0.9)

## Supports

- [[children-have-agency-toys-are-capabilities]] — toys that align with WASI are more portable and composable, strengthening the grant model
- [[observation-at-the-boundary]] — standard WASI interfaces make boundary observation consistent across ecosystem tooling

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[child-construction-canon]] — codifies this as hard rule 8: "WASI is foundation, not option. Toys use standard WASI interfaces where they exist; custom interfaces cover only the delta."
- [[toy-collapse-wasi-alignment]] — collapsed 22 toys to 10, aligning 6 with WASI proposals

## Revision Log

- 2026-03-27: Created — metrics computed by `patina scrape`
