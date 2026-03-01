---
type: belief
id: enum-not-string-for-finite-states
persona: architect
facets: [rust, type-safety, domain-modeling]
entrenchment: high
status: active
endorsed: true
extracted: 2026-03-01
revised: 2026-03-01
---

# enum-not-string-for-finite-states

When a value has a finite set of known states, use an enum, not `String`. `status: String` compiles even when you typo "actve" — `Status::Active` does not. The compiler enforces exhaustive matching and catches invalid transitions at build time.

## Statement

When a value has a finite set of known states, use an enum, not `String`. `status: String` compiles even when you typo "actve" — `Status::Active` does not. The compiler enforces exhaustive matching and catches invalid transitions at build time.

## Evidence

- [[session-20260301-165723]]: Structural audit found 13 files using `status: String` where enums should be used. The three most critical: spec status (7 states: draft/ready/active/paused/blocked/complete/abandoned), belief status (4 states: active/scoped/defeated/archived), layer task status (3 states: pending/in_progress/complete). (weight: 0.95)
- [[session-20260301-165723]]: Two `LanguageInfo` structs with the same name but completely different domains (code profiling vs environment detection) — a naming collision that typed enums/newtypes would prevent. (weight: 0.8)

## Supports

- [[correctness-by-construction-not-convention]] — enums make invalid states unrepresentable by construction
- [[parse-at-boundary-type-the-interior]] — string status parsed from YAML/JSON should become an enum at the parse boundary

## Attacks

<!-- None known -->

## Attacked-By

- Serde round-tripping: enum variants must match YAML/JSON string values exactly — mitigated by `#[serde(rename_all = "snake_case")]` which handles this automatically

## Applied-In

<!-- Not yet applied — requires spec work -->

## Revision Log

- 2026-03-01: Created from structural audit findings
