---
type: belief
id: standards-are-storage-coordination-sits-above
persona: architect
facets: [architecture, data, storage, coordination]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-30
revised: 2026-03-30
---

# standards-are-storage-coordination-sits-above

Portable universal standards are the storage foundation; coordination layers sit above, querying and managing across many instances without replacing what is underneath — SQLite to DuckDB as Git to jj.

## Statement

Portable universal standards are the storage foundation; coordination layers sit above, querying and managing across many instances without replacing what is underneath — SQLite to DuckDB as Git to jj.

## Evidence

- [[session-20260330-083255-177610000]] - Derived from database redesign discussion: Mother owns databases (SQLite per-project), DuckDB federates across them. Same pattern recognized in Git (history unit) vs jj (concurrent interface coordination). The lower layer is maximally portable and tool-agnostic; the upper layer adds coordination without replacing the foundation. (weight: 0.95)

## Supports

- [[wasi-is-foundation-not-option]] — WASI is the standard interface layer; Patina's custom toys cover only the delta above it. Same pattern: standard foundation, coordination above.
- [[observation-at-the-boundary]] — boundary observation works because each layer is self-contained; the coordination layer (Mother) observes the storage layer (children) at the interface, never inside it.

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- Database redesign direction: SQLite per-project (portable storage unit), DuckDB as Mother's cross-project federation query layer (coordination). Neither replaces the other.
- VCS direction: Git remains the history foundation (committed to `patina` branch convention, session tags). jj would sit above as concurrent-interface coordination without replacing git.
- Existing precedent: `read_parquet()` in DuckDB queries parquet files produced by children — DuckDB coordinates across child-produced storage without owning it.

## Revision Log

- 2026-03-30: Created — metrics computed by `patina scrape`
