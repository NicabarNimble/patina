---
type: belief
id: vocabulary-drift-compounds
persona: architect
facets: [architecture, naming, process, maintenance]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-03
revised: 2026-04-03
---

# vocabulary-drift-compounds

Vocabulary drift compounds like tech debt — when a name no longer matches what a thing is, the mismatch spreads through code, config, docs, and session artifacts, and the rename cost grows with every file touched.

## Statement

Vocabulary drift compounds like tech debt — when a name no longer matches what a thing is, the mismatch spreads through code, config, docs, and session artifacts, and the rename cost grows with every file touched.

## Evidence

- [[session-20260403-070944-045859000]] - child-rename (26 commits, 116 files) and adapter-to-interface rename both caused by names that outlived their concepts. The longer the drift, the larger the refactor surface. (weight: 0.9)

## Supports

- [[stale-context-is-hostile-context]] — vocabulary drift is a form of stale context; the name lies about what the thing is
- [[canonical-module-bypass-compounds]] — when the canonical name drifts, callers invent ad-hoc alternatives

## Attacks

## Attacked-By

- [[core-principles-contain-blast-radius]] — principles make renames survivable, but can't prevent the need for them

## Applied-In

- child-rename spec (26 commits, 116 files): `KnowledgeChild` → `Child`, `KnowledgeChildEngine` → `ChildEngine`
- adapter→interface rename (in progress): `ClaudeAdapter` → `ClaudeInterface`, `patina adapter` → `patina interface`
- ducklake-retirement: "ducklake" name persisted in test fixtures and state stores after the concept was retired

## Revision Log

- 2026-04-03: Created — metrics computed by `patina scrape`
