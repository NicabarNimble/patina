---
type: belief
id: git-tags-as-knowledge-refs
persona: architect
facets: [architecture, git, database, protocol]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-15
revised: 2026-02-15
---

# git-tags-as-knowledge-refs

Git tags are Patina's knowledge refs. Session tags and release tags already mark knowledge state boundaries. Instead of building custom ref/snapshot infrastructure, build a tag-aware database — SQLite tables that index git tags and use git show to read historical file state. This gives patina diff, session provenance, and knowledge snapshots without any filesystem changes.

## Statement

Git tags are Patina's knowledge refs. Session tags and release tags already mark knowledge state boundaries. Instead of building custom ref/snapshot infrastructure, build a tag-aware database — SQLite tables that index git tags and use git show to read historical file state. This gives patina diff, session provenance, and knowledge snapshots without any filesystem changes.

## Evidence

- [[session-20260215-075638]]: knowledge-protocol Outcome C proved every proposed ref/snapshot service maps to existing git tags. Patina already creates session-TIMESTAMP and vX.Y.Z tags. The gap is a database layer that links these tags to belief/pattern state and can answer historical queries.

## Supports

- [[git-is-the-knowledge-substrate]] — tags are git's native ref mechanism
- [[knowledge-diff-is-a-command-not-a-substrate]] — diff reads from tags via git show
- [[beliefs-are-the-product]] — tag-aware DB makes belief history queryable, improving the product

## Attacks

<!-- none -->

## Attacked-By

<!-- none yet -->

## Applied-In

- Session tags (`session-TIMESTAMP-start/end`) already mark session boundaries
- Release tags (`vX.Y.Z`) already mark release boundaries
- Not yet wired into a queryable database — candidate for implementation

## Revision Log

- 2026-02-15: Created — metrics computed by `patina scrape`
