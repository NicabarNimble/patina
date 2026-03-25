---
type: belief
id: wrapper-exceptions-are-constrained
persona: architect
facets: [architecture, sdk, toys, boundaries]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-24
revised: 2026-03-24
---

# wrapper-exceptions-are-constrained

Sdk-local wrappers are constrained exceptions: they must stay thin, documented, gate-verified, and revisit-triggered, and never redefine canonical toy ownership.

## Statement

Sdk-local wrappers are constrained exceptions: they must stay thin, documented, gate-verified, and revisit-triggered, and never redefine canonical toy ownership.

## Evidence

- [[session-20260324-105924]] Wrapper policy was formalized while closing sdk-toybox-definition A5 to prevent ownership drift across tiers (weight: 0.96)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-03-24: Created — metrics computed by `patina scrape`
