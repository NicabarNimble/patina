---
type: belief
id: specs-ship-features-audits-ship-quality
persona: architect
facets: [workflow, quality, specs]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-24
revised: 2026-02-24
---

# specs-ship-features-audits-ship-quality

A spec ships a feature. An audit ships quality. Every completed spec gets a review pass — reading implementation against design, testing exit criteria, checking for ungrounded assertions. Audits are session discipline, not a lifecycle gate. Document findings in the session before marking complete.

## Statement

A spec ships a feature. An audit ships quality. Every completed spec gets a review pass — reading implementation against design, testing exit criteria, checking for ungrounded assertions. Audits are session discipline, not a lifecycle gate. Document findings in the session before marking complete.

## Evidence

- [[session-20260224-131253]]: post-implementation Gjengset-style audit of spec-module-split found 1 real bug (hardcoded DB path) and 4 code quality issues (weight: 0.95)
- [[session-20260223-152707]]: implementation readiness audit of spec-workflow-rigor discovered walkthrough phase labels needed — real risk surfaced (weight: 0.85)
- [[session-20260223-120524]]: code reality check found spec-knowledge-evolution referenced non-existent files — "specs written without reading code are wrong" (weight: 0.90)
- [[session-20260222-165738]]: exit criteria testing found 5/6 plugins built after context loss had bugs (weight: 0.90)
- [[session-20260214-202314]]: systematic audit found bugs in 5 of 6 grammar-extraction plugins (weight: 0.90)
- Plugin system v0.17.0 release gate: 26-page formal audit covering 11 sections, zero critical findings caught by process not luck (weight: 0.95)

## Supports

- [[spec-driven-design]] — audits verify that specs actually authorized the right action
- [[dead-code-requires-decision]] — audits surface dead code that annotations hide
- [[context-loss-audit-required]] — generalizes context-loss audits to all completed specs
- [[adversarial-spec-review]] — audits are the post-implementation counterpart to pre-coding adversarial review

## Attacks

_None identified_

## Attacked-By

- "Not every spec needs an audit" (status: active, scope: "trivial fixes and single-line changes don't justify the overhead")

## Applied-In

- spec-module-split: audit found hardcoded DB_PATH bug + 4 quality issues → this session's fixes
- spec-workflow-rigor: implementation readiness audit → design.md with gap analysis
- plugin-system v0.17.0: formal release gate audit → SHIP IT verdict with documented reasoning
- grammar-extraction Phase 3: audit caught bugs in 5/6 plugins before release

## Revision Log

- 2026-02-24: Created — metrics computed by `patina scrape`
