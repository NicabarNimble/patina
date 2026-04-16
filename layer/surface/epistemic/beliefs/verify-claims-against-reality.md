---
type: belief
id: verify-claims-against-reality
persona: architect
facets: [verification, trust, agents]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-10
revised: 2026-04-10
---

# verify-claims-against-reality

When another actor reports work as complete, verify against the actual state of the system before accepting it — reports are claims, the underlying code, tests, and artifacts are the truth.

## Statement

When another actor reports work as complete, verify against the actual state of the system before accepting it — reports are claims, the underlying code, tests, and artifacts are the truth.

## Evidence

- [[session-20260409-143847-707078000]] - A build agent reported all tests passing after a refactor; verification found one test was actually timing out at the test runner's default threshold while being reported as passed in the agent's summary, requiring a config fix to surface (weight: 0.9)
- [[commit-5d32df19]] - Test runner timeout configuration was extended after verification revealed a slow test was being terminated rather than passing as reported
- A spec was reported as already completed in earlier work but had zero acceptance criteria checked off; verification against the actual code found 8 of 11 criteria were met, the rest were real gaps

## Supports

- [[ground-every-assertion]] — assertions are claims until grounded in evidence; verification is the grounding step
- [[spec-driven-design]] — specs are contracts; verifying that work meets the contract is part of honoring it

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- "Trust your collaborators." Attack: verification implies distrust. Defeat: verification is not distrust, it is the discipline of treating reports as data that must be reconciled with the system's actual state. Trustworthy collaborators welcome verification because it strengthens the joint work.

## Applied-In

- After a build agent reported pando execution gaps closed, the full test suite was run to confirm — and revealed a test timing out at the runner's default threshold rather than passing. The fix was a configuration update that would not have been discovered without independent verification.
- After a build agent reported a library SDK refactor complete, structural verification of the resulting crate state confirmed all the structural claims (renames, alias wiring, file moves) but the test suite run surfaced the slow-test issue independently.

## Revision Log

- 2026-04-10: Created — metrics computed by `patina scrape`
