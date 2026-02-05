---
type: belief
id: capability-not-optimization
persona: architect
facets: [design, framing, product]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-04
revised: 2026-02-04
---

# capability-not-optimization

Frame features as capabilities, not optimizations — the real value of a change is often the new thing you can do, not the metric it improves.

## Statement

Frame features as capabilities, not optimizations — the real value of a change is often the new thing you can do, not the metric it improves.

## Evidence

- [[session-20260204-142556]]: D3 two-step retrieval: 'fewer tokens' was the spec framing, 'scan-then-focus' was the actual value — reframing changed nothing in code but guided user understanding and adoption (weight: 0.90)

## Supports

- [[measure-first]] — measuring the right thing depends on framing the feature correctly first
- [[spec-challenge-traceback]] — reframing a feature mid-implementation is a form of spec traceback

## Attacks

- [[optimize-for-metrics]] (status: scoped, reason: "metrics matter, but only after the capability is correctly identified")

## Attacked-By

- [[metrics-drive-adoption]] (status: active, confidence: 0.3, scope: "token savings IS the value for cost-constrained LLM consumers")

## Applied-In

- D3 two-step retrieval — spec said "token efficiency", user reframed to "scan-then-focus", which revealed `--detail` as the real new capability
- D1 BeliefOracle — spec said "fix -0.05 delta", but the real value was beliefs surfacing in default search alongside code

## Revision Log

- 2026-02-04: Created — metrics computed by `patina scrape`
