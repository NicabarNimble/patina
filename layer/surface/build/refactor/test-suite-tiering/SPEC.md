---
type: refactor
id: test-suite-tiering
status: draft
created: 2026-03-29
sessions:
  origin: 20260328-134311-971500000
beliefs:
  - "[[unix-philosophy]]"
  - "[[dependable-rust]]"
related:
  - resources/git/pre-push-checks.sh
  - .github/workflows/test.yml
  - .github/workflows/pr-gate.yml
  - .git/hooks/pre-push
exit_criteria:
  - id: tst1-pre-commit-fast
    text: "Pre-commit hook runs in < 5 seconds. Only formatting and file-size checks."
    checked: false
  - id: tst2-pre-push-bounded
    text: "Pre-push hook runs in < 30 seconds. No cargo test, no clippy, no WASM compilation."
    checked: false
  - id: tst3-ci-complete
    text: "CI runs the full check suite (clippy, tests, WASM parity, integration tests). Blocks merge, not push."
    checked: false
  - id: tst4-no-ssh-timeout
    text: "git push to origin completes without SSH connection timeout on normal network conditions."
    checked: false
  - id: tst5-no-check-loss
    text: "Every check that exists today still runs — nothing dropped, only moved between tiers."
    checked: false
---
# refactor: test-suite-tiering

## Problem

The pre-push hook (`resources/git/pre-push-checks.sh`) runs 14 checks including `cargo test --workspace`, `cargo clippy --workspace`, WASM parity tests, broker integration, and schema validation. This was designed to catch problems fast before they hit CI.

With 11+ WASM children crates in the workspace, the hook now takes several minutes. The SSH connection to GitHub drops before the hook finishes, blocking all pushes. The checks are valuable — the hook has caught real problems since each check was added — but they can't all run locally at this project scale.

CI (`.github/workflows/test.yml`) already runs the same checks. They're duplicated between local and remote.

## History

The pre-push hook grew organically as each spec added guards:
- WIT consistency (from stale WIT deps incident, Feb 2026)
- Crate naming policy (from `crate-naming-policy-and-ci` spec)
- Runtime boundary drift (post-toy-collapse)
- DuckLake WASM parity (ducklake-native-removal spec)
- MCP handler invariants (mcp-thin-handlers spec)
- Schema consistency (schema management spec)
- Broker integration test (pipe protocol drift guard)

Each guard caught a real problem. None should be dropped — only moved to the right tier.

## Goal

Restructure checks into tiers so developers push freely, problems are caught at the right layer, and no check is lost.

## Non-Goals

- Adding new checks.
- Rewriting CI from scratch.
- Changing test content — only moving where tests run.

## Tier Design

### Tier 1 — Pre-commit (< 5 seconds)

Fast, runs on every commit. Catches formatting and obvious mistakes.

```bash
cargo fmt --all -- --check
# file size check (existing: no large staged files > 10MB)
```

That's it. No compilation, no tests, no network.

### Tier 2 — Pre-push (< 30 seconds)

Lightweight structural checks. No compilation, no test execution.

```bash
# WIT consistency (SDK mirrors match canonical — file diff, no cargo)
# WIT mirror completeness (file existence check)
# Crate naming policy (grep-based, no cargo)
# Core/protocol dependency direction (grep-based)
# Single SDK surface (grep-based)
# Runtime boundary drift (grep-based)
# Layer output contract (directory existence check)
# MCP handler invariants (line count + grep, no cargo)
```

These are all the checks from steps 1-7 and 13 of the current hook. They run in seconds because they're file/grep based — no `cargo` invocation.

### Tier 3 — CI on push (blocks merge, not push)

Full compilation and test suite. Runs on GitHub Actions after push.

```yaml
# cargo fmt --check (redundant with pre-commit, but CI is the authority)
# cargo clippy --workspace -- -D warnings
# cargo test --workspace
# cargo build --release
# DuckLake WASM parity (cargo test with WASM artifacts)
# Schema consistency (cargo run -- schema check)
# Broker integration test (conditional, if test-child available)
# cargo install --locked (release verification)
```

These are steps 8-12 and 14 of the current hook, plus clippy and tests. They require `cargo` and take minutes.

### Tier 4 — CI on main push (nightly/release quality)

Existing benchmark job, unchanged.

```yaml
# Retrieval quality benchmark (MRR >= 0.55)
# Already in test.yml as a separate job gated on main push
```

## Migration Map

| Current hook step | Check | Current tier | New tier | Reason |
|---|---|---|---|---|
| 1 | WIT consistency | pre-push | **pre-push** | File diff, fast |
| 2 | WIT mirror completeness | pre-push | **pre-push** | File existence, fast |
| 3 | Crate naming policy | pre-push | **pre-push** | Grep, fast |
| 4 | Core/protocol deps | pre-push | **pre-push** | Grep, fast |
| 5 | Single SDK surface | pre-push | **pre-push** | Grep, fast |
| 6 | Runtime boundary drift | pre-push | **pre-push** | Grep, fast |
| 7 | Layer output contract | pre-push | **pre-push** | Dir check, fast |
| 8 | DuckLake WASM parity | pre-push | **CI** | Runs cargo test |
| 9 | Formatting | pre-push | **pre-commit** | Already fast, better as pre-commit |
| 10 | Clippy | pre-push | **CI** | Full workspace compile |
| 11 | Tests | pre-push | **CI** | Full workspace compile + run |
| 12 | Broker integration | pre-push | **CI** | Runs cargo build + cargo run |
| 13 | MCP handler invariants | pre-push | **pre-push** | Grep + wc, fast |
| 14 | Schema consistency | pre-push | **CI** | Runs cargo run |

**Result:** Pre-push drops from 14 checks to 8, all file/grep based. The 6 heavy checks (cargo-dependent) move to CI where they already run.

## Approach

1. Create new `resources/git/pre-commit-checks.sh` with tier 1 checks.
2. Rewrite `resources/git/pre-push-checks.sh` to only include tier 2 checks (remove cargo-dependent steps).
3. Verify CI already covers all tier 3 checks — add any missing ones.
4. Update `.git/hooks/pre-commit` to call the new script.
5. Update `.git/hooks/pre-push` to call the slimmed script.
6. Test: `git push` completes without SSH timeout.
7. Verify: no check is lost — every check still runs somewhere.

## Verification

```bash
# Tier 1 runs fast
time resources/git/pre-commit-checks.sh   # must be < 5s

# Tier 2 runs fast
time resources/git/pre-push-checks.sh     # must be < 30s

# Tier 3 runs in CI
# Verify test.yml covers: clippy, tests, WASM parity, schema, broker integration

# No check lost
diff <(grep -c '📦' resources/git/pre-push-checks.sh.bak) <(grep -c '📦' resources/git/pre-push-checks.sh resources/git/pre-commit-checks.sh .github/workflows/test.yml)
```

## Build Readiness

Ready to start. This is a pure restructuring — no new checks, no removed checks, just moving them to the right tier.
