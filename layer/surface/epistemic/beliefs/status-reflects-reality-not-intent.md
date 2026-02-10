---
type: belief
id: status-reflects-reality-not-intent
persona: architect
facets: [specs, process, integrity]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-07
revised: 2026-02-07
---

# status-reflects-reality-not-intent

A spec's status must describe what IS happening, not what SHOULD happen. Active means someone is actively building it right now, not that it's a good idea worth building someday.

## Statement

A spec's status must describe what IS happening, not what SHOULD happen. Active means someone is actively building it right now, not that it's a good idea worth building someday.

## Evidence

- [[session-20260207-073728]]: 8 specs marked `active` that nobody was building polluted `patina spec ready` with false signal. Changed all to `design`. (weight: 0.95)
- [[session-20260206-163435]]: Previous session identified "active explores lie" as a pattern — 8 specs marked active in the ready queue but none being built. (weight: 0.8)

## Supports

- [[stale-context-is-hostile-context]] — aspirational status is a form of stale context that misleads LLMs
- [[deferred-is-a-lie]] — same pattern: status labels that don't reflect reality accumulate as lies

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[commit-97be84ae]]: adapter-polish, llm-adapter-refactor `active` → `design`
- [[commit-e37a3b52]]: observability `active` → `design`
- [[commit-f3828074]]: skill-derive `active` → `design`
- [[commit-82206626]]: wit-interfaces, cli-commands `active` → `design`
- [[commit-1db9b1fa]]: lab-automation `active` → `design`
- [[commit-3b89d8dc]]: three-layers `active` → `design`

## Revision Log

- 2026-02-07: Created — metrics computed by `patina scrape`
