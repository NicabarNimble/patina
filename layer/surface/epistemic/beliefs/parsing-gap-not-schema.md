---
type: belief
id: parsing-gap-not-schema
persona: architect
facets: [architecture, pragmatism]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-05
revised: 2026-02-05
---

# parsing-gap-not-schema

The gap is often in parsing, not schema — check if the data already exists before designing new structures

## Statement

The gap is often in parsing, not schema — check if the data already exists before designing new structures

## Evidence

- [[session-20260205-130049]] - Spec frontmatter had blocked_by, blocks, target fields for months but scrape never parsed them. Adding parsing unlocked ready queue without new schema. (weight: 0.9)

## Supports

- [[simplicity-is-architecture]] - Extending existing patterns is simpler than creating new ones
- [[read-code-before-write]] - Reading existing code reveals what data already exists

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `src/commands/scrape/layer/mod.rs` — Extended Frontmatter struct to parse existing blocked_by, blocks, target fields [[commit-86d534ba]]

## Revision Log

- 2026-02-05: Created — metrics computed by `patina scrape`
