---
type: feat
id: grammar-extraction
status: ready
created: 2026-02-14
sessions:
  origin: 20260214-130235
blocked_by: []
related:
- layer/surface/build/feat/patina-sdk/SPEC.md
- layer/surface/architecture-patina-metal.md
- layer/core/patina-identity.md
beliefs:
- patina-is-knowledge-protocol
- graceful-extraction
- separate-worlds-for-isolation
- fix-architecture-not-documentation
---

# feat: Grammar Extraction — Grammars as Pipeline Plugins

> Extract patina-metal's grammar bundle into individual pipeline plugins.
> Fold the routing infrastructure into patina-ai core. Delete
> patina-metal as a crate. Each grammar becomes an independent WASM
> plugin that bundles its own parser — no version conflicts, no binary
> bloat, no recompile to add a language.

## Problem

patina-metal compiles 9 language grammars (8 tree-sitter + 1 Cairo)
into the patina binary. This causes five real problems:

1. **60MB grammar bundle blocks crates.io publishing.** Session
   [[20260127-085434]] hit the 10MB package limit. Current workaround:
   distribute as pre-built binaries, not crates.

2. **52MB binary when goal is 10-15MB.** ONNX runtime + grammar bundle
   dominate. Users who only parse Rust still carry Go, Solidity, Cairo.

3. **Version conflict hell.** Session [[20250901-135830]] and
   `layer/surface/architecture-patina-metal.md` document tree-sitter
   0.23 vs 0.24 "links" conflicts. Solved by building all grammars
   from C source (git submodules, 152-line build.rs). Plugins solve
   this permanently — each plugin bundles its own parser version.

4. **Adding a language = recompile patina.** New grammar needs: git
   submodule, build.rs entry, FFI binding, language processor module,
   full binary rebuild. Should be: write a plugin, install it.

5. **Grammars are not protocol.** patina-identity.md explicitly says:
   "Load grammars as WASM plugins from `~/.patina/pipeline/`, not
   compile 30 languages into the binary." The protocol is capture,
   index, search, believe, evolve. Which languages you can parse is
   tooling.

## Current State

### Parser Technologies (Not All Tree-sitter)

| Language | Parser | Technology | Binary Impact |
|----------|--------|-----------|---------------|
| Rust | tree-sitter-rust | C via FFI | ~2MB |
| Go | tree-sitter-go | C via FFI | ~1MB |
| Python | tree-sitter-python | C via FFI | ~1MB |
| JavaScript | tree-sitter-javascript | C via FFI | ~1MB |
| TypeScript | tree-sitter-typescript | C via FFI | ~1MB |
| TSX | tree-sitter-tsx | C via FFI | ~1MB |
| Solidity | tree-sitter-solidity | C via FFI | ~1MB |
| C | tree-sitter-c | C via FFI | ~1MB |
| C++ | tree-sitter-cpp | C via FFI | ~1MB |
| **Cairo** | **cairo-lang-parser** | **Rust crates** | **~5MB** |

Cairo is architecturally different — it uses the official Cairo
compiler's parser (`cairo-lang-parser`, `cairo-lang-syntax`,
`cairo-lang-filesystem`), not tree-sitter. The extraction must handle
both parser technologies.

### What patina-metal Provides

Two distinct things:

1. **Routing infrastructure** (small, stays in patina-ai):
   - `Metal` enum — language detection from file extensions
   - Extension → language mapping
   - `Analyzer` — unified parsing interface

2. **Grammar implementations** (large, becomes plugins):
   - 8 tree-sitter grammars (C source, FFI bindings)
   - 1 Cairo parser (Rust crates, `CairoParser`)
   - Per-language query logic (symbol extraction, complexity)

### Plugin Dispatch (Already Built)

`src/commands/scrape/code/extract_v2.rs` already implements
plugin-first dispatch:

```
1. Scan file → detect language from extension
2. Check pipeline plugins for extension claim
3. If plugin exists → dispatch to WASM plugin
4. If no plugin → fall back to compiled grammar
5. If no fallback → skip file with warning
```

This is [[graceful-extraction]] in action. The host-side plumbing
exists. We need to build the plugin-side.

## Design

### Principle: Parser-Agnostic Plugins

Grammar plugins are NOT limited to tree-sitter. The pipeline world's
`handle(request: string) -> result<string, string>` interface is
parser-agnostic. A plugin receives source code and returns structured
extraction data as JSON. The plugin can use any parser technology
internally:

