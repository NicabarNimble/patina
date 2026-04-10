---
type: belief
id: integrate-do-not-duplicate
persona: architect
facets: [architecture, integration, complexity-management]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-10
revised: 2026-04-10
---

# integrate-do-not-duplicate

When an existing system already solves a problem, integrate with it — parallel implementations of the same concept create drift, divide attention, and quietly diverge over time.

## Statement

When an existing system already solves a problem, integrate with it — parallel implementations of the same concept create drift, divide attention, and quietly diverge over time.

## Evidence

- [[session-20260409-143847-707078000]] - About to scaffold a custom folder structure and SQLite database to track external project information when the user pointed out the existing repo system already does clone, scrape, index, and search; the right move was to register repos with the existing tool, not invent a parallel store (weight: 0.95)
- [[commit-8138e6b5]] - The scaffold for tracking upstream truth deliberately uses the existing repo tooling instead of a parallel database; the README documents this as the constraint
- [[commit-f5804bef]] - A spec codifies the integration approach as the foundation for tracking upstream direction
- [[spec-ba-truths]] - The spec exists because the first instinct (build a parallel store) was caught and replaced with an integration approach

## Supports

- [[dependable-rust]] — small public interfaces and stable substrates depend on having one canonical implementation per concept, not several
- [[unix-philosophy]] — composing existing tools is the unix way; building parallel tools is its opposite

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- A foundation for tracking upstream project direction was scoped to use the existing repository management system instead of inventing a parallel knowledge store. Integration was scaffolded as a thin orientation folder over the existing infrastructure.
- A library crate's type re-export problem was diagnosed as a special case of the same anti-pattern: trying to provide a parallel type identity instead of using the underlying contract's existing types directly.

## Revision Log

- 2026-04-10: Created — metrics computed by `patina scrape`
