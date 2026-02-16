---
type: belief
id: specs-require-zero-ambiguity
persona: architect
facets: [specs, governance, process]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-16
revised: 2026-02-16
---

# specs-require-zero-ambiguity

Specs must be hardened against code reality until an implementer can execute without deviation — every assertion grounded, every edge case decided, every judgment call pre-made. An ambiguous spec shifts decision-making from the author (who has context) to the implementer (who doesn't).

## Statement

Specs must be hardened against code reality until an implementer can execute without deviation — every assertion grounded, every edge case decided, every judgment call pre-made. An ambiguous spec shifts decision-making from the author (who has context) to the implementer (who doesn't).

## Evidence

- [[session-20260216-064229]]: [[belief-truthfulness]] spec had impossible drift detection approach — `belief_verifications` table is DROP+CREATE every scrape, but spec said "compare against data_freshness". Ungrounded assertion passed initial review, caught only by deep code read (weight: 0.95)
- [[session-20260216-064229]]: 10 implementation concerns surfaced in review — staleness definition drift between Phase A/B, session filename edge cases, transaction safety, drift reset semantics. Each was a judgment call the implementer would have had to make without context (weight: 0.9)
- [[session-20260215-083121]]: 21 specs archived to clear pipeline — many had accumulated without reaching implementation-ready state, evidence that specs without hardening stall (weight: 0.7)

## Supports

- [[ground-assertions-or-pay-review-tax]] — grounding is one mechanism for achieving zero ambiguity
- [[spec-driven-design]] — specs as authority requires they be unambiguous
- [[spec-is-contract]] — contracts with ambiguous terms invite disputes
- [[truthful-specs]] — truthful specs must reflect code reality, not aspirational design

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Diminishing returns: hardening a spec has cost; at some point the last 5% of ambiguity removal may not justify the effort. Counter: the cost of ambiguity compounds at implementation — one missed edge case can cascade into rework.

## Applied-In

- [[belief-truthfulness]] SPEC.md — 3 amendment commits ([[commit-d333ab03]], [[commit-a2f62729]], [[commit-0014de8c]]) hardened the spec from "mostly right" to "zero judgment calls", adding 20 evidence claims, 6 unit tests, verification plan, and crash recovery semantics

## Revision Log

- 2026-02-16: Created — metrics computed by `patina scrape`
