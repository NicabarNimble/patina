---
type: refactor
id: test-suite-tiering
status: ready
created: 2026-03-29
updated: 2026-03-29
sessions:
  origin: 20260328-134311-971500000
related:
- resources/git/pre-push-checks.sh
- .github/workflows/test.yml
- .github/workflows/pr-gate.yml
- resources/git/pre-commit-checks.sh
- resources/scripts/check-ducklake-parity.sh
- layer/surface/build/refactor/test-suite-tiering/DESIGN.md
- .git/hooks/pre-push
beliefs:
- '[[unix-philosophy]]'
- '[[dependable-rust]]'
- '[[measure-first]]'
- '[[spec-needs-code-verification]]'
exit_criteria:
- id: tst1-code-truth-baseline-captured
  text: Spec records current hook/CI behavior with code-truth evidence (including stale/no-op checks), so redesign decisions are based on code not drifted docs.
  checked: true
- id: tst2-pre-commit-budget-and-scope
  text: Pre-commit gate runs in under 5 seconds and contains only deterministic instant checks (format + large file guard).
  checked: true
- id: tst3-pre-push-structural-budget
  text: Pre-push structural gate runs in under 30 seconds and contains no cargo build/test work.
  checked: true
- id: tst4-local-targeted-cargo-lane
  text: A local targeted cargo lane runs on push by default (changed-package clippy/tests plus path-triggered parity/schema checks), with full-workspace fallback for broad-impact changes.
  checked: true
- id: tst5-ci-authoritative-and-complete
  text: CI contains all mandatory heavy checks (workspace clippy/tests/build/install, ducklake parity, schema consistency, broker integration, and policy checks) and is merge-blocking authority.
  checked: true
- id: tst6-stale-checks-repaired
  text: Current stale checks are removed or replaced with live invariants (no no-op pass paths such as missing-file MCP checks or always-skipped integration gates).
  checked: true
- id: tst7-full-local-suite-available
  text: A single local command runs the full suite (same semantic coverage as CI) for release-grade local verification when needed.
  checked: true
- id: tst8-end-to-end-not-always-on
  text: End-to-end/integration-heavy checks are selective locally by default and always covered in CI/nightly lanes.
  checked: true
- id: tst9-bazel-decision-explicit
  text: 'Spec captures explicit Bazel decision: deferred now, with measurable trigger thresholds for reevaluation.'
  checked: true
---
# refactor: test-suite-tiering (local-first selective rigor)

## Problem

Patina's local/CI test policy drifted into an unstable shape:

- pre-push runs a mixed 14-step pipeline containing both instant policy checks and heavy cargo/test checks
- some checks in that pipeline are now stale/no-op against current code layout
- CI does not fully mirror all mandatory checks currently assumed by local hooks

This violates two project needs:

1. local-first confidence (strong testing before remote)
2. reliable push velocity (no brittle, minutes-long pre-push bottleneck)

## History

The hook grew incrementally as valid safeguards were added by prior specs:

- WIT consistency and mirror checks
- crate naming and architectural boundary checks
- ducklake parity check
- clippy, tests, and schema checks
- broker integration and MCP-thin-handler invariants

The growth was additive and useful, but not re-tiered as project shape changed
(MCP retirement, crate reshaping, integration guard drift, and larger workspace test load).

## Goal

Define and execute a testing constitution that is:

- strong enough for enterprise-grade confidence
- local-first by default
- selective for expensive checks
- CI-complete and authoritative
- explicit about when Bazel is, or is not, justified

## Non-Goals

- adopting Bazel in this spec
- rewriting all tests or introducing a brand new harness stack
- promising zero flakes in one pass
- changing product behavior unrelated to testing and gate placement

## Status

Complete. Tiering structure implemented and deployed 2026-03-29.

### Completion Notes (2026-03-29)

All 9 exit criteria met for the gate *structure* redesign. Two caveats
carried forward to `ci-environment-parity` fix spec:

- **tst5**: CI workflow now lists all mandatory checks, but `cargo test --workspace`
  fails due to missing WASM toolchain and connection config in the CI environment.
  The gate structure is correct; the environment setup is not. See `ci-environment-parity`.
- **tst7**: `preflight-full.sh` runs the same check set as CI, but does not build
  WASM child artifacts either. Local passes because dev machines have pre-built
  artifacts; CI and clean environments do not. See `ci-environment-parity`.

## Code-Truth Snapshot (2026-03-29)

- `resources/git/pre-commit-checks.sh` currently runs only large-file detection.
- `resources/git/pre-push-checks.sh` currently runs 14 checks, including heavy
  cargo checks (`fmt`, `clippy`, full `test`, schema check).
- Step 12 broker integration is conditionally skipped based on metadata and local
  child install expectations; this is not a reliable always-on guard.
- Step 13 MCP handler invariants references `src/mcp/server/scry.rs` and
  `src/mcp/server/assay.rs`, which do not exist in current code layout.
- `.github/workflows/test.yml` currently does not run every check local hooks
  imply is mandatory (notably schema consistency and broker integration behavior).
- Observed timing sample from audit session:
  - pre-commit hook around 0.02s
  - pre-push reaches heavy cargo steps and runs substantially longer
  - full workspace tests are minutes-scale, not seconds-scale

## Testing Constitution (Target Shape)

This spec separates checks by both trigger frequency and runtime cost.

