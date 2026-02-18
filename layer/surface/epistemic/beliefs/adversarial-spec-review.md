---
type: belief
id: adversarial-spec-review
persona: architect
facets: [process, spec-driven, quality]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-18
revised: 2026-02-18
---

# adversarial-spec-review

Adversarial spec review before coding catches design gaps that would become bugs — stress-testing exec semantics, function signatures, error handling promises, and algorithm specifics surfaces contradictions and edge cases invisible during implementation

## Statement

Adversarial spec review before coding catches design gaps that would become bugs — stress-testing exec semantics, function signatures, error handling promises, and algorithm specifics surfaces contradictions and edge cases invisible during implementation

## Evidence

- [[session-20260218-162232]]: [[session-20260218-162232]] - spec-launcher-tmux review found 20+ issues across 5 rounds: exec() semantics invalidated fallback promises, pure functions couldn't emit warnings without richer return types, hash algorithms left underspecified, non-UTF-8 paths silently fell back to wrong directory (weight: 0.95)

## Supports

- [[spec-driven-design]] — specs authorize action; adversarial review hardens the authorization
- [[read-code-before-write]] — review is reading before writing, applied to design documents

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Time cost — thorough review can delay implementation start (mitigated: bugs found in spec are cheaper than bugs found in code)

## Applied-In

- [[spec-launcher-tmux]] — 5 review rounds, 20+ issues found before any code written ([[commit-5ac6927c]])

## Revision Log

- 2026-02-18: Created — metrics computed by `patina scrape`
