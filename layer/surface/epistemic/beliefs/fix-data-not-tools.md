---
type: belief
id: fix-data-not-tools
persona: architect
facets: [architecture, data-quality, contracts]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-05
revised: 2026-02-05
---

# fix-data-not-tools

Messy data is a bug, not a reason for permissive tooling — fix the data to conform to the contract, don't build tools that tolerate broken contracts.

## Statement

Messy data is a bug, not a reason for permissive tooling — fix the data to conform to the contract, don't build tools that tolerate broken contracts.

## Evidence

- [[session-20260205-142325]]: During spec triage, chose to fix broken frontmatter rather than build regex-based tools that tolerate missing fields. Anchored in [[system-owns-format]].

## Supports

- [[system-owns-format]] — If system owns format, data must conform to it
- [[milestones-in-specs]] — One source of truth requires conformant data

## Attacks

- [[permissive-parsing]] (status: defeated, reason: "tolerating bad data propagates inconsistency")

## Attacked-By

- [[pragmatic-migration]] (status: scoped, scope: "temporary tolerance during bulk migration is acceptable if followed by cleanup")

## Applied-In

- `src/spec.rs` — Canonical SpecFrontmatter struct; specs that don't parse need fixing
- Spec triage workflow — Fix frontmatter in belief-validation-system rather than building regex fallback

## Revision Log

- 2026-02-05: Created — metrics computed by `patina scrape`
