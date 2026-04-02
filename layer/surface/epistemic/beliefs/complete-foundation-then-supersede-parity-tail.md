---
type: belief
id: complete-foundation-then-supersede-parity-tail
persona: architect
facets: [specs, workflow, migration, lakehouse]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-12
revised: 2026-04-02
---

# complete-foundation-then-supersede-parity-tail

When migration parity conflicts with product-direction quality goals, complete the foundation spec and explicitly supersede the parity tail via split or abandon.

## Statement

When migration parity conflicts with product-direction quality goals, complete the foundation spec and explicitly supersede the parity tail via split or abandon.

## Evidence

- [[session-20260312-160150]]: We shifted from parity-first to lakehouse-quality direction and completed cutover foundation before superseding parity tail (weight: 0.94)
- [[commit-9173a6b7]]: Marked [[ducklake-knowledge-child-cutover]] complete as the delivered foundation slice before parity-tail supersession (weight: 0.90)
- [[commit-52e43e6e]]: Archived abandoned parity-tail follow-up to make supersession explicit instead of leaving ambiguous in-progress scope (weight: 0.90)
- [[commit-6456debd]]: Promoted [[ducklake-github-lakehouse-ingestion]] to active to anchor forward execution on quality-complete ingestion goals (weight: 0.91)

## Supports

- [[patina-is-combination-of-knowledge-and-action]]

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- Historical case study: [[ducklake-knowledge-child-cutover]] and [[ducklake-github-lakehouse-ingestion]]
- Current application: [[pando-vocabulary-alignment]] abandoned in favor of split, executable specs ([[ducklake-retirement]], [[child-rename]], [[engine-consolidate]])

## Revision Log

- 2026-03-12: Created — metrics computed by `patina scrape`
- 2026-04-02: Revised — retained ducklake references as historical case study only; added current split-spec application.
