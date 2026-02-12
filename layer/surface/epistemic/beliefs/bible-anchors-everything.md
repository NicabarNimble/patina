---
type: belief
id: bible-anchors-everything
persona: architect
facets: [governance, architecture, identity, specs]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-11
revised: 2026-02-11
---

# bible-anchors-everything

A core identity document eliminates architectural ambiguity that individual specs cannot — without it, frozen specs are a reference library with no index; with it, every decision traces to 'does this serve a core function?'

## Statement

A core identity document eliminates architectural ambiguity that individual specs cannot — without it, frozen specs are a reference library with no index; with it, every decision traces to 'does this serve a core function?'

## Evidence

- [[session-20260211-125648]]: 29 frozen specs created governance fog — no one could draw the core/plugin boundary. The bible ([[patina-identity]]) resolved it in one document by defining 6 core functions and a 4-question plugin test. The [[plugin-system]] spec references patina-identity for every boundary decision. (weight: 0.9)
- [[session-20260211-121154]]: Even the AI couldn't draw the core/plugin boundary when asked — classifying mother, secrets, adapters, models as "gray area" until the user corrected it. The bible eliminated the ambiguity by defining pillars, not modules. (weight: 0.8)
- [[session-20260211-121154]]: Hard-freezing 29 specs was necessary but insufficient — freeze stops the bleeding but doesn't answer "what is core?" The bible answers that question, making the freeze actionable. (weight: 0.7)

## Supports

- [[spec-driven-design]] — the bible is the meta-spec: it authorizes what kinds of specs can exist and what pillar they serve
- [[work-triages-specs]] — the bible provides the index that lets the build consume frozen specs selectively
- [[transparent-complexity]] — 29 specs was invisible complexity; the bible makes the structure visible

## Attacks

<!-- No beliefs defeated yet -->

## Attacked-By

- Over-documentation: a single identity document can become stale if not updated as the system evolves. The bible must be a living document, not a snapshot.
  - Status: acknowledged — mitigated by the bible being a core pattern (layer/core/), reviewed alongside dependable-rust and unix-philosophy

## Applied-In

- [[patina-identity]] (`layer/core/patina-identity.md`): the bible itself — 6 pillars, 22 core modules, 5 plugin extractions, 8 invariants
- [[plugin-system]] spec: every boundary decision references the 4-question plugin test from the bible

## Revision Log

- 2026-02-11: Created — metrics computed by `patina scrape`
