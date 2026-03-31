---
type: feat
id: repo-oxidize-parallelism
status: active
created: 2026-03-31
sessions:
  origin: 20260331-072235-030494000
related:
- src/commands/repo/
- src/commands/repo/internal/
- src/main.rs
- layer/core/unix-philosophy.md
- layer/core/dependable-rust.md
exit_criteria:
- id: rop1-jobs-flag
  text: Repo oxidize/update flow accepts `--jobs <N>` for bounded inter-repo parallelism with a safe default.
  checked: true
- id: rop2-all-repos-path
  text: A first-class command path exists for `all repos + oxidize` without shell loops (e.g. `patina repo update --all --oxidize --jobs N`).
  checked: true
- id: rop3-bounded-worker-pool
  text: Inter-repo concurrency is bounded by a queue/worker model; no unbounded thread spawning.
  checked: true
- id: rop4-rayon-integration
  text: Implementation uses Rayon for CPU-bound parallel dispatch and preserves deterministic final status reporting.
  checked: true
- id: rop5-failure-isolation
  text: Per-repo failures do not abort the whole batch; output includes per-repo success/failure summary and non-zero exit when any fail.
  checked: true
- id: rop6-oversubscription-guard
  text: Default `jobs` and intra-repo worker behavior avoid catastrophic oversubscription; documented tuning guidance is provided.
  checked: true
- id: rop7-resume-friendly
  text: Batch execution can skip already-successful repos in the same invocation context (or via explicit `--continue`/`--failed-only` mode).
  checked: true
- id: rop8-tests-and-proof
  text: Tests cover bounded concurrency behavior, result aggregation, and error handling. Functional proof demonstrates multi-repo batch updates with `--jobs > 1` and `--failed-only` retry behavior.
  checked: true
---
# feat: Parallel Repo Oxidize (Rayon)

> Make `patina repo ... --oxidize` scale across many repositories with bounded parallelism and reliable summaries.

## Problem

Current multi-repo oxidize workflows are often executed as serial shell loops.
That makes large rebuilds too slow and brittle:

- one slow repo stalls everything,
- CPU utilization is inconsistent,
- users manually orchestrate process management,
- failures are hard to aggregate cleanly.

## Goal

Ship a native Patina batch oxidize path with bounded parallelism.

- **Fast**: multiple repos processed concurrently.
- **Safe**: bounded jobs, no resource explosions.
- **Clear**: deterministic per-repo status summary.
- **Recoverable**: failure isolation and resume-friendly workflow.

## Non-Goals

- Rewriting oxidize internals for distributed compute.
- Introducing async runtime migration for repo commands.
- Changing oxidize math/quality semantics.

## Proposed CLI Shape

- `patina repo update --all --oxidize --jobs <N>`
- Optional follow-up: `--failed-only` for retry pass.

`--jobs` default should be conservative (2-3) and bounded.

## Verification

```bash
cargo check --workspace -q
cargo test -q --lib

# Functional
patina repo update --all --oxidize --jobs 3
```

## Build Readiness

- [ ] CLI help/docs updated with `--jobs` semantics.
- [ ] Bounded inter-repo scheduler implemented.
- [ ] Summary output includes success/failure per repo.
- [ ] At least one E2E benchmark/proof recorded in spec notes.
