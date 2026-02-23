---
type: belief
id: active-is-a-black-hole
persona: architect
facets: [architecture, spec-system, workflow]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-23
revised: 2026-02-23
---

# active-is-a-black-hole

Active is a black hole — every spec system treats it as one state covering 'just started' through 'stalled 3 months' and none handle mid-flight diversions

## Statement

Active is a black hole — every spec system treats it as one state covering 'just started' through 'stalled 3 months' and none handle mid-flight diversions

## Evidence

- [[session-20260223-092355]]: [[session-20260223-092355]] - Researched Rust RFCs (4 states), AWS ADRs (4 states), structured RFCs (5 states). All have active as a single undifferentiated state. None define pause, blocked, or split operations for mid-implementation diversions. Patina had the same gap before spec-workflow-rigor redesign. (weight: 0.9)

## Supports

- [[stale-context-is-hostile-context]] — stale active specs poison LLM context
- [[process-checkpoints-over-tooling]] — checkpoints break the black hole into observable states

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[spec-workflow-rigor]] — adds `paused` and `blocked` as explicit states to break the active black hole. Adds `spec split` for the half-done case.
- Rust RFC process — `active` covers everything from "just merged RFC" to "abandoned mid-implementation." No mid-flight states. (external evidence)
- AWS ADR process — `accepted` is terminal. If the decision changes, create a new ADR. No pause/resume. (external evidence)

## Revision Log

- 2026-02-23: Created — metrics computed by `patina scrape`
