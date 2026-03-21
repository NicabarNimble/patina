# Phase 2 Binary Size Measurement

Phase 2 commit 12 requires a release-size comparison against the Phase 1 baseline.

## Method

- Baseline commit: `19a989aa` (end of Phase 1)
- Current commit under test: `38de6fd9` (after Phase 2 commit 11)
- Built both children in release mode in both environments.

Commands used:

```bash
# Baseline (separate worktree at 19a989aa)
cargo component build --release --manifest-path children/ducklake/Cargo.toml
cargo component build --release --manifest-path children/belief-verifier/Cargo.toml
cargo build --release --target wasm32-wasip2 -p patina-ai-child-ducklake -p patina-ai-child-belief-verifier

# Current
cargo component build --release --manifest-path children/ducklake/Cargo.toml
cargo component build --release --manifest-path children/belief-verifier/Cargo.toml
cargo build --release --target wasm32-wasip2 -p patina-ai-child-ducklake -p patina-ai-child-belief-verifier
```

## Results

### `cargo component build --release` (`wasm32-wasip1` artifacts)

| Artifact | Baseline bytes | Current bytes | Delta |
| --- | ---: | ---: | ---: |
| `patina_ai_child_ducklake.wasm` | 261,703 | 261,699 | -4 |
| `patina_ai_child_belief_verifier.wasm` | 239,345 | 239,351 | +6 |
| **Total** | **501,048** | **501,050** | **+2** |

### `cargo build --release --target wasm32-wasip2`

| Artifact | Baseline bytes | Current bytes | Delta |
| --- | ---: | ---: | ---: |
| `patina_ai_child_ducklake.wasm` | 268,980 | 268,987 | +7 |
| `patina_ai_child_belief_verifier.wasm` | 247,445 | 247,458 | +13 |
| **Total** | **516,425** | **516,445** | **+20** |

## Interpretation

Binary size is effectively flat for Phase 2 (byte-level noise, no material growth).
The expected reduction is not realized yet. This is consistent with Phase 2 delivering
toolchain/world wiring but not yet removing all monolithic compatibility paths.
