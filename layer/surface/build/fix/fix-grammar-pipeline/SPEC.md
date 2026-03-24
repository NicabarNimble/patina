---
type: fix
id: fix-grammar-pipeline
status: draft
created: 2026-03-24
sessions:
  origin: 20260324-101606-299953000
exit_criteria:
  - GF1: "`patina repo add` on a Rust-heavy repo extracts >0 symbols via WASM pipeline"
  - GF2: "Native Rust fallback parses files without ABI error when WASM grammars are absent"
  - GF3: "Lazy discovery path uses consistent naming (child.toml + child.wasm)"
  - GF4: "`patina setup grammars` deploys children with child.toml and child.wasm"
---
# fix: Fix WASM grammar child discovery and native Rust fallback

> Plugin→child rename broke WASM grammar discovery. The native Rust fallback is also broken due to tree-sitter ABI mismatch. Both code paths fail, leaving Patina unable to extract Rust symbols from any ref repo.

## Problem

After the plugin→child vocabulary refactor (v0.43.7, March 2026), `patina repo add` on any Rust-heavy repo produces zero code symbols. Every `.rs` file errors with "Failed to set Rust language". Other languages hit "No pipeline plugin for {lang}".

This means all three recently added repos (AbdelStark/llm-provable-computer, tempoxyz/wallet, tempoxyz/mpp-rs) and likely any repo added since the refactor have empty code indices — lexical/git-history search works, but semantic code search is completely broken.

## Root Cause

**Two independent failures that compound into total Rust extraction failure:**

### RC1: WASM grammar discovery broken by naming mismatch

The plugin→child refactor (commit `9b29a89b`, Mar 22) removed the `[plugin]` → `[child]` fallback in `ChildManifest::from_path()`. However, the installed WASM grammars at `~/.patina/pipeline/grammar-*/` were deployed in February and still have:
- `plugin.toml` (not `child.toml`) as the manifest filename
- `[plugin]` section (not `[child]`) inside the TOML
- `plugin.wasm` (not `child.wasm`) as the WASM binary filename

The discovery chain:
1. `extract_v2.rs:262` — `engine.discover()` scans `~/.patina/pipeline/`
2. `pipeline.rs:225` — finds `plugin.toml` via filename fallback (this part works)
3. `pipeline.rs:232` — calls `ChildManifest::from_path()` which requires `[child]` section
4. Parse fails → discovery returns empty map → no WASM plugins loaded

Additionally, the **lazy discovery path** (`extract_v2.rs:320-321`) hardcodes `child.toml` + `plugin.wasm` — mixed naming that wouldn't work with either old or new conventions.

### RC2: Native Rust fallback has tree-sitter ABI mismatch

When WASM discovery returns empty, `process_file_with_plugins()` falls through to `process_file_by_language()` (`extract_v2.rs:474`), which calls the compiled-in `RustProcessor`. This processor uses:
- `tree-sitter` 0.24.7 (parser runtime)
- `tree-sitter-rust` 0.24.0 (grammar)

`parser.set_language(&language)` fails at runtime due to ABI version incompatibility between these two crates, despite compiling successfully.

## Fix

### F1: Reinstall grammar children with correct naming

Update `patina setup grammars` to deploy with `child.toml` (with `[child]` section) and `child.wasm`. The source templates in `grammars/*/` already have the correct `child.toml` — the install command just needs to copy the right filenames.

### F2: Fix lazy discovery path naming consistency

In `extract_v2.rs:320-321`, change:
```rust
let manifest_path = path.join("child.toml");
let wasm_path = path.join("plugin.wasm");  // ← wrong
```
to:
```rust
let manifest_path = path.join("child.toml");
let wasm_path = path.join("child.wasm");
```

### F3: Fix tree-sitter ABI mismatch in native fallback

Bump `tree-sitter-rust` to match `tree-sitter` 0.24.7 ABI, or pin both to compatible versions. The native Rust fallback exists per `[[graceful-extraction]]` design — Patina must always parse Rust even with zero plugins installed — so it must actually work.

### F4: Verify end-to-end

Re-add or update one of the broken repos and confirm >0 symbols extracted via WASM pipeline.

## Exit Criteria

- **GF1**: `patina repo add` on a Rust-heavy repo extracts >0 symbols via WASM pipeline
- **GF2**: Native Rust fallback parses files without ABI error when WASM grammars are absent
- **GF3**: Lazy discovery path uses consistent naming (`child.toml` + `child.wasm`)
- **GF4**: `patina setup grammars` deploys children with `child.toml` and `child.wasm`
