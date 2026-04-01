# Design: CAR Dedup and Path Truth (A9, A17-A21)

## Principle Alignment

- [[patina-identity]] and [[dependable-rust]]: one canonical source per concern.
- [[unix-philosophy]]: remove duplicate utility logic that fractures behavior.

## Strategy

- Prioritize high-traffic duplication first (path construction and semver).
- For each duplicate family:
  - pick one canonical implementation,
  - migrate all call sites,
  - delete the duplicate.

## Verification

- Compile + full lib tests.
- Targeted tests around section parsing, semver bumping, and path computation.

## Out of Scope

- Dead module deletion and deprecated command cleanup.
