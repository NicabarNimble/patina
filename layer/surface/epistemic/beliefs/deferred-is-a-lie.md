---
type: belief
id: deferred-is-a-lie
persona: architect
facets: [process, specs, workflow]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-06
revised: 2026-02-06
---

# deferred-is-a-lie

Deferred is a lie — every spec is either done, exploring, honestly blocked, or should be archived. Limbo states accumulate debt.

## Statement

Deferred is a lie — every spec is either done, exploring, honestly blocked, or should be archived. Limbo states accumulate debt.

## Evidence

- [[session-20260206-060219]]: Spec audit found 18 specs in deferred/, many superseded or shipped but never closed. Established rule: "Deferred should be eliminated as a limbo state." (weight: 0.9)
- [[session-20260206-122524]]: Triaged all 15 remaining deferred specs. Every one had a clear honest destination (complete/archive/explore/blocked feat). The deferred/ directory was deleted entirely. (weight: 1.0)
- Observation: 4 specs had been completed months ago but never moved out of deferred/ (database-identity, ref-repo-semantic phases, spec-spec-as-skill). Limbo masked real progress. (weight: 0.7)

## Supports

- [[spec-first]] — specs must have honest status, not euphemistic parking
- [[measure-the-measurement]] — "deferred" avoids the measurement of whether work is valuable

## Attacks

- Notion that "deferred" is a useful triage category — it isn't, it's procrastination with a label

## Attacked-By

- Pragmatism: sometimes you genuinely don't know what to do with a spec yet (counter: then it's "explore", which is honest about uncertainty)

## Applied-In

- Session [[session-20260206-122524]]: eliminated deferred/ entirely. 15 specs → 3 complete, 4 archived, 5 explore, 3 feat (2 blocked, 1 ready).
- Established 5 honest destinations: complete, archive, explore, blocked feat, ready feat. No "deferred" needed.

## Revision Log

- 2026-02-06: Created — metrics computed by `patina scrape`
