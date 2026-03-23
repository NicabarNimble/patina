---
type: belief
id: core-verbs-standalone-mother-additive
persona: architect
facets: [architecture, protocol, mother, cli]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-22
revised: 2026-03-22
---

# core-verbs-standalone-mother-additive

Patina core protocol verbs must remain standalone-capable in the CLI; Mother enhances and orchestrates those verbs but must not be a hard dependency for local protocol execution.

## Statement

Patina core protocol verbs must remain standalone-capable in the CLI; Mother enhances and orchestrates those verbs but must not be a hard dependency for local protocol execution.

## Evidence

- [[session-20260321-162736-004031000]] - pre-v1 build surfaced daemon-first stubs in front of working local verbs, prompting explicit zero-fallback cutover correction (weight: 1.0)
- [[session-20260320-212325-011658000]] - architecture clarification: Patina is the knowledge protocol and Mother is infrastructure; core verbs and beliefs are the product surface (weight: 1.0)
- [[spec-patina-zero-fallback-cutover]] - cutover gates were introduced to remove scaffold/fallback drift and enforce explicit runtime boundaries (weight: 0.9)

## Supports

- [[patina-is-knowledge-protocol]] — protocol core should work standalone
- [[beliefs-are-the-product]] — infrastructure exists to serve belief quality
- [[core-primitives-are-not-children]] — children are strategy extensions, not protocol ownership

## Attacks

- "Daemon-first routing should always front core verbs" — defeated for protocol operations that are valid local file+git workflows

## Attacked-By

- "Single daemon path simplifies architecture" — valid pressure; mitigation is explicit enhancement merge points and parity tests

## Applied-In

- `layer/surface/build/refactor/patina-zero-fallback-cutover/SPEC.md` — layering contract for daemon enhancement without core protocol dependency
- `src/commands/lake.rs` — validation-before-daemon correction prevents daemon coupling for deterministic local checks

## Revision Log

- 2026-03-22: Created — metrics computed by `patina scrape`
