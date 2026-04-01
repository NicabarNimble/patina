# Design: CAR Architecture (A7, A25)

## Principle Alignment

- [[dependable-rust]]: preserve black-box boundaries and avoid reverse imports.
- [[unix-philosophy]]: one architectural concern per spec.

## Gate Strategy

- A7: move enrichment utilities out of command-internal dependency path so retrieval stands alone.
- A25: move spec dispatch ownership to library-side surface consumed by CLI, not vice versa.

## Verification

- Compile and full lib tests.
- `patina scry` functional checks.
- `patina spec list/show/check/ready/blocked/next` functional checks.

## Out of Scope

- Correctness/panic fixes from A1-A6.
- Dead code and cleanup removals.
