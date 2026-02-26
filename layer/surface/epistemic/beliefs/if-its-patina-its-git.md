---
type: belief
id: if-its-patina-its-git
persona: architect
facets: [architecture, data-model]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-26
revised: 2026-02-26
---

# if-its-patina-its-git

Patina is git-centric — the source model is always a local git clone, there is no patina project without git

## Statement

Patina is git-centric — the source model is always a local git clone, there is no patina project without git

## Evidence

- [[session-20260226-102315]]: [[session-20260226-102315]] - Established during data-architecture-v2 vision rewrite: git repo is the one source, src/ and layer/ are directories in git not separate sources (weight: 0.9)

## Supports

- [[events-are-autobiography-not-telemetry]] — git as sole source makes events.db the only non-git-derived data, reinforcing its irreplaceability

## Attacks

## Attacked-By

## Applied-In

- [[spec-data-architecture-v2]] § Source Model — "the source is always a local git clone"
- `src/commands/scrape/` — all scrapers parse from git working tree or .git/ history

## Revision Log

- 2026-02-26: Created — metrics computed by `patina scrape`
