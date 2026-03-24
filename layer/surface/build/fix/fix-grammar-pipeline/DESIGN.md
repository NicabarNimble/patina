# Design: fix-grammar-pipeline

## Why This Design

The plugin-to-child vocabulary migration left installed WASM grammars with stale naming. Rather than restoring backward compatibility (which hides drift), we complete the migration: fix the discovery and installer code to use child vocabulary, reinstall grammars, and fix the tree-sitter ABI mismatch that broke the safety net.

This aligns with:
- **dependable-rust**: small interface, no silent failures. If grammars are missing, fail loud.
- **spec-driven-design**: each fix traces to a diagnosed root cause.
- **safety-boundaries**: changes are project-scoped, no system-level side effects.

## Build Target

Restore code symbol extraction for all ref repos. Both WASM pipeline and native Rust fallback must work independently.

## Resolved Decisions

1. No `[plugin]` fallback in manifest parser — the rename is final.
2. Native Rust fallback stays — graceful-extraction design is sound.
3. WASM binary filename changes from `plugin.wasm` to `child.wasm` in discovery and installer for vocabulary consistency.

## Commits

### C1: `fix: pin tree-sitter and tree-sitter-rust to compatible ABI`
- **File:** `Cargo.toml` lines 80-81
- **Change:** Pin `tree-sitter-rust` to a version compatible with `tree-sitter` 0.24.7 ABI
- **Verify:** `cargo check` passes, native RustProcessor can set language

### C2: `fix: use child.wasm in pipeline discovery paths`
- **File:** `src/commands/scrape/code/extract_v2.rs` line 321
  - `plugin.wasm` → `child.wasm`
- **File:** `src/child/internal/pipeline.rs` line 226
  - `plugin.wasm` → `child.wasm`
- **Verify:** `rg "plugin\.wasm" src/commands/scrape src/child/internal/pipeline.rs` returns 0

### C3: `fix: use child.wasm in setup grammars installer`
- **File:** `src/commands/setup/grammars.rs` line 65
  - `plugin.wasm` → `child.wasm`
- **Verify:** `rg "plugin\.wasm" src/commands/setup/grammars.rs` returns 0

### C4: (manual, not a code commit) `chore: reinstall grammar children`
- Run `patina setup grammars --force`
- Verify `ls ~/.patina/pipeline/grammar-rust/` shows `child.toml` + `child.wasm`
- Run `patina repo update tempoxyz/mpp-rs` and confirm >0 symbols

## Direct Code Targets

| File | Lines | Change |
|------|-------|--------|
| `Cargo.toml` | 80-81 | Pin tree-sitter-rust version |
| `src/commands/scrape/code/extract_v2.rs` | 321 | `plugin.wasm` → `child.wasm` |
| `src/child/internal/pipeline.rs` | 226 | `plugin.wasm` → `child.wasm` |
| `src/commands/setup/grammars.rs` | 65 | `plugin.wasm` → `child.wasm` |

4 files, 4 lines changed. Scalpel.

## Verification Plan

1. `cargo check` — tree-sitter versions resolve and compile
2. `cargo build` — binary builds
3. `rg "plugin\.wasm" src/` — returns only non-discovery/installer references (if any)
4. `patina setup grammars --force` — deploys with child naming
5. `patina repo update tempoxyz/mpp-rs` — extracts >0 symbols
6. Remove `~/.patina/pipeline/grammar-rust/` and re-scrape — native fallback works without ABI error

## Build Readiness

All files read. All line numbers confirmed. No ambiguous decisions. Ready to cut.
