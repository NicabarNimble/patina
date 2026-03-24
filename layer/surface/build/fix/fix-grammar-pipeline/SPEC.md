---
type: fix
id: fix-grammar-pipeline
status: active
created: 2026-03-24
sessions:
  origin: 20260324-101606-299953000
exit_criteria:
- id: wasm-extracts-symbols
  text: patina repo add on a Rust-heavy repo extracts >0 symbols via WASM pipeline
  checked: false
- id: native-fallback-works
  text: Native Rust fallback parses files without ABI error when WASM grammars are absent
  checked: false
- id: consistent-child-naming
  text: All discovery paths and installer use child.toml + child.wasm consistently
  checked: false
- id: setup-grammars-deploys-correctly
  text: patina setup grammars --force deploys children that pipeline discovery loads
  checked: false
---
# fix: Fix WASM grammar child discovery and native Rust fallback

> Plugin-to-child rename broke WASM grammar discovery. The native Rust fallback is also broken due to tree-sitter ABI mismatch. Both code paths fail, leaving Patina unable to extract code symbols from any ref repo.

## Problem

After the plugin-to-child vocabulary refactor (v0.43.7, March 2026), `patina repo add` on any Rust-heavy repo produces zero code symbols. Every `.rs` file errors with "Failed to set Rust language". Other languages hit "No pipeline plugin for {lang}".

All repos added since the refactor have empty code indices. Lexical and git-history search work, but semantic code search is broken.

This violates **dependable-rust** (the public interface promises code extraction but silently delivers nothing) and **oxidized-knowledge** (the knowledge layer is incomplete without code symbols).

## Goal

Restore grammar extraction so that both the WASM pipeline path and the native Rust fallback work correctly. One commit per fix target, verifiable independently.

## Status

Draft. Root cause fully diagnosed. Ready for implementation.

## Non-Goals

- Renaming `plugin.wasm` to `child.wasm` everywhere in one shot (the WASM binary naming can be addressed separately; discovery already works with `plugin.wasm`)
- Adding new grammar children or languages
- Changing the graceful-extraction design (Rust fallback stays)

## Solution

Three surgical fixes, each independently verifiable:

### F1: Fix tree-sitter ABI mismatch in native Rust fallback

Pin `tree-sitter` and `tree-sitter-rust` to compatible versions in `Cargo.toml`. The native fallback exists per graceful-extraction design and must work.

**Target:** `Cargo.toml` (lines 80-81)

### F2: Fix lazy discovery path WASM filename

Change `extract_v2.rs:321` from `plugin.wasm` to `child.wasm` to match the manifest filename convention. Also update `pipeline.rs:226` for consistency.

**Targets:**
- `src/commands/scrape/code/extract_v2.rs` (line 321)
- `src/child/internal/pipeline.rs` (line 226)

### F3: Update setup grammars installer WASM filename

Change `setup/grammars.rs:65` from `plugin.wasm` to `child.wasm` so newly installed grammars use the child vocabulary consistently.

**Target:** `src/commands/setup/grammars.rs` (line 65)

### F4: Reinstall grammar children

Run `patina setup grammars --force` to redeploy from source templates (which have correct `child.toml` with `[child]` section). Then verify with `patina repo update` on a broken repo.

## Root Cause

**RC1: WASM grammar discovery broken by naming mismatch.**
Installed grammars (Feb 2026) have `plugin.toml` with `[plugin]` section. Commit `9b29a89b` (Mar 22) removed `[plugin]` fallback in `ChildManifest::from_path()`. Discovery returns empty map. Source templates already have correct `child.toml` with `[child]`, but grammars were never reinstalled.

**RC2: Native Rust fallback has tree-sitter ABI mismatch.**
`tree-sitter` resolved to 0.24.7, `tree-sitter-rust` to 0.24.0. `parser.set_language()` fails at runtime despite compiling. The fallback that should catch RC1 is itself broken.

## Implementation Order

1. F1 — Fix tree-sitter versions (unblocks native fallback immediately)
2. F2 — Fix WASM filename in discovery paths
3. F3 — Fix WASM filename in installer
4. F4 — Reinstall and verify end-to-end

## Resolved Decisions

- **Keep native Rust fallback.** Per graceful-extraction design, Patina must parse Rust with zero plugins. This is the right safety net.
- **No [plugin] fallback restoration.** The rename is done; old artifacts should be replaced, not accommodated. Fail loud per dependable-rust.
- **WASM binary rename (plugin.wasm to child.wasm) is in scope.** Discovery paths should use child vocabulary consistently. This is a small change across three files.

## Verification

```bash
# After F1: native fallback works
cargo test -- rust_processor  # or build and test manually

# After F2+F3: installer and discovery use consistent names
rg "plugin\.wasm" src/commands/scrape/code/extract_v2.rs src/child/internal/pipeline.rs src/commands/setup/grammars.rs
# Should return 0 matches

# After F4: end-to-end
patina setup grammars --force
patina repo update tempoxyz/mpp-rs
# Should show >0 symbols extracted
```

## Build Readiness

All root causes diagnosed. All target files read. No ambiguity in the fix. Each commit is a single-line or two-line change except F1 which is a version pin.

## Exit Criteria

See frontmatter exit_criteria.
