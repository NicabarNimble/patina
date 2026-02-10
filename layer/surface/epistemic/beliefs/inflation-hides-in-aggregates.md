---
type: belief
id: inflation-hides-in-aggregates
persona: architect
facets: [evaluation, methodology, data-integrity]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-07
revised: 2026-02-07
---

# inflation-hides-in-aggregates

Per-item metric anomalies (like P@10 exceeding 100%) are invisible in aggregates — always sanity-check individual results before trusting summary statistics

## Statement

Per-item metric anomalies (like P@10 exceeding 100%) are invisible in aggregates — always sanity-check individual results before trusting summary statistics

## Evidence

- [[session-20260207-094828]]: [[session-20260207-094828]] - NL eval showed plausible aggregate P@10 of 41.1% while per-query values exceeded 100% (up to 300%). The doc_id double-counting bug inflated lexical-only from 31.1% to 77.2%, completely inverting the fusion-is-harmful narrative. (weight: 0.95)

## Supports

- [[measure-the-measurement]]
- [[andrew-ng-over-shoulder]]

## Attacks

<!-- none yet -->

## Attacked-By

<!-- none yet -->

## Applied-In

- `src/commands/eval/mod.rs` commit [[4772bd20]] — fixed doc_id dedup in P@K calculation
- [[eval-repair]] spec — corrected baseline numbers after bug found
- [[retrieval-tuning]] spec — entire problem statement rewritten with corrected data

## Revision Log

- 2026-02-07: Created — metrics computed by `patina scrape`
