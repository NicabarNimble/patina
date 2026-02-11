---
type: refactor
id: database-identity-federation
status: design
created: 2026-02-06
blocked_by:
  - mother-architecture
blocks: []
related:
  - layer/surface/build/feat/mother-architecture/SPEC.md
beliefs:
  - project-config-in-git
---

# refactor: Database Identity — Federation Integration

> Wire UIDs into mother graph, DB generation tracking, and cross-DB references.

## Context

Phase 1 (UID creation) shipped in [[database-identity]] — all projects and ref repos have
stable 8-hex-char UIDs via `.patina/uid`. Phase 2-3 wire those UIDs into federation.

## Phase 2: DB Generation + Mother Graph

- [ ] `_meta` table in patina.db with generation counter (incremented on rebuild)
- [ ] Mother graph uses UID as primary key (not name/path)
- [ ] Collision detection on registration (warn, don't block)
- [ ] Edges reference UIDs, not names/paths
- [ ] Staleness detection via `last_indexed_generation`

## Phase 3: Cross-DB References

- [ ] Eventlog stores `source_uid` on federated results
- [ ] Scry results include source UID in output
- [ ] Full provenance tracking (`[uid:generation]` on every result)

## Design

See archived [[spec/database-identity]] for full design details:
- DB generation schema (`_meta` table)
- Collision handling (detect + warn, last-write-wins)
- Federation schema (`nodes` + `edges` tables with UID keys)
- Multi-user namespace (`user:uid`)

## Blocked By

Mother federation — Phase 2-3 require the mother graph to be actively used for
cross-project queries. No point wiring UIDs into a graph that isn't queried yet.
