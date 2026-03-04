---
type: refactor
id: mother-maturation
status: draft
created: 2026-03-04
blocked_by:
- plugin-infrastructure
sessions:
  origin: 20260304-120702
beliefs:
- mother-is-connection-and-continuity
- patina-is-beliefs-plus-action
- local-first-edge-deployable
exit_criteria:
- id: children-complete
  text: All child specs (data-architecture-v3, persona-federation, continuous-operation) are complete
  checked: false
---
# refactor: Mother Maturation — Federation, Lakes, and Continuity

> Grow Mother into the full federation layer. Data lakes, persona
> registry, belief streams, continuous operation.

## Context

This is a **container spec** tracking three child specs that mature
Mother from a project registry into the nervous system connecting all
Patina instances.

**Architecture context:**
- [[session-20260303-190855]] — "Mother has always been the catch-all
  for things that don't fit in a project AND for connecting multiple
  Patina instances. It is so important for patina to connect."
- [[session-20260304-120702]] — established Mother as "connection +
  continuity", scoped to registries and routing, not business logic.
  Designed 4-layer data flow: Sources → Lakes → Modified Data Blocks →
  Projects. Mother manages lakes and routes belief streams.
- [[mother-is-connection-and-continuity]] — Mother federates instances,
  manages shared resources, runs continuously
- [[patina-is-beliefs-plus-action]] — Mother enables the "evolve" verb
  across projects via federation
- [[local-first-edge-deployable]] — Mother coordinates local and edge nodes

**What Mother does today:**
- Project registry (`mother.db → project_registry`)
- Ref repo registry (cross-project git references)
- Mother-child plugin world (daemon with heartbeat, tick, health)
- Cross-project belief search (`patina mother`)
- Graph.db for federated FTS5 search

**What Mother needs to become:**
- Persona registry (who) — UIDs, visibility, linking
- Lake registry (external data) — name, kind, location, credentials, sync
- Connector registry (how to reach sources) — plugin-to-lake mapping
- Cross-project belief streams (knowledge flow) — directional, scoped
- Continuous operation — daemon, always syncing, not batch

## Children

| Spec | What it delivers | Build order |
|------|-----------------|-------------|
| [[data-architecture-v3]] | Provenance on events, lake registry in Mother | First |
| [[persona-federation]] | Persona registry, belief provenance, linking | Second (or parallel) |
| [[continuous-operation]] | Mother daemon, streaming, always-on | Third (depends on v3 + personas) |

## Exploration Needed

- **Modified data blocks** — what are they exactly? Structured tables?
  Semantic embeddings? Filtered views? This concept emerged in session
  20260304-120702 but needs more design work. Could be its own spec.
- **Edge interface** — how do Patina apps on Cloudflare/Vercel connect
  back to local Mother? WebSocket? HTTP polling? Push via R2? Not specced
  yet, future work after continuous-operation.
- **E2EE on belief streams** — user has blockchain/Signal background
  (Giza, Starknet STWO). Belief streams could be E2EE. Not specced yet
  but [[content-addressed-references]] keeps the path open.
- **Multi-user** — projects as islands with federation through Mother,
  not shared git. Needs more design but the Mother-child model points
  the right direction.

## Exit Criteria

This spec is complete when all three children are complete.
