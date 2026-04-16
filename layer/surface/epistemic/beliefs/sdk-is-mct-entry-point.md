---
type: belief
id: sdk-is-mct-entry-point
persona: architect
facets: [sdk, architecture, developer-experience]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-09
revised: 2026-04-09
---

# sdk-is-mct-entry-point

patina-sdk is the developer entry point into MCT — it should help developers understand Mother-Child-Toy architecture, build children with correct WIT wiring, use and discover toys, assemble pandos, and verify consistency.

## Statement

patina-sdk is the developer entry point into MCT — it should help developers understand Mother-Child-Toy architecture, build children with correct WIT wiring, use and discover toys, assemble pandos, and verify consistency.

## Evidence

- [[session-20260409-143847-707078000]] - SDK rebuild revealed toy wrappers are a leaf, not the trunk; developer needs span understanding, building, wiring, composing, and verifying (weight: 0.9)

## Supports

- [[children-have-agency-toys-are-capabilities]] — SDK must surface the toy grant model to developers
- [[compiler-enforced-safety]] — SDK should catch WIT/toy mismatches at dev time, not load time
- [[wasi-is-foundation-not-option]] — SDK guides developers into the WASM component world correctly

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-04-09: Created — metrics computed by `patina scrape`
