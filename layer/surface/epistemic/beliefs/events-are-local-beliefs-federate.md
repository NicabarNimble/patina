---
type: belief
id: events-are-local-beliefs-federate
persona: architect
facets: [architecture, federation, data, replication]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-30
revised: 2026-03-30
---

# events-are-local-beliefs-federate

Events never federate between Mothers — they are the machine-local autobiography of a project's experience. Beliefs federate — they are the distilled knowledge that travels via git (today) and iroh (future). Projections (patina.db) rebuild locally from code. The replication unit is always the distilled artifact, never the raw event stream.

## Statement

Events never federate between Mothers — they are the machine-local autobiography of a project's experience. Beliefs federate — they are the distilled knowledge that travels via git (today) and iroh (future). Projections (patina.db) rebuild locally from code. The replication unit is always the distilled artifact, never the raw event stream.

## Evidence

- [[session-20260330-083255-177610000]] - Discovered through prior art study: LiveStore syncs events not databases, Matrix federates room events not room state, Git replicates objects not working copies. Every mature federated system replicates distilled artifacts, not raw state. Applied to Patina: beliefs travel, events stay. (weight: 0.9)

## Supports

- [[projects-are-sovereign-mother-coordinates]] — sovereignty means each Mother has its own experience (events). Only distilled knowledge (beliefs) crosses boundaries.
- [[if-its-patina-its-git]] — git is the federation mechanism for declarations. Events are experiences, not declarations.
- [[standards-are-storage-coordination-sits-above]] — SQLite events are the local storage unit. Federation is the coordination layer above.

## Attacks

- Event-sourcing architectures where the event log IS the replication unit (e.g., Kafka topics replicated across brokers). Patina's events are machine-specific; replicating them would mix autobiographies from different machines.

## Attacked-By

- Cross-machine debugging: if events don't federate, you can't trace a child's behavior across two Mothers. Mitigation: beliefs carry evidence links back to the originating session/events, providing provenance without raw event replication.

## Applied-In

- [[spec-greenfield-mother-patina-data-platform]] INV-4 — federation table explicitly states events NEVER federate, beliefs federate via git/iroh.
- Multi-Mother design in DESIGN.md — Mother-A and Mother-B share project_uid via git, each has its own events.db. Only beliefs (signed by persona keypair) travel between them.

## Revision Log

- 2026-03-30: Created — metrics computed by `patina scrape`
