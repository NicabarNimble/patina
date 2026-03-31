# Design: Parallel Repo Oxidize (Rayon)

## Why This Design

The pain point is inter-repo orchestration, not just per-repo compute.
Patina already does expensive CPU work during oxidize, but the operator
experience for "do all repos" is commonly a serial shell loop.

Rayon gives a clean, battle-tested path for bounded CPU-parallel dispatch
inside one process while preserving Rust ergonomics and avoiding async churn.

## Core Approach

Add bounded inter-repo parallelism at the command layer:

1. Resolve target repo set (`--all` or explicit names).
2. Build a work list.
3. Execute work list with Rayon thread pool size = `jobs`.
4. Collect structured result records.
5. Print deterministic summary sorted by repo id.

## Execution Model

```
repo list -> Vec<RepoTarget>
        -> rayon pool (N jobs)
           -> per repo: update + optional oxidize
           -> RepoRunResult { repo, ok, duration, error }
        -> aggregate + stable print + exit code
```

### Boundedness

- Inter-repo parallelism controlled by `--jobs`.
- `jobs` clamped to `[1, max_safe]` where `max_safe` is host-core-aware.
- Default `jobs`: 2 or 3 (final pick based on quick benchmark).

### Oversubscription Guard

If oxidize internally uses worker threads, effective concurrency can explode.
Guardrail options:

- reduce internal oxidize workers when `jobs > 1`, or
- cap `jobs` more aggressively if internal workers remain fixed.

This gate requires explicit tuning note in help/docs.

## Failure Semantics

- Any single repo failure is isolated; other repos continue.
- Final exit code non-zero if any repo failed.
- Summary prints:
  - `ok` list,
  - `failed` list with short cause,
  - elapsed totals.

## Resume-Friendly Behavior

Two acceptable implementations (pick one for v1):

1. `--failed-only` retry mode from previous run report.
2. `--continue` that skips repos marked successful in a run-state file.

Either is sufficient for gate; v1 can keep state in a simple local JSON file.

## Candidate Code Targets

- `src/commands/repo/mod.rs`
  - CLI flags (`--jobs`, `--all`, retry mode)
- `src/commands/repo/internal/*`
  - batch scheduler + result aggregation
- any oxidize config plumbing needed to avoid oversubscription

## Commit Plan

1. `feat(repo): add jobs flag and all-repo oxidize command path`
2. `feat(repo): implement bounded rayon batch scheduler`
3. `feat(repo): add aggregated status reporting and non-zero failure exit`
4. `feat(repo): add retry/continue mode for failed repos`
5. `test(repo): add bounded concurrency and aggregation tests`
6. `docs(repo): add tuning guidance for jobs/workers`

## Verification Plan

1. Unit tests for job clamping and result aggregation ordering.
2. Integration test with mixed success/failure repos.
3. E2E run on known repo set with `--jobs 1` vs `--jobs 3` and timing note.
4. Confirm no unbounded thread creation paths.

## Open Questions

- Best default `jobs` on laptop-class machines: 2 vs 3?
- Should summary emit JSON mode for automation in v1 or v2?
- Where to persist retry metadata for `--failed-only`?
