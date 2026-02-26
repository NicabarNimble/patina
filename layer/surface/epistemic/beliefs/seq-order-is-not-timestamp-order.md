---
type: belief
id: seq-order-is-not-timestamp-order
persona: architect
facets: [eventlog, sqlite, data-integrity]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-25
revised: 2026-02-25
---

# seq-order-is-not-timestamp-order

Eventlog seq order is not timestamp order — git.commit events are inserted newest-first from git log, so ORDER BY seq DESC returns the oldest commit, not the newest. Any query assuming seq correlates with chronological time is wrong.

## Statement

Eventlog seq order is not timestamp order — git.commit events are inserted newest-first from git log, so ORDER BY seq DESC returns the oldest commit, not the newest. Any query assuming seq correlates with chronological time is wrong.

## Evidence

- [[session-20260225-221415]]: [[session-20260225-221415]] - Found two bugs in measure capture: ORDER BY seq DESC returned oldest git.commit (Jul 2025 instead of Feb 2026), and seq-based windowing for files_tracked selected oldest 100 commits instead of newest 100. Fixed in [[commit-a62bd24d]]. (weight: 0.95)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `src/commands/measure/internal.rs` — changed git.commit latest_ts from `ORDER BY seq DESC LIMIT 1` to `MAX(timestamp)`, and files_tracked from seq-based windowing to timestamp-based windowing

## Revision Log

- 2026-02-25: Created — metrics computed by `patina scrape`
