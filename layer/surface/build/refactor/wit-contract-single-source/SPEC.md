---
type: refactor
id: wit-contract-single-source
status: draft
created: 2026-03-26
sessions:
  origin: 20260325-150227-161735000
related:
  - wit/
  - wit/toys/
  - wit/worlds/
  - sdk/patina-sdk/wit/
  - sdk/patina-sdk-core/build.rs
  - sdk/patina-sdk-data/build.rs
  - sdk/patina-sdk-agent/build.rs
  - resources/scripts/check-wit-toys-sync.sh
exit_criteria:
  - id: wcs1-sdk-wit-not-checked-in
    text: "`sdk/patina-sdk/wit/` is either .gitignore'd or removed from the repo. No WIT files are checked into the SDK crate directory."
    checked: false
  - id: wcs2-dead-wit-removed
    text: "`sdk/patina-sdk/wit/mother-child/` is deleted. No dead WIT worlds exist in any location."
    checked: false
  - id: wcs3-build-rs-canonical
    text: "All SDK sub-crate build.rs files reference `wit/` (root canonical) as their sole WIT source during workspace builds. No hardcoded WIT copies in SDK source tree."
    checked: false
  - id: wcs4-xtask-publish
    text: "A `cargo xtask publish-sdk` (or equivalent) command exists that: copies canonical `wit/` into SDK for crates.io bundling, verifies byte-for-byte parity, runs `cargo publish --dry-run`, and cleans up after."
    checked: false
  - id: wcs5-worlds-dedup
    text: "`wit/worlds/` toy mirrors are eliminated or generated. No manually-maintained duplicate WIT files exist. The sync script is replaced by a compile-time or build-time check."
    checked: false
  - id: wcs6-builds-pass
    text: "`cargo check --workspace`, `cargo test -q`, and all children build successfully."
    checked: false
---
# refactor: WIT contract single source of truth

> Make `wit/` the sole source of WIT definitions. Eliminate checked-in copies, replace script-based sync with build-time enforcement.

## Problem

WIT interface definitions exist in three locations:

1. **`wit/`** (root) — canonical definitions. 22 toy interfaces, 4 child worlds, schemas.
2. **`sdk/patina-sdk/wit/`** — static copy checked into git for crates.io `include`. Contains dead `mother-child/` world.
3. **`wit/worlds/`** — mirrors of `wit/toys/*.wit` files. Checked by `check-wit-toys-sync.sh`.

The SDK sub-crates (`patina-sdk-{core,data,agent}`) already do the right thing: their `build.rs` references `../../wit/toys/` directly and copies to `OUT_DIR` at compile time with `cargo:rerun-if-changed`. But the umbrella `sdk/patina-sdk/` has a static `wit/` directory checked into git, and `wit/worlds/` manually mirrors `wit/toys/`.

A shell script (`check-wit-toys-sync.sh`) validates mirrors match. This is a runtime check that someone must remember to run. Drift is possible — and `sdk/patina-sdk/wit/mother-child/` already demonstrates it (dead world still present).

## Goal

Single source of truth: `wit/` at repo root. Everything else either references it at build time or generates from it at publish time. Drift becomes a compile error, not a script check.

## Non-Goals

