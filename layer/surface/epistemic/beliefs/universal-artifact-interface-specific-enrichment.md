---
type: belief
id: universal-artifact-interface-specific-enrichment
persona: architect
facets: [sessions, architecture, interfaces]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-20
revised: 2026-03-20
---

# universal-artifact-interface-specific-enrichment

Session artifacts have a universal format (same YAML frontmatter, same markdown body, same git tags) but each interface is responsible for how that artifact gets created and enriched based on its own persistence model

## Statement

Session artifacts have a universal format (same YAML frontmatter, same markdown body, same git tags) but each interface is responsible for how that artifact gets created and enriched based on its own persistence model

## Evidence

- [[session-20260320-075256-088035000]] - session audit of 35 sessions revealed interface-specific quality gaps: Claude sessions need full artifact as continuity mechanism, OpenCode has native SQLite persistence (weight: 0.9)

## Supports

- [[durability-lives-outside-interface-process]] — universal artifact is what the child writes to; interface-specific enrichment is how each child populates it
- [[session-as-interface-agnostic-work-record]] — prior belief from [[session-20260318-221008-061837000]]

## Attacks

- [[session-system-needs-multi-interface-redesign]] — this belief IS the redesign direction, replacing the "needs redesign" problem statement

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[spec-interface-session-model]] — explore spec built around this principle (Thread 2: Interface-specific artifact enrichment)

## Revision Log

- 2026-03-20: Created — metrics computed by `patina scrape`
