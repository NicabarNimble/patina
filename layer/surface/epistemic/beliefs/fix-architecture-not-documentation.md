---
type: belief
id: fix-architecture-not-documentation
persona: architect
facets: [architecture, methodology]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-14
revised: 2026-02-14
---

# fix-architecture-not-documentation

Don't paper over architectural problems with comments or documentation — if the fix is designable and buildable, spec and build it. Comments that promise future work are technical debt disguised as helpfulness.

## Statement

Don't paper over architectural problems with comments or documentation — if the fix is designable and buildable, spec and build it. Comments that promise future work are technical debt disguised as helpfulness.

## Evidence

- [[session-20260214-130235]]: [[plugin-template-polish]] item 1 proposed a Cargo.toml comment promising crates.io syntax with no plan to deliver. Instead we designed [[patina-sdk]] to eliminate the absolute path entirely — real fix, not band-aid. (weight: 0.9)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-02-14: Created — metrics computed by `patina scrape`
