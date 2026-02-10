---
type: belief
id: intrusive-knowledge-graph
persona: architect
facets: [architecture, belief-system, data-structures]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-09
revised: 2026-02-09
---

# intrusive-knowledge-graph

Knowledge graph relationships (supports, attacks, evidence links) should be intrusive to the knowledge artifacts themselves, not managed by an external index. Artifacts become self-describing and portable — when you move a belief, its connections move with it.

## Statement

Knowledge graph relationships (supports, attacks, evidence links) should be intrusive to the knowledge artifacts themselves, not managed by an external index. Artifacts become self-describing and portable — when you move a belief, its connections move with it.

## Evidence

- [[session-20260209-120229]]: [[session-20260209-061005]] - LOATs (Large Arrays of Things) stream analysis: intrusive data structures put next_sibling and parent inside the Thing itself rather than in an external container. Patina beliefs already do this — Supports/Attacks/Attacked-By/Evidence sections are intrusive links embedded in the markdown file. The graph topology lives in the artifacts, not in a separate relationship database. This is a deliberate design choice worth naming. (weight: 0.9)

## Supports

- [[beliefs-are-entities-not-documents]]: Intrusive links make beliefs discoverable entities, not just documents to be searched
- [[anti-tunneling-as-belief-challenge]]: Attacked-By as an intrusive link means the challenge mechanism lives inside the belief, not in an external validation system

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Intrusive links duplicate information — if belief A supports belief B, that fact lives in A's file but B doesn't know about it until scrape cross-references. Counter: this is the same tradeoff as intrusive lists — the scraper materializes the reverse index, but the source of truth is always the artifact itself.

## Applied-In

- **Belief files**: `## Supports`, `## Attacks`, `## Attacked-By`, `## Evidence` with `[[wikilinks]]` are all intrusive links — the graph topology is embedded in the markdown
- **Session files**: `sessions.origin` in spec frontmatter, git tags bracketing sessions — provenance links are intrusive to the artifacts
- **Spec files**: `beliefs:` and `related:` frontmatter fields link specs to beliefs and other specs intrusively
- **Contrast with external-index approach**: A triple store or relationship DB would manage links externally — but then moving/archiving an artifact breaks its connections

## Revision Log

- 2026-02-09: Created — metrics computed by `patina scrape`
