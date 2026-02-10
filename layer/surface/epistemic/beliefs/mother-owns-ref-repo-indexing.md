---
type: belief
id: mother-owns-ref-repo-indexing
persona: architect
facets: [architecture, mother, ref-repos]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-08
revised: 2026-02-08
---

# mother-owns-ref-repo-indexing

Reference repos are shared knowledge that mother holds and indexes — projects declare a dependency on that knowledge, but mother decides how to store, index, and serve it.

## Statement

Reference repos are shared knowledge that mother holds and indexes — projects declare a dependency on that knowledge, but mother decides how to store, index, and serve it.

## Evidence

- [[session-20260208-113613]]: Phase 2 cleanup discussion: `oxidize_for_repo()` creates recipes inside ref repos, but this is the project reaching past mother to configure shared infrastructure. Projects are the door, mother is the house. (weight: 0.9)
- [[session-20260208-113613]]: 100 dojo games shouldn't each decide how dojo gets indexed. Mother holds one copy, knows which projects use it, routes queries via graph edges. (weight: 0.85)
- [[mother-is-the-daemon]]: Mother is the always-running daemon — ref repo lifecycle (clone, scrape, oxidize, serve) is a facet of mother, not a project responsibility. (weight: 0.8)

## Supports

- [[mother-is-the-daemon]]: Ref repo management is a facet of the daemon, not separate
- [[unix-philosophy]]: Projects do one job (your work). Mother does one job (shared knowledge). Don't mix.
- [[corpus-composition-over-model]]: Mother can choose the right corpus composition for ref repos independently of how projects compose their knowledge domain

## Attacks

- [[repo-add-complete-result]]: Current `patina repo add` runs oxidize from project context, creating recipes inside ref repos. This works but violates the ownership boundary.

## Attacked-By

- Pragmatism: today mother doesn't have its own oxidize pipeline. `oxidize_for_repo()` works. Removing it before mother can replace it would break ref repo indexing.

## Applied-In

- NOT YET APPLIED — current `oxidize_for_repo()` in `src/commands/oxidize/mod.rs` violates this belief by creating recipes inside ref repos. Future: mother-owned indexing pipeline.

## Revision Log

- 2026-02-08: Created — metrics computed by `patina scrape`
