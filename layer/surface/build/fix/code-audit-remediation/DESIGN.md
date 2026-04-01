# Design: Code Audit Remediation Program Split

## Intent

Restructure the original single remediation spec into a program of focused specs so execution follows Patina core values:

- [[unix-philosophy]]: one spec, one job.
- [[dependable-rust]]: narrow interfaces, local change reasoning.
- [[spec-driven-design]]: explicit contracts and bounded authorization.

## Program Topology

```
code-audit-remediation (umbrella)
  -> car-safety-a1-a6
  -> car-architecture-a7-a25
  -> car-deadcode-a8-a24
  -> car-dedup-a9-a21
  -> car-cleanup-non-a
```

## Execution Order

1. **Safety** first to remove panic/concurrency/data-loss risks.
2. **Architecture** second so dependency direction is settled before deletions.
3. **Dead code** third after canonical ownership is clear.
4. **Dedup** fourth to converge behavior around the stabilized architecture.
5. **Cleanup** last (deprecated flags/docs/archive hygiene).

## Contract Boundaries

- Umbrella spec: sequencing and governance only.
- Child specs: all file-level implementation authorization.
- No cross-child opportunistic refactors.
- Commit granularity: one gate or one tightly coupled pair only.

## Gjengset Lens Applied

- Prefer eliminating global footguns and panic paths before any broad refactor.
- Keep behavior changes explicit and test-backed.
- Avoid mixing boundary moves with large deletions in the same gate.
- Make rollback straightforward through narrow commits.

## Completion Condition

This umbrella closes when all five child specs are complete and each has objective verification evidence recorded in its own exit criteria.