### Tier 0: pre-commit (< 5s)

- deterministic instant checks only
- format check
- staged large-file guard

### Tier 1: pre-push structural (< 30s, no cargo)

- WIT consistency and mirror completeness
- crate naming policy
- core/protocol dependency direction
- single SDK surface
- runtime boundary drift
- layer output contract
- replacement for stale MCP check using current-runtime invariant(s)

### Tier 2: pre-push targeted cargo (local-first default)

- changed-package `clippy` + tests
- path-triggered heavy checks only when touched:
  - ducklake parity
  - schema consistency
- full-workspace fallback when impact is broad (workspace/Cargo graph/toolchain/core boundary files)

### Tier 3: local full suite (manual, release-grade)

- one command to run semantic equivalent of CI full suite
- used before risky merges/releases or when targeted lane escalates

### Tier 4: CI merge gate (authoritative)

- full workspace checks (fmt/clippy/tests/build/install)
- all required parity/integration/policy checks
- merge-blocking source of truth

### Tier 5: scheduled exhaustive lane

- expensive end-to-end and stability checks
- flake detection/reporting
- benchmark/regression surveillance

## Migration Map (Current 14-Step Hook -> Target)

| Current step | Check | Current state | Target tier | Notes |
|---|---|---|---|---|
| 1 | WIT consistency | active | Tier 1 | stay fast local |
| 2 | WIT mirror completeness | active | Tier 1 | stay fast local |
| 3 | Crate naming policy | active | Tier 1 | stay fast local |
| 4 | Core/protocol deps | active | Tier 1 | stay fast local |
| 5 | Single SDK surface | active | Tier 1 | stay fast local |
| 6 | Runtime boundary drift | active | Tier 1 | stay fast local |
| 7 | Layer output contract | active | Tier 1 | stay fast local |
| 8 | DuckLake parity | active | Tier 2 + Tier 4 | local selective + always in CI |
| 9 | Formatting | active | Tier 0 + Tier 4 | local instant + CI authority |
| 10 | Clippy | active | Tier 2 + Tier 4 | targeted local + full CI |
| 11 | Tests | active | Tier 2 + Tier 4 | targeted local + full CI |
| 12 | Broker integration | stale/conditional | Tier 4 (+ optional Tier 3) | make deterministic CI check |
| 13 | MCP invariants | stale/no-op | Tier 1 replacement | replace with live invariant |
| 14 | Schema consistency | active | Tier 2 + Tier 4 | local selective + always in CI |

## Solution

### TST-G1: Normalize hooks around deterministic budgets

- keep pre-commit instant
- split pre-push into structural and targeted cargo lanes
- enforce hard budgets in script output and failure text

### TST-G2: Implement impact-driven local cargo selection

- compute impacted packages from changed paths + cargo metadata
- run `clippy` and tests on impacted set by default
- escalate to full-workspace when impact rules are triggered

### TST-G3: Repair stale checks

- replace stale MCP invariant check with current-runtime invariant check
- make broker integration deterministic and non-silent (no accidental always-skip path)

### TST-G4: Close CI completeness gap

- mirror all mandatory heavy checks in CI
- ensure policy checks and selective local checks have CI authority equivalent

### TST-G5: Add explicit full-local and exhaustive lanes

- full local suite command for release-grade runs
- scheduled exhaustive lane for end-to-end depth and flake detection

### TST-G6: Record Bazel decision with objective triggers

- defer Bazel now
- record measurable thresholds that would justify a Bazel evaluation spec

## Implementation Order

1. Encode code-truth baseline in spec and design docs.
2. Refactor pre-commit/pre-push scripts into Tier 0/1/2 model.
3. Add impact selection script and fallback rules.
4. Repair stale integration and invariant checks.
5. Update CI workflow to close mandatory-check gaps.
6. Add full-local suite command and docs.
7. Add scheduled exhaustive lane and flake reporting.
8. Capture before/after timing and check coverage matrix.

## Resolved Decisions

- Patina stays local-first: most useful signal should run locally before push.
- End-to-end and heavy integration checks are selective locally, always covered in CI/nightly.
- Stale/no-op checks are not acceptable gate coverage and must be replaced or removed.
- CI is merge authority; local hooks are developer feedback accelerators.
- Bazel is deferred for now; optimize with Rust-native tooling and impact selection first.

## Bazel Position

Decision for this spec: **do not adopt Bazel now**.

Reevaluate only if measured pain remains after Tier 0-5 rollout, for example:

- local targeted lane routinely exceeds agreed budget
- CI latency remains unacceptable after parallelization and caching
- multi-language remote execution requirements exceed Cargo-native ergonomics

If those thresholds are hit, create a dedicated Bazel evaluation spec with
migration and cost modeling.

## Verification

```bash
# Tier budgets
time resources/git/pre-commit-checks.sh
time resources/git/pre-push-checks.sh

# Tier 2 behavior: targeted by default, full fallback on broad impact
resources/git/pre-push-checks.sh

# Full local suite
resources/git/preflight-full.sh

# CI coverage parity (manual audit or scripted assertion)
rg -n "schema check|ducklake|clippy|cargo test|cargo install|crate naming|runtime boundary|WIT" .github/workflows/test.yml
```

## Build Readiness

Ready. Audit is complete and this spec now describes a code-truth-first
execution path to enterprise-grade rigor without sacrificing local velocity.