- Tree-sitter (C, compiled to WASM via wasi-sdk)
- Cairo-lang-parser (Rust, compiled to wasm32-wasip2 natively)
- Pest, nom, or any Rust parser combinator
- Hand-written parsers
- Future: WASM-native tree-sitter (upstream support maturing)

### Phase 1: Spike — One Grammar as Plugin

Build the Rust grammar as a pipeline plugin to prove the pattern.
Rust is the right choice because:
- Self-hosting (patina is Rust, must always parse Rust)
- Well-tested tree-sitter grammar
- Validates the full chain: build → install → dispatch → extract

**Spike deliverables:**
- `grammar-rust/` plugin project (built with patina-sdk pipeline)
- Compiles tree-sitter-rust to wasm32-wasip2
- Handles `parse` operation, returns `ExtractedData`-compatible JSON
- Performance benchmark: plugin vs compiled-in
- Decision: acceptable overhead for plugin dispatch?

**Technical risks to validate:**
- Tree-sitter C code compiling to wasm32-wasip2 (via wasi-sdk/clang)
- Parser initialization cost per request (pipeline plugins are
  stateless)
- WASM component size (tree-sitter grammar + runtime)

### Phase 2: Cairo Plugin (Non-Tree-sitter Proof)

Build the Cairo grammar as a pipeline plugin using cairo-lang-parser.
This proves the system works for non-tree-sitter parsers.

Cairo is actually simpler than tree-sitter for WASM because it's
pure Rust — no C FFI, no wasi-sdk. Just:
```toml
[dependencies]
patina-sdk = { version = "0.21", features = ["pipeline"] }
cairo-lang-parser = "2.12"
cairo-lang-syntax = "2.12"
cairo-lang-filesystem = "2.12"
```

Compile to `wasm32-wasip2` and the parser works in WASM natively.

**Key question:** Do the cairo-lang crates compile to wasm32-wasip2?
They're pure Rust with no native deps (as far as we know), but this
needs verification.

### Phase 3: Extract Remaining Grammars

Once both patterns are proven (tree-sitter and non-tree-sitter),
extract the remaining 7 grammars as individual plugins:

```
grammar-go/         (tree-sitter)
grammar-python/     (tree-sitter)
grammar-javascript/ (tree-sitter)
grammar-typescript/  (tree-sitter, handles .ts + .tsx)
grammar-solidity/    (tree-sitter)
grammar-c/          (tree-sitter, handles .c + .h)
grammar-cpp/        (tree-sitter, handles .cpp + .cc + .hpp + etc)
```

TypeScript and TSX may merge into one plugin (same upstream grammar
repo, different entry points). C and C++ may merge similarly. Let
the natural boundaries emerge during extraction.

### Phase 4: Fold Infrastructure, Delete Metal

Once all 9 grammars are plugins:

1. **Move routing into patina-ai** — `Metal` enum, extension mapping,
   `Language` detection. This is ~100 lines of pattern matching, not
   a separate crate.

2. **Keep Rust as compiled fallback** — Per [[graceful-extraction]],
   patina must always parse Rust even with zero plugins installed.
   The Rust grammar stays compiled into patina-ai (or a minimal
   built-in fallback parser).

3. **Remove patina-metal from workspace** — Delete the crate, the
   grammars/ submodule directory, the 152-line build.rs. The workspace
   loses ~60MB of C source.

4. **Update patina-ai Cargo.toml** — Remove `patina-metal` dependency.
   Binary size drops significantly.

### Plugin Distribution

Grammar plugins ship via the mechanism built in [[patina-sdk]] and
[[plugin-distribution]]:

```bash
# First run — install common grammars
patina setup grammars

# Or install individually
patina plugin install grammar-python
patina plugin install grammar-go

# Community grammars (new languages patina never supported)
patina plugin install grammar-zig
patina plugin install grammar-elixir
```

`patina setup grammars` installs a default set (Rust, Python, Go,
JS/TS) to `~/.patina/pipeline/`. Users add or remove grammars for
their stack.

### Request/Response Format

Pipeline plugins receive and return JSON envelopes:

**Request:**
```json
{
  "version": "1",
  "op": "parse",
  "payload": {
    "source": "fn main() { ... }",
    "path": "src/main.rs",
    "language": "rust"
  }
}
```

**Response:**
```json
{
  "version": "1",
  "data": {
    "functions": [...],
    "types": [...],
    "imports": [...],
    "symbols": [...],
    "complexity": 42
  }
}
```

