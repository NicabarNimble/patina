---
type: belief
id: yegge-lens-spec-code-proportionality
persona: architect
facets: [architecture, specs, process, design-philosophy]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-03
revised: 2026-03-03
---

# yegge-lens-spec-code-proportionality

Spec-to-code ratio must remain proportional. Over-specification is entropy — a 500-line spec for a 50-line change signals process overhead, not rigor. Specs authorize and scope work; they do not substitute for readable code. When spec complexity exceeds implementation complexity, the spec needs compression, not the code.

## Statement

Spec-to-code ratio must remain proportional. Over-specification is entropy — a 500-line spec for a 50-line change signals process overhead, not rigor. Specs authorize and scope work; they do not substitute for readable code. When spec complexity exceeds implementation complexity, the spec needs compression, not the code.

## Evidence

- [[session-20260303-101839]]: Formalized from Steve Yegge's platform philosophy. Creates productive tension with spec-driven-design (which defaults to 'when in doubt, SPEC'). The resolution: spec when scope is ambiguous, compress when the spec outgrows the work. Applied implicitly — Patina specs that were too broad (e.g., original data-architecture covering 5 areas) were split into focused sub-specs. (weight: 0.8)

## Supports

- [[spec-driven-design]] — productive tension: specs authorize work, this belief constrains spec size

## Attacks

## Attacked-By

- [[spec-driven-design]] Rule 4 — "when in doubt, SPEC" defaults toward more spec, not less. Resolution: both are right at different thresholds. Spec when scope is ambiguous, compress when spec outgrows the work.

## Applied-In

- [[data-architecture-v2]] — split into 5 sub-specs when the parent grew too large for its implementation scope
- [[scrape-diff-driven]] — focused 7-EC spec for a performance refactor, proportional to the work

## Revision Log

- 2026-03-03: Created — metrics computed by `patina scrape`
