---
type: belief
id: context-loss-audit-required
persona: architect
facets: [process, quality, sessions]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-14
revised: 2026-02-14
---

# context-loss-audit-required

When building across sessions where context was lost mid-build, audit is a required quality gate — not optional. Mechanical ports degrade without re-reading source; 5 of 6 plugins built after context loss had bugs.

## Statement

When building across sessions where context was lost mid-build, audit is a required quality gate — not optional. Mechanical ports degrade without re-reading source; 5 of 6 plugins built after context loss had bugs.

## Evidence

- [[session-20260214-202314]]: [[session-20260214-202314]] - grammar-extraction Phase 3 audit: 5/6 plugins had bugs after context loss (missing node types, corrupted imports, duplicate entries, wrong call edges) (weight: 0.95)

## Supports

- [[read-code-before-write]] — audit is the systematic form of "read before write"
- [[graceful-extraction]] — compiled-in fallback catches plugin bugs, but audit prevents them

## Attacks

<!-- None yet -->

## Attacked-By

<!-- None yet -->

## Applied-In

- [[session-20260214-202314]]: Audited 7 grammar plugins, found and fixed bugs in 5 of 6 built after context loss

## Revision Log

- 2026-02-14: Created — metrics computed by `patina scrape`
