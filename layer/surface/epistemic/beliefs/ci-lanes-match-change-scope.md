---
type: belief
id: ci-lanes-match-change-scope
persona: architect
facets: [ci, testing, architecture]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-29
revised: 2026-03-29
---

# ci-lanes-match-change-scope

CI lanes match what changed — unit tests on every push, integration tests on path trigger, release build on PR merge gate only. Parallel jobs so wall time equals the slowest lane, not the sum.

## Statement

CI lanes match what changed — unit tests on every push, integration tests on path trigger, release build on PR merge gate only. Parallel jobs so wall time equals the slowest lane, not the sum.

## Evidence

- [[session-20260329-202000]] - CI dropped from 57 min serial to 5 min parallel (warm) by splitting lint/test/release into concurrent jobs with conditional WASM steps (weight: 0.9)

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
