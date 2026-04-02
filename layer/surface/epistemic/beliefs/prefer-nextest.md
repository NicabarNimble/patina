---
type: belief
id: prefer-nextest
persona: architect
facets: [rust, testing, ci]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-29
revised: 2026-03-29
---

# prefer-nextest

Use cargo-nextest as the default test runner — per-process isolation eliminates shared-state races and enables parallel execution without code changes

## Statement

Use cargo-nextest as the default test runner — per-process isolation eliminates shared-state races and enables parallel execution without code changes

## Evidence

- [[session-20260329-202000]] - WASM tests 2.4x faster (132s vs 320s serial), env_test_mutex becomes harmless no-op, flaky cargo-test race eliminated (weight: 0.9)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-03-29: Created — metrics computed by `patina scrape`
