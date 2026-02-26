---
type: refactor
id: data-db-split
status: draft
created: 2026-02-26
sessions:
  origin: 20260226-124149
related:
- data-architecture-v2
exit_criteria: []
---
# refactor: Database Split — events.db + patina.db Separation

> Create events.db for runtime events (autobiography), separate from patina.db (rebuildable cache). Rewire ~7 writers, ~3 readers, implement ATTACH for cross-system queries, one-time migration of 96 existing runtime events, update rebuild to skip events.db.

## Current State

## Target State

## Steps

## Exit Criteria
