---
type: belief
id: anchored-beliefs-decay-faster
persona: architect
facets: [epistemic, beliefs, maintenance]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-26
revised: 2026-03-26
---

# anchored-beliefs-decay-faster

Beliefs that reference specific file paths, type names, or code structures decay faster than beliefs that state principles. After large refactors, principle-level beliefs survive while their anchoring details (Applied-In paths, type names, code examples) drift. Structure beliefs with a durable principle statement and treat code anchors as mutable evidence.

## Statement

Beliefs that reference specific file paths, type names, or code structures decay faster than beliefs that state principles. After large refactors, principle-level beliefs survive while their anchoring details (Applied-In paths, type names, code examples) drift. Structure beliefs with a durable principle statement and treat code anchors as mutable evidence.

## Evidence

- [[session-20260326-165149-931909000]]: Post-toy-collapse belief audit found 5 beliefs with stale anchors. All 5 had sound principles — the core statements were *more* true after the collapse. But Applied-In paths pointed at deleted files (`compat.rs`, `wit/worlds/`), type names referenced deleted types (`ConnectorToy`, `DuckLakeGrant`, `StorageToy`), and Attacked-By sections described mitigations using retired patterns. (weight: 0.95)
- [[session-20260326-165149-931909000]]: Beliefs that survived unchanged: [[five-boundaries-no-overlap]], [[children-are-wasm]], [[core-primitives-are-not-children]] — all principle-level with minimal code anchoring. Beliefs that needed revision: [[compat-seam-before-rewire]], [[children-have-agency-toys-are-capabilities]], [[connector-toy-is-indivisible-authority]], [[initialize-is-capability-grant]] — all had specific file paths and type names in Applied-In. (weight: 0.90)

## Supports

- [[stale-context-is-hostile-context]] — stale anchors in beliefs are a specific instance of hostile context
- [[practical-memory-over-epistemic-formalism]] — belief system is decision memory; anchors are the mechanical verification hooks that need maintenance

## Attacks

<!-- none yet -->

## Attacked-By

- "Why have code anchors at all if they decay?" — Because anchors enable mechanical verification (`patina scrape` can check if referenced paths exist). The solution is to treat them as mutable evidence, not to omit them.

## Applied-In

- Belief audit in [[session-20260326-165149-931909000]] — 5 beliefs revised for anchor drift after [[toy-collapse-wasi-alignment]] (40 commits, 128 files)

## Revision Log

- 2026-03-26: Created in [[session-20260326-165149-931909000]] — observed pattern during post-refactor belief staleness audit.
