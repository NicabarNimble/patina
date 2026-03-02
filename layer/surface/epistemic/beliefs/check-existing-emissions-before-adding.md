---
type: belief
id: check-existing-emissions-before-adding
persona: architect
facets: [measurement, architecture, plugins]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-26
revised: 2026-02-26
---

# check-existing-emissions-before-adding

Check existing emission sources (including WASM plugins) before adding new measure::emit calls — runtime observation catches what static code reading misses in plugin architectures

## Statement

Check existing emission sources (including WASM plugins) before adding new measure::emit calls — runtime observation catches what static code reading misses in plugin architectures

## Evidence

- [[session-20260226-065302]]: Task 3 doctor emission would have duplicated WASM plugin patina-doctor's existing measure.capture event; discovered only by running `patina doctor` and inspecting eventlog, not from Rust source reading (weight: 0.9)

## Supports

- [[measure-reads-tables-not-events]] — both beliefs say: look at runtime data, not just code

## Applied-In

- [[commit-84783746]]: Removed duplicate doctor emission after discovering WASM plugin coverage via `sqlite3 .patina/local/data/patina.db "SELECT ... FROM eventlog WHERE json_extract(data, '$.tool') = 'doctor'"`

## Revision Log

- 2026-02-26: Created — metrics computed by `patina scrape`
