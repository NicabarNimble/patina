---
type: belief
id: kamp-lens-real-implementations
persona: architect
facets: [testing, engineering, design-philosophy]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-03
revised: 2026-03-03
---

# kamp-lens-real-implementations

No mocks unless strictly necessary. Real implementations preferred. Tests live near code. The build-and-test cycle should exercise the actual binary, not a simulated version of it. If your test suite passes but the binary doesn't work, your tests are theater.

## Statement

No mocks unless strictly necessary. Real implementations preferred. Tests live near code. The build-and-test cycle should exercise the actual binary, not a simulated version of it. If your test suite passes but the binary doesn't work, your tests are theater.

## Evidence

- [[session-20260303-101839]]: Formalized from Poul-Henning Kamp's engineering philosophy. Patina enforces this directly: CLAUDE.md mandates 'cargo build --release && cargo install --path . && patina <command>' for all testing. The measure system tests against real scrape output, not mocked databases. pre-push-checks.sh runs the actual binary. (weight: 0.9)

## Supports

- [[measure-is-ambient-health-for-llm-context]] — measure uses real scrape output, not mocked data

## Attacks

## Attacked-By

## Applied-In

- CLAUDE.md testing rule: `cargo build --release && cargo install --path . && patina <command>`
- `resources/git/pre-push-checks.sh` — runs the actual binary, not test harnesses
- 37 measure tests in `src/commands/measure/` test against real metric structures

## Revision Log

- 2026-03-03: Created — metrics computed by `patina scrape`
