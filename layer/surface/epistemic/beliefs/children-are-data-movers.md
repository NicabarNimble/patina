---
type: belief
id: children-are-data-movers
persona: architect
facets: [architecture, children, toys]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-26
revised: 2026-03-26
---

# children-are-data-movers

Children are data movers and transformers, not general-purpose compute. They operate within Patina's data flow: ingesting, transforming, and routing data through the platform. This scopes the toy set to data-oriented primitives.

## Statement

Children are data movers and transformers, not general-purpose compute. They operate within Patina's data flow: ingesting, transforming, and routing data through the platform. This scopes the toy set to data-oriented primitives.

## Evidence

- [[session-20260325-150227-161735000]] - Discovery that in a world of 1000s of children, they have a tighter scope than 'anything WASM can do' — they're scoped to Patina's data concerns. A github toy vs a google workspace toy are just http toys with different creds. (weight: 0.95)

## Supports

- [[children-have-agency-toys-are-capabilities]] — bounded agency within data movement scope, not unbounded compute
- [[children-are-wasm]] — WASM sandbox is the right isolation for data workers
- [[four-roles-no-overlap]] — children are one role (data movement), not everything

## Attacks

- General-purpose WASM child model (children can do anything WASM supports) — superseded by data-mover scoping

## Attacked-By

- Future domain expansion: if Patina enters domains requiring compute-heavy children (ML inference, simulation), the data-mover scope may be too narrow. Mitigated by: WASI 0.3 proposals (`wasi:nn` for ML) could expand toy set without changing the bounded-agency model.

## Applied-In

- [[toy-collapse-wasi-alignment]] — toy set scoped to data primitives (http, fs, store, events, state, log, task, peer, git, connect) because children are data movers, not general-purpose workers
- [[cloudflare-worker-child]] — portable child proof uses data-oriented toy subset (http, state, log)

## Revision Log

- 2026-03-26: Created — metrics computed by `patina scrape`