The `data` schema matches what `ExtractedData` currently expects.
Plugins serialize to this format. The host deserializes. Parser
technology is invisible to the host.

### Version Conflict: Permanently Solved

Each grammar plugin bundles its own parser. Plugin A can use
tree-sitter 0.24, plugin B can use tree-sitter 0.25, plugin C
can use cairo-lang-parser 3.0. No "links" conflicts. No monolithic
build.rs. No git submodule coordination.

This is the structural fix for the problem that `architecture-patina-metal.md`
documented. The compiled-from-source approach was a tactical fix (it
worked). Plugins are the strategic fix (the problem cannot recur).

## Naming: crates.io Awareness

The `patina` crate on crates.io is a UEFI firmware SDK (unrelated).
They use underscore naming (`patina_macro`, `patina_stacktrace`).
Our crates use hyphen naming (`patina-ai`, `patina-sdk`). Low
confusion risk, but worth noting:

- `patina-ai` — clearly distinct (already our package name)
- `patina-sdk` — slightly generic but "SDK for building Patina WASM
  plugins" in the description disambiguates
- Grammar plugins would NOT be published to crates.io — they're
  WASM binaries distributed via `patina plugin install`, not Rust
  library crates

## Files to Change

```
# Phase 1: Spike
grammar-rust/                     # New plugin project (outside workspace)
grammar-rust/Cargo.toml           # patina-sdk pipeline + tree-sitter deps
grammar-rust/plugin.toml          # provides: languages = ["rs"]
grammar-rust/src/lib.rs           # PipelinePlugin impl, tree-sitter parse

# Phase 2: Cairo
grammar-cairo/                    # New plugin project
grammar-cairo/Cargo.toml          # patina-sdk pipeline + cairo-lang deps
grammar-cairo/plugin.toml         # provides: languages = ["cairo"]
grammar-cairo/src/lib.rs          # PipelinePlugin impl, cairo parser

# Phase 3: Remaining grammars (7 more plugin projects)

# Phase 4: Fold and delete
src/commands/scrape/code/metal.rs       # Metal enum + extension mapping (from patina-metal)
src/commands/scrape/code/extract_v2.rs  # Update fallback path
Cargo.toml                              # Remove patina-metal from workspace + deps
patina-metal/                           # DELETE entire crate
grammars/                               # DELETE submodule directory
```

## Build Order

1. **Spike: Rust grammar plugin** — Prove tree-sitter compiles to
   WASM, benchmark dispatch overhead, validate extraction format.
   Decision gate: if overhead > 10x, reconsider approach.

2. **Cairo grammar plugin** — Prove non-tree-sitter parsers work
   as plugins. Validate cairo-lang crates compile to wasm32-wasip2.

3. **Extract remaining 7 grammars** — Mechanical once pattern proven.
   One commit per grammar or batch similar ones.

4. **Fold Metal routing into patina-ai** — Move extension mapping,
   keep Rust fallback, delete patina-metal crate.

5. **Update docs and specs** — architecture-patina-metal.md becomes
   historical. patina-identity.md extraction table updated.

## Exit Criteria

### Critical
- [ ] At least one tree-sitter grammar works as a pipeline plugin
      (Rust grammar, end-to-end: build → install → scrape → extract)
- [ ] At least one non-tree-sitter grammar works as a plugin (Cairo)
- [ ] `patina scrape` produces identical extraction output with
      grammar plugins vs compiled-in (same symbols, functions, types)
- [ ] patina-metal removed from workspace

### Important
- [ ] All 9 current grammars available as pipeline plugins
- [ ] Binary size drops to <20MB (from 52MB)
- [ ] `patina setup grammars` installs default grammar set
- [ ] Performance within 5x of compiled-in (acceptable for scrape —
      not real-time editing)
- [ ] Version conflict impossible (each plugin bundles own parser)

### Nice-to-have
- [ ] At least one new language grammar (Zig, Elixir, Swift) built
      by community pattern
- [ ] Grammar plugin template added to scaffold
      (`patina plugin init grammar-zig --world pipeline`)
- [ ] `patina scrape` reports which grammars are plugin vs fallback

### Pre-push
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace`
- [ ] `cargo test --workspace`

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-14 | draft | Designed from session discussion. Extracts 9 grammars (8 tree-sitter + 1 Cairo) from patina-metal into pipeline plugins. Parser-agnostic design — plugins bring their own parser technology. Blocked by [[patina-sdk]] (plugins need the SDK to build). |
