---
type: belief
id: events-are-autobiography-not-telemetry
persona: architect
facets: [architecture, data-model, events]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-26
revised: 2026-03-08
---

# events-are-autobiography-not-telemetry

events.db captures the project's autobiography — epistemic moments, lifecycle transitions, decisions, and discoveries — not just operational metrics like scrape timings. events.db is project-scope: lake-bound data bypasses it entirely, with its own provenance trail (append-only Parquet, lake_sync metadata). Each consumer scope owns its own audit history.

## Statement

events.db captures the project's autobiography — epistemic moments, lifecycle transitions, decisions, and discoveries — not just operational metrics like scrape timings. events.db is project-scope: lake-bound data bypasses it entirely, with its own provenance trail (append-only Parquet, lake_sync metadata). Each consumer scope owns its own audit history.

## Evidence

- [[session-20260226-102315]]: [[session-20260226-102315]] - Kleppmann/Schickling analysis: the event log should capture every meaningful moment, layer/ markdown files are projections of current state, events are the irreplaceable history (weight: 0.9)

## Supports

- [[beliefs-are-where-machine-meets-human]] — epistemic events (belief.created, belief.contested) are the history that the belief layer grounds into
- [[measure-reads-tables-not-events]] — measure reads projections, but events.db provides the temporal dimension projections lack

## Attacks

- [[check-existing-emissions-before-adding]] — scoped: still valid for ops emissions, but the autobiography framing expands what "emission" means beyond ops

## Attacked-By

## Applied-In

- [[spec-data-architecture-v2]] § Layer 0: Events — event type registry expanded to include epistemic, lifecycle, decision, and discovery events
- [[spec-data-architecture-v2]] § Data Flow — events.db shown as IRREPLACEABLE, receiving all meaningful moments
- [[spec-raw-lake-ingestion]] — lake-bound data bypasses events.db; lakehouse child owns its own provenance (append-only Parquet, _ingested_at, _content_hash columns)
- [[spec-connector-owns-tables]] — events.db scoped as project-scope canonical write side; lake scope has separate audit trail

## Revision Log

- 2026-02-26: Created — metrics computed by `patina scrape`
- 2026-03-08: Revised — scoped to project. Lake-bound data bypasses events.db; each consumer scope owns its own audit history. Spirit unchanged (autobiography not telemetry), scope narrowed.
