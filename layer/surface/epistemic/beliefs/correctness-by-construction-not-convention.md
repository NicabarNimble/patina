---
type: belief
id: correctness-by-construction-not-convention
persona: architect
facets: [architecture, data-model, safety]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-26
revised: 2026-02-26
---

# correctness-by-construction-not-convention

Safety properties should be enforced by data layout (separate files, physical separation) rather than code conventions (WHERE clauses, flags, guards). When you can make the wrong thing impossible by construction, don't rely on every future developer remembering the right convention.

## Statement

Safety properties should be enforced by data layout (separate files, physical separation) rather than code conventions (WHERE clauses, flags, guards). When you can make the wrong thing impossible by construction, don't rely on every future developer remembering the right convention.

## Evidence

- [[session-20260226-152857]]: [[session-20260226-094014]] - The pivotal reframing that drove data-db-split. The 'protected rebuild' alternative (export/restore runtime events during rebuild) was rejected because it required every future rebuild path to include the protection logic. File separation makes the wrong thing (deleting events during rebuild) impossible because rebuild never opens events.db. (weight: 0.95)

## Supports

- [[if-its-patina-its-git]] — justifies WHY the two sources of truth (git + events.db) are physically separated files rather than logically separated tables in one database
- [[events-are-autobiography-not-telemetry]] — the autobiography can't be accidentally destroyed if it lives in a file that rebuild never opens

## Attacks

## Attacked-By

## Applied-In

- [[spec-data-db-split]] — events.db + patina.db separation: rebuild deletes patina.db, never opens events.db. The wrong thing (destroying runtime events) is impossible by construction.
- `src/commands/scrape/mod.rs:114` — `execute_rebuild()` operates on `db_path` (patina.db) only. events.db path never appears in the rebuild code path.
- [[spec-data-db-split-fixes]] — the fix spec exists because the construction principle wasn't applied fully: JSONL replica (machine-loss durability) and loud write failures (corruption visibility) are convention-dependent gaps that need construction-level fixes.

## Revision Log

- 2026-02-26: Created — metrics computed by `patina scrape`
