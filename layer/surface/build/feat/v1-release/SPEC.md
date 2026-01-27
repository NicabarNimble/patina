---
type: feat
id: v1-release
status: in_progress
created: 2026-01-27
updated: 2026-01-27
sessions:
  origin: 20260127-085434
  work: []
related:
  - spec/go-public
milestones:
  - version: "1.0.0"
    name: Distribution-ready release
    status: pending
current_milestone: "1.0.0"
---

# feat: v1.0 Release

> Ship Patina as a distributable product. Small binary, easy install, no build-from-source friction.

**Goal:** Users install Patina in seconds, not minutes. Binary is slim, heavy assets download on demand, homebrew makes it one command.

---

## The Problem

Today Patina is 52MB with everything baked in — ONNX Runtime, tree-sitter grammars (8 languages), embedding infrastructure. Installing from source requires compiling C/C++ grammars and linking ONNX. There's no pre-built binary or package manager support.

Crates.io publishing is blocked by `patina-metal` bundling 60MB of grammar source files (10MB crate limit).

---

## Packaging Architecture

### Current State

| Asset | Size | Bundled/Downloaded |
|-------|------|-------------------|
| Tree-sitter grammars (native C) | ~10-15MB in binary | Compiled at build time |
| ONNX Runtime | ~15-20MB in binary | Linked at build time |
| Embedding models (.onnx) | ~30-90MB | Downloaded at runtime |

### Target State

| Asset | Size | Strategy |
|-------|------|----------|
| Patina binary (slim) | ~5-10MB | Core CLI only |
| Tree-sitter grammars (.wasm) | ~10MB | Downloaded on demand |
| ONNX Runtime (.dylib) | ~15MB | Downloaded on demand |
| Embedding models (.onnx) | ~30-90MB | Downloaded on demand (existing) |

### Runtime Asset Management

```
patina (slim binary)
├── patina doctor        → checks what's installed, what's missing
├── patina setup         → downloads all runtime assets
│   ├── grammars/*.wasm     (~10MB, tree-sitter WASM)
│   ├── libonnxruntime.dylib (~15MB)
│   └── models/*.onnx       (~30-90MB, existing flow)
└── ~/.patina/lib/       → runtime assets directory
```

First run: `patina setup` or auto-download on first use.
Subsequent runs: instant, everything cached locally.

---

## WASM Grammars

Replace compiled-in C grammars with tree-sitter WASM modules loaded at runtime.

**Why WASM over native dylibs:**
- Portable across architectures (same .wasm on arm64 and x86_64)
- Sandboxed execution
- Smaller than native equivalents
- Tree-sitter has native WASM support

**Trade-off:** Slower parsing than native C. Acceptable for Patina's use case (scraping, not real-time editing).

**Migration path:**
1. Build .wasm grammars from existing vendored sources
2. Host on GitHub releases as `patina-grammars-v1.0.0.tar.gz`
3. Load via tree-sitter WASM runtime instead of compiled-in native code
4. Remove `patina-metal` C build step (or make it optional for dev builds)

---

## ONNX Runtime

Currently statically linked via `ort` crate's `download-binaries` feature.

**Target:** Ship without ONNX baked in. Download `libonnxruntime.dylib` on demand, load via `ORT_DYLIB_PATH`.

**`ort` supports this:** The crate can load a dynamic library at runtime instead of bundling one.

---

## Distribution Channels

### GitHub Releases (primary)
- Release workflow triggered on version tags (`v*`)
- Builds on macOS arm64 (mac only for now)
- Produces `patina-v{version}-aarch64-apple-darwin.tar.gz`
- Stripped binary with release profile optimizations

### Homebrew Tap
- Separate repo: `NicabarNimble/homebrew-tap`
- Formula points to GitHub release tarball
- Install: `brew install NicabarNimble/tap/patina`
- Formula updated on each release (manual or automated)

### Release Profile

```toml
[profile.release]
strip = true
lto = true
codegen-units = 1
opt-level = "z"  # optimize for size
```

---

## Exit Criteria

- [ ] WASM grammar loading replaces compiled-in C grammars
- [ ] ONNX Runtime loaded dynamically (not statically linked)
- [ ] `patina setup` downloads all runtime assets
- [ ] `patina doctor` reports asset status
- [ ] GitHub release workflow produces macOS arm64 binary
- [ ] Homebrew tap formula works (`brew install NicabarNimble/tap/patina`)
- [ ] Binary under 15MB (stripped, before compression)
- [ ] Tarball under 5MB compressed

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-01-27 | in_progress | Spec created. Current binary 52MB, 14MB compressed. |
