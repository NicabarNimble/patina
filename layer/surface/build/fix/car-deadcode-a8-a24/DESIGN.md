# Design: CAR Dead Code (A8, A10-A12, A14-A15, A22-A24)

## Principle Alignment

- [[unix-philosophy]]: remove tools/modules that no longer do a job.
- [[spec-driven-design]]: each deletion must be explicitly authorized and verified.

## Strategy

- Apply deletions in dependency order:
  - A8 before A10.
  - A11/A12 in isolated commits due to broad file impact.
- Use compile failures as the map for remaining hidden callers.

## Verification

- Compile + full lib tests after each deletion cluster.
- No blanket `allow(dead_code)` added as escape hatch.

## Out of Scope

- Behavior fixes (A1-A6) and inversion fixes (A7/A25).
