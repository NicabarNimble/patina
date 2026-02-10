---
type: belief
id: never-tune-on-eval
persona: architect
facets: [evaluation, methodology, ml-discipline]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-07
revised: 2026-02-07
---

# never-tune-on-eval

Never tune parameters against the same data you evaluate on — require train/test splits before any weight or threshold changes

## Statement

Never tune parameters against the same data you evaluate on — require train/test splits before any weight or threshold changes

## Evidence

- [[session-20260207-094828]]: [[session-20260207-094828]] - Attempted tuning 25 intent weights against 25 NL test queries (1:1 params to data points). Achieved apparent P@10 improvement (41.1% to 59.4%) that was both inflated by a measurement bug and statistically meaningless. Reverted. (weight: 0.95)

## Supports

- [[measure-the-measurement]]
- [[andrew-ng-over-shoulder]]

## Attacks

<!-- none yet -->

## Attacked-By

<!-- none yet -->

## Applied-In

- [[retrieval-tuning]] spec — blocked Phase 2 (intent weighting) on held-out test set pre-requisite
- `src/retrieval/intent.rs` — weight changes reverted, comment documents the guard

## Revision Log

- 2026-02-07: Created — metrics computed by `patina scrape`
