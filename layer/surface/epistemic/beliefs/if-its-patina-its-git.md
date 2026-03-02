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

Git is the source of record for what a project declares. events.db is the source of record for what a project experiences. Together they are the complete truth. Everything else is derived.

## Statement

Git is the source of record for what a project declares. events.db is the source of record for what a project experiences. Together they are the complete truth. Everything else is derived.

## Evidence

- [[session-20260226-102315]]: Established during data-architecture-v2 vision rewrite: git repo is the one source, src/ and layer/ are directories in git not separate sources (weight: 0.9)
- [[session-20260226-124149]]: Refined through Kleppmann/Schickling/Jahns analysis. The original "patina is git" was an oversimplification — Patina produces lived experience (search behavior, belief lifecycle, decision moments) that doesn't exist in git. events.db is a genuine second source of truth, not a materialization of something in git. Schickling's framing: SQLite is the runtime, JSONL is a replica for disaster recovery. (weight: 0.95)

## Supports

- [[events-are-autobiography-not-telemetry]] — events.db as second source of truth reinforces its irreplaceability: the autobiography can't be derived from git

## Attacks

## Attacked-By

## Applied-In

- [[spec-data-architecture-v2]] § Source Model — two sources of truth: git (declarations) + events.db (experience)
- `src/commands/scrape/` — all scrapers parse from git working tree or .git/ history
- [[spec-data-db-split]] — the physical separation enforces the two-source-of-truth model

## Revision Log

- 2026-02-26: Created — "patina is git-centric, the source model is always a local git clone"
- 2026-02-26: Revised — refined from "git is the one source" to "git + events.db are the two sources of truth." Original framing was an oversimplification that didn't account for runtime knowledge (search behavior, belief lifecycle, decisions) that events.db captures and git cannot. Schickling-aligned: events.db is a real source, JSONL is a replica.