- Do NOT change WIT interface contents (no toy additions/removals/renames).
- Do NOT change the SDK's public API or feature flags.
- Do NOT restructure `wit/` internally (world composition stays as-is).
- Do NOT add warg/registry support (ecosystem isn't ready).
- Do NOT change how children reference WIT (their `child.toml` / `Cargo.toml` metadata stays).

## Current State

```
wit/                              ← canonical (22 toys, 4 worlds, schemas)
wit/worlds/*.wit                  ← manually mirrors wit/toys/*.wit (drift risk)
sdk/patina-sdk/wit/               ← checked-in copy for crates.io (stale mother-child/)
sdk/patina-sdk-core/build.rs      ← already reads ../../wit/toys/ ✓
sdk/patina-sdk-data/build.rs      ← already reads ../../wit/toys/ ✓
sdk/patina-sdk-agent/build.rs     ← already reads ../../wit/toys/ ✓
check-wit-toys-sync.sh            ← script-based drift check (not enforced)
```

## Target State

```
wit/                              ← sole source of truth (unchanged)
wit/worlds/*.wit                  ← generated from wit/toys/ by build.rs or xtask
sdk/patina-sdk/wit/               ← .gitignore'd; generated at publish time only
sdk/patina-sdk-core/build.rs      ← reads ../../wit/toys/ (unchanged, already correct)
sdk/patina-sdk-data/build.rs      ← reads ../../wit/toys/ (unchanged, already correct)
sdk/patina-sdk-agent/build.rs     ← reads ../../wit/toys/ (unchanged, already correct)
xtask/src/main.rs                 ← publish-sdk command generates wit/ copy, verifies, publishes
check-wit-toys-sync.sh            ← deleted (replaced by build-time check)
```

## Solution

### Step 1: Delete dead SDK WIT

Remove `sdk/patina-sdk/wit/mother-child/`. It references a deleted runtime path.

### Step 2: .gitignore SDK WIT copy

Add `sdk/patina-sdk/wit/` to `.gitignore`. Remove the checked-in copy from git tracking. The crates.io `include` directive stays — it will bundle whatever exists at publish time.

### Step 3: Create xtask publish-sdk

Create `xtask/` workspace member with a `publish-sdk` command that:
1. Copies `wit/` → `sdk/patina-sdk/wit/` (only the worlds/toys/deps needed by SDK, not schemas)
2. Strips dead worlds (mother-child)
3. Verifies byte-for-byte parity with canonical source
4. Runs `cargo publish -p patina-sdk --dry-run`
5. Optionally runs the actual publish
6. Cleans up the generated copy (or leaves it for inspection)

### Step 4: Eliminate wit/worlds/ toy mirrors

Currently `wit/worlds/*.wit` contains both:
- Composed world definitions (e.g., `belief-verifier.wit`, `ducklake.wit`) — these are unique
- Mirror copies of `wit/toys/*.wit` files — these are duplicates

Options (needs investigation):
- **Option A**: Generate the mirrors via build.rs or xtask, .gitignore them
- **Option B**: Restructure so worlds reference toys via WIT package imports instead of file copies
- **Option C**: If wit-bindgen/cargo-component requires the files to be co-located, keep them but add a build.rs assertion that panics on drift

### Step 5: Replace sync script with build-time check

Delete `resources/scripts/check-wit-toys-sync.sh`. Add a workspace-level build.rs or test that asserts WIT parity at compile time. If any mirror drifts, `cargo check` fails.

## Implementation Order

1. Delete dead `sdk/patina-sdk/wit/mother-child/` (immediate, zero risk)
2. Create xtask crate with publish-sdk command
3. .gitignore `sdk/patina-sdk/wit/` and remove from git tracking
4. Investigate wit/worlds/ mirror strategy (Option A/B/C)
5. Replace sync script with build-time assertion
6. Verify full workspace build + children build

## Resolved Decisions

- SDK sub-crate build.rs files are already correct — they reference canonical `wit/` via relative path. No changes needed there.
- The umbrella `patina-sdk` crate is the only one published to crates.io. Sub-crates are workspace-internal with path deps.
- `wit/` at repo root is and remains the canonical source of truth.
- `sdk/patina-sdk/wit/mother-child/` is dead — MotherChild runtime, trait, and SDK API are all deleted.

## Verification

```bash
cargo check --workspace -q
cargo test -q
# Verify SDK wit/ is not tracked:
git ls-files sdk/patina-sdk/wit/ | wc -l  # should be 0
# Verify xtask works:
cargo xtask publish-sdk --dry-run
# Verify no dead WIT:
test ! -d sdk/patina-sdk/wit/mother-child
# Verify children still build:
cargo check -p patina-ai-child-doctor
cargo check -p patina-ai-child-ducklake
```

## Build Readiness

Ready to execute. Step 4 (wit/worlds/ mirror strategy) needs investigation before committing to Option A/B/C, but Steps 1-3 can proceed independently.
