# Design: WIT contract single source of truth

## Why This Design

Three copies of WIT definitions create drift surfaces. The SDK already has dead WIT (`mother-child/`) proving the point. Script-based sync checks are opt-in — they don't prevent drift, they detect it after the fact. Build-time enforcement makes drift a compile error.

This is also a prerequisite for greenfield crate extraction: moving engine and store into their own crates will add more WIT consumers. If copies exist today, extraction multiplies the sync problem.

## Build Target

5 steps, each independently valuable. xtask crate is the main new artifact.

## Resolved Decisions

- SDK sub-crate build.rs files already reference canonical `wit/` — no changes needed.
- Only `patina-sdk` umbrella is published to crates.io. Sub-crates are workspace-internal.
- `wit/` stays at repo root. Not moved, not renamed.

## Commits

1. `refactor(sdk): delete dead mother-child WIT from SDK` — Remove `sdk/patina-sdk/wit/mother-child/`. Immediate cleanup.

2. `feat(xtask): add publish-sdk command` — Create `xtask/` crate. Command copies canonical `wit/` into SDK, verifies parity, runs `cargo publish --dry-run`. Add `xtask` to workspace members with `publish = false`.

3. `refactor(sdk): .gitignore SDK WIT copy` — Add `sdk/patina-sdk/wit/` to `.gitignore`. Remove tracked files with `git rm --cached`.

4. `refactor(wit): eliminate worlds/ toy mirrors or add build-time assertion` — Investigate whether wit-bindgen requires physical co-location. Implement chosen option. Delete `resources/scripts/check-wit-toys-sync.sh`.

5. `test: verify full workspace + children build after WIT consolidation` — Clean build, all children compile, xtask dry-run passes.

## Direct Code Targets

- `sdk/patina-sdk/wit/mother-child/` — delete
- `sdk/patina-sdk/wit/` — .gitignore, remove from tracking
- `xtask/` — new crate (Cargo.toml, src/main.rs)
- `Cargo.toml` — add xtask to workspace members
- `.gitignore` — add `sdk/patina-sdk/wit/`
- `resources/scripts/check-wit-toys-sync.sh` — delete after replacement
- `wit/worlds/` — investigate mirror strategy

## xtask Design Sketch

```rust
// xtask/src/main.rs — publish-sdk command
// 1. Copy canonical wit/ into sdk/patina-sdk/wit/ (selective: toys, worlds, deps, child worlds)
// 2. Strip dead worlds (mother-child)
// 3. Verify byte-for-byte parity with canonical source
// 4. Run cargo publish -p patina-sdk [--dry-run]
// 5. Clean up generated copy
```

## Verification Plan

```bash
cargo check --workspace -q
cargo test -q
git ls-files sdk/patina-sdk/wit/ | wc -l  # 0
cargo xtask publish-sdk --dry-run
cargo check -p patina-ai-child-doctor
```

## Build Readiness

Steps 1-3 are straightforward. Step 4 needs investigation of wit-bindgen requirements for `wit/worlds/` file co-location.

## Open Questions

- Does wit-bindgen require toy WIT files to be physically present alongside world WIT files in `wit/worlds/`? If yes, keep mirrors but add build-time assertion.
- Should xtask handle sub-crate publishing if they're ever published individually?
