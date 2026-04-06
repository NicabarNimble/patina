---
type: belief
id: test-against-installed-binary
persona: architect
facets: [testing, migration, operations]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-06
revised: 2026-04-06
---

# test-against-installed-binary

Test migrations against the installed release binary, not the debug build. The debug binary in the project directory is not what other projects or sessions use.

## Statement

Test migrations against the installed release binary, not the debug build. The debug binary in the project directory is not what other projects or sessions use.

## Evidence

- [[session-20260405-133644]] - Removed serde aliases after confirming migration on debug binary. Other projects broke because the installed release binary was still old. Had to restore aliases and cargo install. (weight: 1.0)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-04-06: Created — metrics computed by `patina scrape`
