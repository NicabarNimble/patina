---
type: belief
id: duplicate-before-extract
persona: architect
facets: [architecture, rust, refactoring]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-04
revised: 2026-02-04
---

# duplicate-before-extract

Duplicate small code before extracting shared modules — two consumers isn't enough to justify abstraction, a third would be.

## Statement

Duplicate small code before extracting shared modules — two consumers isn't enough to justify abstraction, a third would be.

## Evidence

- [[session-20260204-074557]] - Duplicated ~30-line UDS client (`uds_get`, `uds_post`, `parse_http_body`) in `mother/internal.rs` rather than extracting from `session.rs`. Two consumers, same pattern, chose duplication over shared module. (weight: 0.9)
- Rule of three — well-known refactoring heuristic (Fowler, Beck). Extract when the third consumer appears, not the second. (weight: 0.7)

## Supports

- [[dependable-rust]] — "Don't split when unnecessary" / "Simple commands: sequential steps, no abstraction needed"
- [[use-whats-in-the-tree]] — Prefer what exists over introducing new structure

## Attacks

<!-- None identified -->

## Attacked-By

- DRY principle — "Don't Repeat Yourself" argues any duplication is a maintenance burden. Counter: ~30 lines is below the threshold where duplication cost exceeds abstraction cost. The coupling introduced by extraction (shared module, import chains, coordinated changes) often outweighs the savings.

## Applied-In

- `src/secrets/session.rs` — UDS client functions (`uds_get`, `uds_post`, `parse_http_body`), first consumer
- `src/mother/internal.rs` — Same UDS client functions duplicated, second consumer (commit [[0dd3b9ca]])

## Revision Log

- 2026-02-04: Created — metrics computed by `patina scrape`
