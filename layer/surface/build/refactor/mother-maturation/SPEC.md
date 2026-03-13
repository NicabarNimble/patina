---
type: refactor
id: mother-maturation
status: draft
created: 2026-03-04
blocked_by: []
sessions:
  origin: 20260304-120702
related:
- raw-lake-ingestion
beliefs:
- mother-is-connection-and-continuity
- patina-is-beliefs-plus-action
- local-first-edge-deployable
exit_criteria:
- id: children-complete
  text: Remaining downstream specs are either completed, rewritten, or explicitly superseded, and the surviving Mother roadmap no longer depends on missing child specs
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
  Designed data flow: Sources → Data Lakes (raw) → Data Blocks
  (structured, shape TBD) → Projects. Mother manages lakes and
  routes belief streams.
- [[mother-is-connection-and-continuity]] — Mother federates instances,
  manages shared resources, runs continuously
- [[patina-is-beliefs-plus-action]] — Mother enables the "evolve" verb
  across projects via federation
- [[local-first-edge-deployable]] — Mother coordinates local and edge nodes

**Pipe architecture alignment ([[spec-pipe-architecture]]):**
Pipe architecture provides the unifying model for Mother's
responsibilities: Mother as broker (Netflix/Kafka pattern) with dual
role — manages children (spawn, monitor, lifecycle) AND routes facts
(source→destination based on pub/sub declarations). The child taxonomy
(connector, transport, lakehouse, transform), pipe protocol, and
connection model give Mother a coherent vocabulary for all the pieces
this container spec coordinates.

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
| [[data-architecture-v3]] | Provenance on events (local/external/derived) | **Complete** (v0.39.3) |
| [[mother-doctrine-cleanup]] | Clean Mother/child/toy doctrine before broader expansion | **Complete** (2026-03-12) |
| [[persona-federation]] | Persona registry, belief provenance, linking | Future build after queue cleanup |
| [[continuous-operation]] | Mother daemon, streaming, always-on | Future build after queue cleanup |

## Implementation Prerequisites

Resolve before or during implementation of child specs:

- **Data Blocks concept.** Lakes are raw, blocks are shaped — but what
  ARE blocks? Structured tables, semantic embeddings, filtered views, or
  something more versatile? This concept needs its own design and may
  become its own spec. Does NOT block [[spec-data-architecture-v3]]
  (provenance + lake registry proceed without it), but blocks the full
  4-layer data flow. See [[session-20260304-120702]] and
  [[spec-mother-maturation]] DESIGN.md, "Data Blocks: Exploration
  Territory."

## Future Work (not blocking children)

- **Edge interface** — how Patina apps on Cloudflare/Vercel connect
  back to local Mother. Deferred to a dedicated edge-interface spec
  after [[spec-continuous-operation]]. See continuous-operation DESIGN.md.
- **E2EE on belief streams** — [[content-addressed-references]] keeps
  the path open. Persona linking architecture does not preclude it.
- **Multi-user** — projects as islands with federation through Mother.
  The Mother-child model points the direction. Post-continuous-operation.

## Exit Criteria

This spec is complete when all three children are complete.
