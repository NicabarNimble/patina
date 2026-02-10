---
type: belief
id: anti-tunneling-as-belief-challenge
persona: architect
facets: [epistemic, belief-system, anti-tunneling]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-09
revised: 2026-02-09
---

# anti-tunneling-as-belief-challenge

The anti-tunneling playbook should be a diagnostic lens applied to existing beliefs as they mature, not a gate that prevents belief creation. Beliefs should bubble up with minimal resistance; the system proves and challenges them through their existence — tunneling risk is a computed audit dimension alongside grounding, verification, and use metrics.

## Statement

The anti-tunneling playbook should be a diagnostic lens applied to existing beliefs as they mature, not a gate that prevents belief creation. Beliefs should bubble up with minimal resistance; the system proves and challenges them through their existence — tunneling risk is a computed audit dimension alongside grounding, verification, and use metrics.

## Evidence

- [[session-20260209-075426]]: [[session-20260209-061005]] - Cross-project analysis of marcus/sidecar led to anti-tunneling playbook discussion. Key insight: belief creation should be frictionless, but the system should detect when beliefs are solving the wrong problem (mountain language, assumed constraints, growing complexity in applied-in). This is distinct from verification (which checks if claims are true) — tunneling analysis checks if the belief is necessary. (weight: 0.9)

## Supports

- [[practical-memory-over-epistemic-formalism]]: Low-friction belief creation aligns with practical memory — capture first, formalize later
- [[measure-the-measurement]]: Tunneling risk is a measurement of belief quality, not a gate on belief creation
- [[specs-as-context-sources]]: Tunneling analysis could feed into spec context pipeline as a warning signal

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Without a creation gate, low-quality beliefs accumulate faster than the system can challenge them. Counter: audit warnings already flag "no-evidence" and "floating" beliefs — tunneling risk adds another dimension, not a new problem.

## Applied-In

- **Proposed audit dimension**: `patina belief audit` surfaces tunnel-risk alongside grounding, verification, and use metrics
- **Tunneling signals identified**: mountain language in statement, complexity growth in applied-in, no defeated attacks, assumed constraints unidentified
- **Reference**: Anti-Tunneling Playbook techniques (Reframe Gate, Assumption Ledger, Premortem) as remediation suggestions when tunnel-risk is high

## Revision Log

- 2026-02-09: Created — metrics computed by `patina scrape`
