# Design: Test Suite Tiering (Local-First, Selective, and Complete)

## Why This Design

Patina needs two properties at once:

1. strong local confidence before push
2. sustainable push/merge velocity as workspace complexity grows

Current checks mix instant policy guards and minutes-scale cargo work in one
pre-push path, and some guards are now stale against current code layout. This
design separates check lanes by cost and blast radius while preserving full
coverage in CI.

## Build Target

After this refactor:

- pre-commit remains instant and deterministic
- pre-push structural checks remain cargo-free
- pre-push cargo checks run in targeted mode by default, with explicit full
  fallback
- CI is complete and authoritative for merge safety
- exhaustive end-to-end depth runs on scheduled lane, not every local push
- stale/no-op checks are removed or replaced with live invariants

## Execution Model

### 1) Two-axis tiering

- axis A: trigger frequency (`commit`, `push`, `manual`, `CI`, `nightly`)
- axis B: execution cost (`instant`, `fast`, `targeted-cargo`, `full-cargo`, `exhaustive`)

This avoids forcing every check into one hook.

### 2) Impact-driven local cargo lane

Targeted local lane computes impact set and runs only required heavy checks.

Inputs:

- changed files from git diff
- package graph from `cargo metadata`
- explicit path trigger map for parity/schema/integration checks

Behavior:

- run clippy/tests for impacted packages by default
- escalate to full workspace when broad-impact files change (root `Cargo.toml`,
  lockfile, shared core crates, toolchain files, boundary policy scripts)
- trigger ducklake parity/schema checks only when affected paths change

### 3) Deterministic fallback policy

If impact resolution is uncertain, fail closed to broader coverage:

- fallback to full workspace lane locally
- never silently skip mandatory checks

### 4) CI authority

CI always runs required full checks and is merge-blocking:

- full workspace fmt/clippy/tests/build/install
- parity/integration checks
- policy checks

Local hooks optimize iteration; CI is final authority.

## Resolved Decisions

- keep local-first testing as default developer experience
- use targeted cargo runs locally instead of full-suite-on-every-push
- preserve one-command full local suite for release-grade verification
- treat stale/no-op checks as defects in test architecture
- defer Bazel for now; revisit only with measured threshold breach

## Commits

1. `refactor(testing): split local gates into tiered hook lanes`
   - normalize pre-commit and pre-push structure around budgeted lanes.

2. `feat(testing): add impact analysis for targeted cargo checks`
   - compute impacted packages and fallback rules.

3. `refactor(testing): repair stale pre-push invariants`
   - remove/replace no-op MCP check and always-skipped integration path.

4. `ci(testing): close coverage gaps in merge gate`
   - ensure schema/parity/integration/policy coverage is complete in CI.

5. `feat(testing): add full local suite command`
   - one deterministic command mirroring CI semantic coverage.

6. `ci(testing): add scheduled exhaustive and flake-observation lane`
   - run expensive end-to-end checks outside hot push path.

7. `docs(testing): publish constitution and operation guide`
   - document lane ownership, budgets, and escalation rules.

## Direct Code Targets

- `resources/git/pre-commit-checks.sh`
  - tier 0 instant checks only.
- `resources/git/pre-push-checks.sh`
  - tier 1 structural + tier 2 targeted orchestration.
- `resources/git/preflight-full.sh` (new)
  - full local suite command.
- `resources/scripts/` (new impact + invariant helpers)
  - impact mapping and path-trigger check logic.
- `.github/workflows/test.yml`
  - full merge-gate coverage and job structure updates.
- `.github/workflows/` (new scheduled workflow if needed)
  - exhaustive and flake-observation lane.
- `resources/git/README.md`
  - contributor-facing hook and lane documentation.

## Verification Plan

1. Budget checks
   - validate pre-commit under 5s
   - validate structural pre-push under 30s

2. Impact checks
   - single-crate change runs targeted package tests
   - broad-impact change triggers full fallback

3. Triggered heavy checks
   - ducklake path change triggers parity lane
   - schema path change triggers schema check lane

4. CI parity
   - verify CI includes all mandatory heavy checks
   - verify local full suite and CI produce equivalent pass/fail semantics

5. Stale-check elimination
   - no missing-file checks remain in active hook path
   - no mandatory integration check is silently skipped by default

## Build Readiness

- [x] Code-truth audit captured (hooks + workflows + stale checks)
- [x] Constitution and gate model specified
- [x] Bazel decision recorded with reevaluation policy
- [ ] Script and workflow implementation pending
- [ ] Timing and coverage proofs pending

## Bazel Decision and Trigger Policy

Current decision: no Bazel adoption in this slice.

Reevaluation trigger examples:

- targeted local lane still exceeds practical budget after impact selection
- CI remains slow/fragile after pipeline parallelization and caching
- cross-language remote execution needs materially exceed Cargo-native options

If triggered, create a dedicated Bazel evaluation spec with migration cost,
ownership model, and rollback plan.

## Open Questions

- Should targeted local testing include direct reverse dependencies by default,
  or only changed packages unless fallback triggers?
- Where should flake quarantine policy live (workflow-only vs test harness metadata)?
- Which checks should hard-fail nightly vs only emit regression reports?
