---
type: belief
id: incremental-maintenance-requires-stable-ids
persona: architect
facets: [data-architecture, scrape, maintenance]
confidence:
  score: 0.85
entrenchment: medium
status: active
extracted: 2026-03-03
references: [scrape-diff-driven]
---

# incremental-maintenance-requires-stable-ids

Incremental index maintenance requires stable identifier schemas.

## Statement

When an index uses incremental updates (insert/delete by key prefix) instead of full rebuilds, the key format becomes an implicit contract. Changing the key format without a migration plan silently breaks incremental updates — old entries persist, new entries duplicate, and search quality degrades without any error signal.

## Evidence

- [[scrape-diff-driven]] EC6 — FTS5 incremental uses `LIKE './path.rs%'` prefix matching because the eventlog `source_id` format is `./path.rs::symbol_name`. If the `./` prefix is dropped or `::` separator changes, the LIKE clause silently matches zero rows, leaving stale entries.
- Guard-rail test `test_source_id_format_matches_fts5_assumption` in `database.rs` — asserts the source_id format so format changes produce a test failure rather than silent degradation.
- Rename gap: file renames produce the new path in the delta but not the old path. Old file's FTS5 entries persist until the next full rebuild. This is a concrete example of the general principle.

## Supports

- [[eventlog-is-truth]] — the eventlog's source_id is an identifier contract that downstream consumers depend on

## Attacks

- (none)

## Implications

- Before changing eventlog source_id format: update `populate_fts5()` incremental path AND the guard-rail test.
- Any new incremental index must document its key format assumptions and add a guard-rail test.
- Full rebuilds are the safety net — they correct accumulated drift from incremental bugs.

## Applied-In

- `src/commands/scrape/database.rs` — `populate_fts5()` LIKE prefix matching, guard-rail test
- `src/commands/scrape/code/database.rs` — source_id format: `format!("{}::{}", path, name)`

## Revision Log

- 2026-03-03: Extracted from [[scrape-diff-driven]] audit. Generalizes the FTS5 prefix coupling into a maintenance principle.
