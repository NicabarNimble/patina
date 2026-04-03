---
type: belief
id: core-principles-contain-blast-radius
persona: architect
facets: [architecture, process, core-principles]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-03
revised: 2026-04-03
---

# core-principles-contain-blast-radius

Core principles (unix-philosophy, dependable-rust, adapter-pattern) make architectural evolution survivable, not unnecessary. They shrink the blast radius of change but cannot prevent the need for change when you learn something fundamental.

## Statement

Core principles (unix-philosophy, dependable-rust, adapter-pattern) make architectural evolution survivable, not unnecessary. They shrink the blast radius of change but cannot prevent the need for change when you learn something fundamental.

## Evidence

- [[session-20260403-070944-045859000]] - Weeks of refactors (ducklake-retirement, child-rename, vocabulary-alignment, federation redesign) happened despite strong core principles. But each refactor was contained: child-rename was 26 commits with no breakage because the public interface was small. The principles worked — they just don't prevent learning. (weight: 0.95)

## Supports

- [[dependable-rust]] — black-box modules make replacement possible without breaking callers
- [[vocabulary-drift-compounds]] — principles contain the blast radius of renames but can't prevent the need

## Attacks

## Attacked-By

- "If principles worked, why weeks of refactors?" — because principles scope the change, not prevent the learning that triggers it

## Applied-In

- child-rename: `dependable-rust` kept the public interface small, so 116 files changed but zero breakage
- ducklake-retirement: adapter-pattern trait boundary meant removing ducklake was scoped to one implementation, not the whole query layer
- federation redesign: unix-philosophy decomposition meant retiring one component didn't cascade

## Revision Log

- 2026-04-03: Created — metrics computed by `patina scrape`
