---
type: belief
id: belief-identity-is-slug-not-hash
persona: architect
facets: [belief-system, identity, architecture]
entrenchment: high
status: active
endorsed: true
extracted: 2026-02-15
revised: 2026-02-15
---

# belief-identity-is-slug-not-hash

Belief identity is the human-readable slug, not a content hash. Unlike git objects where changing a byte creates a new identity, beliefs are meant to be mutable — evidence accumulates, entrenchment shifts, relationships form — while the slug remains stable. Content hashing creates meaningless object churn for knowledge artifacts.

## Statement

Belief identity is the human-readable slug, not a content hash. Unlike git objects where changing a byte creates a new identity, beliefs are meant to be mutable — evidence accumulates, entrenchment shifts, relationships form — while the slug remains stable. Content hashing creates meaningless object churn for knowledge artifacts.

## Evidence

- [[session-20260215-075638]]: Analyzed sync-first belief: statement rarely changes but evidence/entrenchment/relationships change constantly. A hash of the assertion changes on rewording (same belief, new hash). A hash of the full file changes on every evidence addition. Neither is useful as identity. The slug handles this correctly already.

## Supports

- [[git-is-the-knowledge-substrate]] — if slug is identity, git's file-level CAS is sufficient
- [[beliefs-are-entities-not-documents]] — entities need stable identity across mutations

## Attacks

<!-- none -->

## Attacked-By

<!-- none yet -->

## Applied-In

- knowledge-protocol explore — this insight was the decisive factor in choosing Outcome C
- Current belief format (`layer/surface/epistemic/beliefs/{slug}.md`) validated as correct

## Revision Log

- 2026-02-15: Created — metrics computed by `patina scrape`
