# Design: CAR Safety (A1-A6)

## Principle Alignment

- [[dependable-rust]]: keep public behavior stable while fixing internals.
- [[unix-philosophy]]: only correctness/safety in this spec.
- Gjengset lens: remove panic/concurrency/data-loss risks first.

## Gate Strategy

1. A1 UTF-8 safety fix + regression test.
2. A2 retrieval path parameterization, no `set_current_dir` in retrieval query flow.
3. A3 capability unification and parity test.
4. A4 real starting commit persistence and legacy-safe fallback behavior.
5. A5 dynamic index dimension probing.
6. A6 canonical frontmatter type adoption.

## Verification

- Compile + full lib tests.
- Focused functional checks:
  - `patina scry` single repo and cross-repo paths,
  - session start/end artifact correctness,
  - mother runtime session metadata correctness.

## Out of Scope

- Dead code deletion and deprecated command cleanup.
- Architecture inversion gates A7/A25.
