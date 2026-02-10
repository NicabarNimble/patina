---
type: belief
id: andrew-ng-over-shoulder
persona: architect
facets: [evaluation, methodology, quality-gate]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-07
revised: 2026-02-07
---

# andrew-ng-over-shoulder

Before shipping any metric-driven change, ask: is the measurement truthful? Is the improvement real or overfitting? Would this survive a held-out test? If you cannot answer yes to all three, fix the measurement first.

## Statement

Before shipping any metric-driven change, ask: is the measurement truthful? Is the improvement real or overfitting? Would this survive a held-out test? If you cannot answer yes to all three, fix the measurement first.

## Evidence

- [[session-20260207-094828]]: [[session-20260207-094828]] - Weight tuning appeared to improve NL P@10 by 18pp but was chasing inflated metrics from a precision bug. User intervention caught the issue before shipping. Three-question checklist formalized as a gate for all future metric-driven changes. (weight: 0.95)

## Supports

- [[measure-first]]
- [[measure-the-measurement]]

## Attacks

<!-- none yet -->

## Attacked-By

<!-- none yet -->

## Applied-In

- [[retrieval-tuning]] spec — pre-requisites section gates all tuning on three-question checklist
- Session [[20260207-094828]] — weight changes reverted after failing the checklist

## Revision Log

- 2026-02-07: Created — metrics computed by `patina scrape`
