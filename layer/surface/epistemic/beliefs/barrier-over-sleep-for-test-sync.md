---
type: belief
id: barrier-over-sleep-for-test-sync
persona: architect
facets: [testing, concurrency, rust]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-31
revised: 2026-03-31
---

# barrier-over-sleep-for-test-sync

Test synchronization should use deterministic primitives (Barrier, channels, condvars) instead of timing-based sleeps; sleep-based coordination creates flaky tests that fail under load or on different hardware

## Statement

Test synchronization should use deterministic primitives (Barrier, channels, condvars) instead of timing-based sleeps; sleep-based coordination creates flaky tests that fail under load or on different hardware

## Evidence

- [[session-20260331-080327-949611000]] - G6 drain test shutdown_flag_drains_in_flight_requests failed 4/5 runs with 25ms sleep, fixed to 10/10 passes with std::sync::Barrier handshake (weight: 0.95)

## Supports

- [[dependable-rust]] — deterministic tests are part of a dependable interface; flaky tests erode trust in the test suite

## Attacks

## Attacked-By

## Applied-In

- `mother/src/http_daemon.rs` — `shutdown_flag_drains_in_flight_requests` test uses `std::sync::Barrier::new(2)` to synchronize handler start with shutdown flag set ([[commit-c79707c7]])
- `mother/src/http_daemon.rs` — `send_request` helper hardened with non-fatal `shutdown(Write)` to tolerate server-side close races ([[commit-0e52122e]])

## Revision Log

- 2026-03-31: Created — metrics computed by `patina scrape`
