---
type: belief
id: children-are-wasm
persona: architect
facets: [architecture, children, runtime, mother]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-25
revised: 2026-03-25
---

# children-are-wasm

Children are external WASM runtime units; native capabilities belong to Mother services and are not children.

## Statement

Children are external WASM runtime units; native capabilities belong to Mother services and are not children.

## Evidence

- [[session-20260319-071818-503477000]] - removed native child dual-system via spec-native-child-removal and kept WASM child lane as canonical (weight: 1.0)

## Supports

- [[five-boundaries-no-overlap]] — children are workers, Mother is infrastructure; role boundaries stay explicit.
- [[core-primitives-are-not-children]] — core verbs remain protocol primitives, children stay extension lane.
- [[children-have-agency-toys-are-capabilities]] — agency/capability model remains, with WASM runtime as the child boundary.

## Attacks

- [[pipes-are-processes-not-wasm]] — defeats the multi-runtime child doctrine.
- [[host-proxied-io-is-the-security-model]] — defeats the dual-runtime security framing.
- [[sandbox-profiles-are-parameterized]] — defeats native child sandbox-profile doctrine.
- [[os-sandboxes-enforce-ports-not-domains]] — defeats native child OS-sandbox network policy as child-runtime doctrine.

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[child-construction-canon]] — "Only Three Things" section: children are WASM components that do compute. Two worlds: knowledge-child (Mother-hosted, has toys) and pipeline (host-invoked, log only).
- `sdk/patina-sdk/README.md` — SDK authoring lane documented as WASM children.
- `src/child/internal/mod.rs` — child manifest and runtime gating centered on child worlds/kinds.
- [[scaffold-world-retirement]] — retired command and task worlds, leaving only knowledge-child and pipeline as WASM-only child lanes.

## Revision Log

- 2026-03-25: Created — metrics computed by `patina scrape`
