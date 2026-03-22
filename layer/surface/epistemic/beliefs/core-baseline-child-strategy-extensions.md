---
type: belief
id: core-baseline-child-strategy-extensions
persona: architect
facets: [architecture, protocol, children, toys, resilience]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-22
revised: 2026-03-22
---

# core-baseline-child-strategy-extensions

Children extend protocol verbs through strategy interfaces; core verbs must retain standalone baseline behavior and child failures must not break core results.

## Statement

Children extend protocol verbs through strategy interfaces; core verbs must retain standalone baseline behavior and child failures must not break core results.

## Evidence

- [[session-20260321-162736-004031000]] - pre-v1 review exposed daemon-first stubs and fallback drift, motivating a core-baseline plus strategy-extension contract (weight: 1.0)
- [[session-20260320-212325-011658000]] - architecture deep-dive reframed toys/children as composable strategy extensions while preserving protocol identity (weight: 0.95)
- [[core-primitives-are-not-children]] - established that primitives are core-owned and children feed into them (weight: 0.9)

## Supports

- [[patina-is-knowledge-protocol]] — protocol core remains standalone and LLM-agnostic
- [[core-primitives-are-not-children]] — primitives are core; children contribute strategies
- [[children-have-agency-toys-are-capabilities]] — bounded child agency through capability contracts

## Attacks

- "Core verbs should route daemon-first and depend on strategy children for primary behavior"

## Attacked-By

- "Single daemon execution path reduces complexity" — valid concern; mitigated by strict merge/failure contracts and parity tests

## Applied-In

- `layer/surface/build/refactor/patina-zero-fallback-cutover/SPEC.md` — basis for command-level baseline/enhancement behavior matrix
- `src/commands/lake.rs` — validation-before-daemon correction preserves deterministic local behavior before orchestration attempts

## Revision Log

- 2026-03-22: Created — metrics computed by `patina scrape`
