---
type: belief
id: velocity-over-calendar-freshness
persona: architect
facets: [workflow, governance, specs, velocity]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-27
revised: 2026-03-27
---

# velocity-over-calendar-freshness

In high-velocity repos, spec freshness is determined by relevant commit drift and contract-surface churn, not by elapsed calendar time.

## Statement

In high-velocity repos, spec freshness is determined by relevant commit drift and contract-surface churn, not by elapsed calendar time.

## Evidence

- [[session-20260327-021039-379187000]] - While hardening spec governance, day-based freshness repeatedly misclassified risk; commit-path drift proved the reliable stale signal. (weight: 0.95)

## Supports

- [[stale-context-is-hostile-context]]
- [[truthful-specs]]
- [[specs-describe-current-code-not-aspirations]]

## Attacks

- [[calendar-age-is-enough-for-staleness]] (status: defeated, reason: elapsed time alone misses high-change windows and over-flags low-change windows)

## Attacked-By

- [[low-velocity-repos-can-use-calendar-gates]] (status: scoped, confidence: 0.35, scope: "small teams with infrequent commits")

## Applied-In

- Spec promote freshness gate now checks global drift, related-path drift, and contract-surface drift before `ready -> active` transitions.
- Child construction canon updated to define staleness in commit-velocity terms rather than day-based aging.

## Revision Log

- 2026-03-27: Created — metrics computed by `patina scrape`
- 2026-03-27: Enriched with supports/attacks and applied-in governance examples from the same session.
