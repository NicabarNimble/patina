---
type: belief
id: git-is-the-knowledge-substrate
persona: architect
facets: [architecture, protocol, git, beliefs]
entrenchment: high
status: active
endorsed: true
extracted: 2026-02-15
revised: 2026-02-15
---

# git-is-the-knowledge-substrate

Git is Patina's knowledge substrate — don't build a second content-addressed layer on top. Belief files in a git repo already have hashing, history, diffing, branching, and snapshots. The protocol is the pipeline (scrape/oxidize/scry), not the storage layout.

## Statement

Git is Patina's knowledge substrate — don't build a second content-addressed layer on top. Belief files in a git repo already have hashing, history, diffing, branching, and snapshots. The protocol is the pipeline (scrape/oxidize/scry), not the storage layout.

## Evidence

- [[session-20260215-075638]]: Read all belief write/read/scrape code paths. Content addressing fails because belief identity is the slug not the hash. Every proposed object/ref/snapshot service maps to an existing git primitive. Explored in knowledge-protocol spec, Outcome C.

## Supports

- [[belief-identity-is-slug-not-hash]] — the slug/hash distinction is why CAS fails here
- [[patina-is-knowledge-protocol]] — the protocol is the pipeline, git is the substrate

## Attacks

- [[patina-is-knowledge-layer]] — the "git-style substrate" framing led to the CAS proposal; the substrate IS git, not something built on git

## Attacked-By

- [[beliefs-are-entities-not-documents]] (status: active, scope: "entities need richer identity than files" — tension exists but git+slug handles it)

## Applied-In

- knowledge-protocol explore spec — closed as Outcome C
- Session tags and release tags already serve as knowledge refs

## Revision Log

- 2026-02-15: Created — metrics computed by `patina scrape`
