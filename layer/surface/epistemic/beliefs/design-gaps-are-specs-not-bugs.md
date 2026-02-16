---
type: belief
id: design-gaps-are-specs-not-bugs
persona: architect
facets: [process, spec-driven, methodology]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-16
revised: 2026-02-16
---

# design-gaps-are-specs-not-bugs

When real-world testing reveals a scenario the spec didn't account for, that's a design gap not a bug — write a spec amendment before coding the fix

## Statement

When real-world testing reveals a scenario the spec didn't account for, that's a design gap not a bug — write a spec amendment before coding the fix

## Evidence

- [[session-20260216-155323]]: Mother sync per-source rebuild was a design gap (global rebuild vs per-source was never considered in cross-project-beliefs SPEC), but was treated as a bug fix and coded directly, violating spec-driven-design (weight: 0.95)

## The Test

"Did the spec intend this behavior?"
- **Yes, and it's broken** → bug fix, code directly
- **The spec never considered it** → design gap → spec amendment first

## Supports

- [[spec-driven-design]] — specs are the authority; skipping them for "urgent fixes" erodes the pattern
- [[specs-require-zero-ambiguity]] — a design gap is proof the spec had ambiguity

## Attacks

## Attacked-By

- Velocity pressure: "it's a small fix, just ship it" — but changing a function's contract (new parameter, different semantics) is never small

## Applied-In

- `sync_knowledge()` signature change (2026-02-16): added `synced_sources` parameter, changed from global DELETE to per-source DELETE. Should have been a spec amendment to [[cross-project-beliefs]] SPEC before coding.

## Revision Log

- 2026-02-16: Created — metrics computed by `patina scrape`
