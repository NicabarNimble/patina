---
type: feat
id: grammar-extraction
status: active
created: 2026-02-14
revised: 2026-02-14
sessions:
  origin: 20260214-130235
  review: 20260214-170156
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
- parser-agnostic-interfaces
- structural-fixes-over-tactical
- gate-exports-on-target-arch
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

2. **69MB binary when goal is 10-15MB.** ONNX runtime + grammar bundle
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

## Current State (Verified by Code Review)

### Parser Technologies (Not All Tree-sitter)

| Language | Parser | Technology | parser.c Size |
|----------|--------|-----------|---------------|
| Rust | tree-sitter-rust | C via FFI | 5.8MB |
| Go | tree-sitter-go | C via FFI | 1.4MB |
| Python | tree-sitter-python | C via FFI | 3.3MB |
| JavaScript | tree-sitter-javascript | C via FFI | 2.4MB |
| TypeScript | tree-sitter-typescript | C via FFI | 8.3MB |
| TSX | tree-sitter-tsx | C via FFI | 8.4MB |
| Solidity | tree-sitter-solidity | C via FFI | 2.1MB |
| C | tree-sitter-c | C via FFI | 3.7MB |
| C++ | tree-sitter-cpp | C via FFI | 17MB |
| **Cairo** | **cairo-lang-parser** | **Rust crates** | **N/A** |

Cairo is architecturally different — it uses the official Cairo
compiler's parser (`cairo-lang-parser`, `cairo-lang-syntax`,
`cairo-lang-filesystem`), not tree-sitter. The extraction must handle
both parser technologies.

**Total C grammar source: ~52MB.** C++ alone is 17MB.

### What patina-metal Actually Provides (Code Review)

Three distinct things, not two:

1. **Metal enum + routing** (patina-metal/src/metal.rs, ~200 lines):
   - `Metal` enum — 9 variants for supported languages
   - `from_extension()` — extension → language detection
   - `tree_sitter_language()` / `tree_sitter_language_for_ext()` —
     returns the compiled tree-sitter Language for a metal
   - `normalize_node_kind()` — maps language-specific AST node types
     to generic categories (function, struct, trait, etc.)
   - `file_pattern()` — glob patterns for file discovery

2. **Grammar FFI bindings** (patina-metal/src/grammars.rs, ~50 lines):
   - `extern "C"` declarations for 8 tree-sitter grammars
   - Safe wrapper functions (e.g., `language_rust() -> Language`)
   - These link against C libraries compiled by build.rs

3. **Cairo parser** (patina-metal/src/cairo.rs, ~280 lines):
   - Full Cairo AST parser using `SimpleParserDatabase`
   - Extracts functions, structs, traits, impls, modules, imports
   - Returns `CairoSymbols` struct (not `ExtractedData`)

**NOT heavily used by scrape:** The `Analyzer` struct in lib.rs and
the `queries.rs` module (which only has .scm files for Rust and Go)
are largely unused. The actual extraction happens in the language
processors, not through Analyzer.

### Where the Real Extraction Logic Lives

**NOT in patina-metal.** The heavy lifting is in
`src/commands/scrape/code/languages/` — 9 per-language processor
modules (each 200-400 lines):

```
src/commands/scrape/code/languages/
├── rust.rs          # RustProcessor::process_file()
├── go.rs            # GoProcessor::process_file()
├── python.rs        # PythonProcessor::process_file()
├── javascript.rs    # JavaScriptProcessor::process_file()
├── typescript.rs    # TypeScriptProcessor::process_file()
├── solidity.rs      # SolidityProcessor::process_file()
├── cairo.rs         # CairoProcessor::process_file()
├── c.rs             # CProcessor::process_file()
├── cpp.rs           # CppProcessor::process_file()
└── mod.rs           # Language enum (duplicate of Metal!)
```

Each processor:
1. Gets a tree-sitter Language from `patina_metal::Metal::X`
2. Creates its own `tree_sitter::Parser`
3. Parses source code into an AST
4. Walks the AST with custom extraction logic
5. Returns `ExtractedData` (symbols, functions, types, imports,
   call_edges, constants, members)

**A grammar plugin replaces a language processor**, not the
patina-metal Analyzer. The plugin must implement the same extraction
logic that the processor does today.

### Dual Enum Problem

There are TWO language enums that do the same thing:
- `patina_metal::Metal` (metal.rs) — used by language processors
- `scrape::code::languages::Language` (languages/mod.rs) — used by
  extract_v2.rs dispatch

Both map identical file extensions. When Metal folds into patina-ai,
these must unify into a single enum.

### Query Files (Only 2 of 9)

Only Rust and Go have `.scm` query files in `patina-metal/queries/`.
The other 7 languages have no queries. The extraction logic for ALL
9 languages lives in the per-language processors, not in tree-sitter
queries. This is important — the plugins need to port the processor
logic, not query files.

### Plugin Dispatch (Already Built)

`src/commands/scrape/code/extract_v2.rs` already implements
plugin-first dispatch:

```
1. Scan file → detect language from extension (Language enum)
2. Check pipeline plugins for extension claim (HashMap<String, Plugin>)
3. If plugin exists → build JSON envelope → dispatch to WASM plugin
4. Parse response as ExtractedData (serde_json::from_str)
5. If plugin fails → fall back to compiled language processor
6. If no processor → skip file with error
```

This is [[graceful-extraction]] in action. The host-side plumbing
exists and is tested. We just need actual plugins.

### ExtractedData Schema (The Plugin Contract)

The response JSON must deserialize to `ExtractedData`:

```rust
pub struct ExtractedData {
    pub symbols: Vec<CodeSymbol>,     // name, kind, path, line, signature
    pub functions: Vec<FunctionFact>, // name, file, params, return_type, complexity
    pub types: Vec<TypeFact>,         // name, file, kind, fields
    pub imports: Vec<ImportFact>,     // file, import_path, alias
    pub call_edges: Vec<CallGraphEntry>, // caller, callee, file, line, call_type
    pub constants: Vec<ConstantFact>, // name, file, value, const_type, scope
    pub members: Vec<MemberFact>,     // container, name, file, member_type, visibility
}
```

This is the stable contract. All 7 field types derive
`serde::Serialize` + `serde::Deserialize`. The host already does
`serde_json::from_str::<ExtractedData>(&response)` (extract_v2.rs:241).

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

### Critical Feasibility: C Code to wasm32-wasip2

Tree-sitter grammar plugins need C code compiled to WASM. Two C
compilation steps are required:

1. **tree-sitter runtime** — the `tree-sitter` crate's build.rs
   compiles `src/lib.c` via `cc::Build` (~2000 lines of C)
2. **Grammar C sources** — each grammar's `parser.c` + optional
   `scanner.c` compiled in the plugin's own build.rs via `cc::Build`

The `cc` crate supports cross-compilation to wasm32-wasip2 when
a WASM-capable C compiler is available. **wasi-sdk** provides this.

**Setup for grammar plugin authors:**
```bash
# Install wasi-sdk (one-time)
# macOS: download from https://github.com/WebAssembly/wasi-sdk/releases
export WASI_SDK_PATH=/opt/wasi-sdk

# In plugin project's .cargo/config.toml:
[target.wasm32-wasip2]
linker = "/opt/wasi-sdk/bin/wasm-ld"

[env]
CC_wasm32_wasip2 = { value = "/opt/wasi-sdk/bin/clang", force = true }
AR_wasm32_wasip2 = { value = "/opt/wasi-sdk/bin/llvm-ar", force = true }
```

**Prior art:** Zed editor uses this exact approach — their extension
system compiles tree-sitter grammars to WASM via wasi-sdk. Commits
in the zed-industries/zed ref repo confirm this:
- "Bump Tree-sitter for bug fixes affecting YAML parser loaded via WASM"
- "Compile and instantiate wasm modules on a background thread"
- "Bump Tree-sitter for inclusion of strncat in wasm c stdlib"

**End users never compile grammars.** They install pre-built `.wasm`
binaries via `patina plugin install`. Only plugin authors need wasi-sdk.

### Phase 1: Spike — Cairo Grammar Plugin (Pure Rust)

**Changed from original:** Spike Cairo first, not Rust. Cairo is pure
Rust — no wasi-sdk needed, no C toolchain variable. This proves the
end-to-end pipeline plugin pattern cleanly.

Cairo is the right first choice because:
- Pure Rust → `cargo build --target wasm32-wasip2` with no C toolchain
- Validates the full chain: build → install → dispatch → extract
- Isolates the plugin pattern from the C-compilation question
- The existing CairoProcessor (languages/cairo.rs) is the reference

**Spike deliverables:**
- `grammar-cairo/` plugin project (outside workspace, uses patina-sdk)
- Compiles to wasm32-wasip2 as a WASM component
- Handles `parse` operation, returns `ExtractedData` JSON
- Install to `~/.patina/pipeline/grammar-cairo/`
- `patina scrape` dispatches to plugin, produces same output as
  the compiled-in CairoProcessor

**Risk to validate:** Do cairo-lang-parser, cairo-lang-syntax,
cairo-lang-filesystem compile to wasm32-wasip2? They're pure Rust
(Salsa incremental computation framework, no native deps), but the
dependency tree is large. Plugin binary size could be substantial.

**Decision gate:** If cairo-lang crates don't compile to wasm32-wasip2
(unlikely but possible), Cairo stays compiled-in as a special case
and we proceed with tree-sitter grammars only.

### Phase 2: Rust Grammar Plugin (Tree-sitter + wasi-sdk)

Build the Rust grammar as a pipeline plugin to prove the tree-sitter
pattern. This introduces the C-compilation variable.

**Deliverables:**
- `grammar-rust/` plugin project with wasi-sdk build configuration
- Bundles tree-sitter runtime + tree-sitter-rust grammar C source
- build.rs compiles parser.c + scanner.c via cc crate targeting wasm32
- Handles `parse` operation with full extraction logic from
  `languages/rust.rs` (the richest processor — constants, members,
  call edges)
- Performance benchmark: plugin vs compiled-in
- Document wasi-sdk setup in README

**Technical risks:**
- wasi-sdk availability and setup friction for developers
- Parser init cost per request (pipeline plugins are stateless —
  each `handle()` creates a new Parser)
- WASM component size (tree-sitter runtime + 5.8MB parser.c compiled)
- tree-sitter's `src/lib.c` compatibility with WASM (Zed proves this
  works, but version-specific issues possible)

**Decision gate:** If overhead > 10x of compiled-in, reconsider.
If wasi-sdk setup is too painful, consider `zig cc` as alternative.

### Phase 3: Extract Remaining Grammars

Once both patterns are proven (pure Rust and tree-sitter + C),
extract the remaining 7 grammars. Each plugin ports its corresponding
language processor from `src/commands/scrape/code/languages/`.

```
grammar-go/         ports languages/go.rs          (1.4MB parser.c)
grammar-python/     ports languages/python.rs      (3.3MB parser.c)
grammar-javascript/ ports languages/javascript.rs  (2.4MB parser.c)
grammar-typescript/ ports languages/typescript.rs   (8.3MB + 8.4MB TS/TSX)
grammar-solidity/   ports languages/solidity.rs     (2.1MB parser.c)
grammar-c/          ports languages/c.rs            (3.7MB parser.c)
grammar-cpp/        ports languages/cpp.rs          (17MB parser.c)
```

**TypeScript plugin handles both .ts and .tsx** — same upstream
grammar repo, dual parser entry points. Claims languages = ["ts", "tsx"].

**C and C++ stay separate** — very different parser sizes (3.7MB vs
17MB) and different AST node kinds. No benefit to merging.

### Phase 4: Fold Infrastructure, Delete Metal

Once all 9 grammars are plugins:

1. **Unify Language enums** — Merge `Metal` (from patina-metal) and
   `Language` (from scrape/code/languages/mod.rs) into a single enum
   in patina-ai. Keep the extension mapping, drop the tree-sitter
   language lookup (plugins own that now).

2. **Move `normalize_node_kind()` into plugins** — Each plugin
   normalizes its own AST node types before serializing to
   ExtractedData. The host doesn't need language-specific knowledge.

3. **Keep Rust as compiled fallback** — Per [[graceful-extraction]],
   patina must always parse Rust even with zero plugins installed.
   Keep `languages/rust.rs` + tree-sitter-rust compiled-in. Remove
   the other 8 language processors.

4. **Remove patina-metal from workspace** — Delete the crate, the
   grammars/ directory (52MB+ of C source), the 152-line build.rs,
   the FFI bindings, the Analyzer, the query files.

5. **Update patina-ai Cargo.toml** — Remove `patina-metal` dependency.
   Remove `tree-sitter` dep from patina-ai if only used through metal
   (check: rust.rs fallback still needs it).

### Phase 5: Setup Grammars Command

`patina setup grammars` — first-run convenience that installs the
default grammar plugin set to `~/.patina/pipeline/`.

**Problem:** After Phase 4, a fresh patina install can only parse
Rust. Running `patina scrape` on a Python repo gives "No pipeline
plugin for Python — install with `patina plugin install`." But
`patina plugin install` doesn't exist yet either. Users must
manually copy plugin.wasm + plugin.toml to the right directory.
This is unacceptable for v1.0.

**Design:**

The grammar plugins are already built as WASM binaries (~0.5-2MB
each). They live in the `grammar-*/` directories in this repo.
`patina setup grammars` needs to get them into `~/.patina/pipeline/`.

**Source of truth for default grammars:**

```toml
# resources/grammar-defaults.toml
[grammars]
default = ["rust", "go", "python", "javascript", "typescript", "c", "cpp", "solidity", "cairo"]
```

**Two installation sources (checked in order):**

1. **Local build artifacts** — if `grammar-*/` directories exist
   adjacent to the patina binary (dev/contributor workflow), copy
   from there. This is how contributors who build from source get
   grammars.

2. **GitHub releases** — download pre-built `.wasm` + `plugin.toml`
   from the patina GitHub releases page. Each release tags grammar
   plugin binaries as assets. This is how end users get grammars.

**Command behavior:**

```bash
patina setup grammars              # Install all defaults
patina setup grammars --list       # Show what would be installed
patina setup grammars --only go,py # Install specific grammars
patina setup grammars --force      # Reinstall (overwrite existing)
```

For each grammar:
1. Check if already installed in `~/.patina/pipeline/grammar-<lang>/`
2. Skip if present (unless `--force`)
3. Copy or download `plugin.wasm` + `plugin.toml`
4. Verify: load manifest, check world = "pipeline"
5. Report: "Installed grammar-go v0.1.0 (598KB)"

**Output:**
```
Installing default grammar plugins to ~/.patina/pipeline/...
  grammar-rust      v0.1.0  ✓ already installed
  grammar-go        v0.1.0  ✓ installed (598KB)
  grammar-python    v0.1.0  ✓ installed (1.2MB)
  ...
  9/9 grammars ready
```

**Integration with `patina init`:**

When `patina init .` runs on a new project and no grammar plugins
are installed, print a hint:

```
Hint: No grammar plugins found. Run `patina setup grammars` to
install language parsers for scraping.
```

**Files to change:**

```
resources/grammar-defaults.toml           # Default grammar manifest
src/commands/setup.rs                     # New: setup command
src/commands/setup/grammars.rs            # New: grammar installer
src/commands/mod.rs                       # Register setup command
src/main.rs                              # Wire CLI subcommand
```

**Exit criteria:**
- `patina setup grammars` installs all 9 grammars to `~/.patina/pipeline/`
- Idempotent — running twice doesn't duplicate or error
- Works from local build artifacts (dev workflow)
- `patina scrape` works on any supported language after setup

### Phase 6: Performance Benchmark

Measure plugin dispatch overhead vs the compiled-in baseline that
existed before Phase 4. The spec requires "within 5x."

**Problem:** We deleted the compiled-in processors. We can't A/B
test anymore. But we have the `grammar-compare.sh` timing data and
can construct a focused benchmark.

**Method:**

Benchmark the one language where we DO have both paths: **Rust**.
`rust.rs` is compiled-in. `grammar-rust` is a WASM plugin. Both
produce identical output (verified by grammar-compare.sh). Parse
the same set of files through each path and compare wall-clock time.

```bash
patina bench grammar                    # Run the benchmark
patina bench grammar --files 100        # Limit file count
patina bench grammar --language rs      # Specific language
```

**Implementation:**

1. Collect N Rust source files from the current repo (or a ref repo)
2. **Compiled-in path:** Call `RustProcessor::process_file()` directly
   for each file. Measure total time.
3. **Plugin path:** Call `PipelineEngine::handle()` for each file.
   Measure total time. This includes: JSON envelope build → WASM
   instantiation → plugin parse → JSON response deserialize.
4. Report: files processed, total time each path, ratio, per-file
   average.

**Output:**
```
Grammar performance benchmark (195 Rust files):

  Compiled-in:  1.2s  (6.2ms/file)
  Plugin WASM:  3.8s  (19.5ms/file)
  Overhead:     3.2x

  ✓ Within 5x threshold
```

**Decision gates:**
- If overhead <= 5x: PASS. Document the number, move on.
- If overhead 5-10x: ACCEPTABLE for batch scrape. Note in docs.
  Consider parser caching in WasmCell if it's WASM instantiation
  cost (not parse cost).
- If overhead > 10x: INVESTIGATE. Profile where time goes. WASM
  compile? JSON serde? Parser init? Fix or document the tradeoff.

**Files to change:**

```
src/commands/bench.rs                    # Add grammar subcommand
src/commands/bench/grammar.rs            # New: grammar benchmark
```

**Exit criteria:**
- Benchmark runs and produces a clear overhead ratio
- Result documented in this spec's status log
- If > 5x, root cause identified and documented

### Plugin Distribution

Grammar plugins ship as pre-built WASM binaries, NOT as crates.io
packages. Distribution via the mechanism built in [[patina-sdk]]:

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

**Request** (already built in extract_v2.rs `build_parse_envelope()`):
```json
{
  "op": "parse",
  "version": "1",
  "payload": {
    "source": "fn main() { ... }",
    "language": "rs"
  }
}
```

**Response** (must deserialize to `ExtractedData`):
```json
{
  "symbols": [
    {"path": "./src/main.rs", "name": "main", "kind": "function",
     "line": 1, "end_line": 3, "signature": "fn main()"}
  ],
  "functions": [
    {"file": "./src/main.rs", "name": "main", "params": "",
     "return_type": "", "complexity": 1, "start_line": 1,
     "end_line": 3, "is_public": false}
  ],
  "types": [],
  "imports": [],
  "call_edges": [],
  "constants": [],
  "members": []
}
```

Note: `payload.language` is the file extension (e.g., "rs", "py"),
NOT the language name. This matches how `pipeline_plugins.get(ext)`
dispatches in extract_v2.rs. The `path` field is NOT in the request
envelope today — plugins receive source code only, file path context
comes from the host. If plugins need path for their ExtractedData
output, we add it to the payload.

### Version Conflict: Permanently Solved

Each grammar plugin bundles its own parser. Plugin A can use
tree-sitter 0.24, plugin B can use tree-sitter 0.25, plugin C
can use cairo-lang-parser 3.0. No "links" conflicts. No monolithic
build.rs. No git submodule coordination.

This is the structural fix for the problem that `architecture-patina-metal.md`
documented. The compiled-from-source approach was a tactical fix (it
worked). Plugins are the strategic fix (the problem cannot recur).

## Files to Change

```
# Phase 1: Cairo spike (pure Rust, no wasi-sdk)
grammar-cairo/                    # New plugin project (outside workspace)
grammar-cairo/Cargo.toml          # patina-sdk pipeline + cairo-lang deps
grammar-cairo/.cargo/config.toml  # wasm32-wasip2 target config
grammar-cairo/plugin.toml         # provides: languages = ["cairo"]
grammar-cairo/src/lib.rs          # PipelinePlugin impl, ports languages/cairo.rs

# Phase 2: Rust spike (tree-sitter + wasi-sdk)
grammar-rust/                     # New plugin project (outside workspace)
grammar-rust/Cargo.toml           # patina-sdk pipeline + tree-sitter deps
grammar-rust/.cargo/config.toml   # wasi-sdk CC/AR configuration
grammar-rust/build.rs             # cc::Build for parser.c + scanner.c
grammar-rust/plugin.toml          # provides: languages = ["rs"]
grammar-rust/src/lib.rs           # PipelinePlugin impl, ports languages/rust.rs
grammar-rust/grammars/rust/src/   # Copy of tree-sitter-rust C source

# Phase 3: Remaining grammars (7 more plugin projects, same pattern)

# Phase 4: Fold and delete
src/commands/scrape/code/languages/mod.rs  # Unify Language + Metal enums
src/commands/scrape/code/extract_v2.rs     # Update fallback path
src/commands/scrape/code/languages/*.rs    # DELETE 8 processors (keep rust.rs)
Cargo.toml                                 # Remove patina-metal from workspace
patina-metal/                              # DELETE entire crate
```

## Build Order

1. **Cairo grammar plugin** — Prove pure Rust compiles to wasm32-wasip2.
   Validate end-to-end: build → install → scrape → extract matches
   compiled-in output. No wasi-sdk needed — isolates the plugin
   pattern from C-compilation.

2. **Rust grammar plugin** — Prove tree-sitter C compiles to
   wasm32-wasip2 via wasi-sdk. Benchmark dispatch overhead. Document
   wasi-sdk setup. Decision gate: if overhead > 10x, reconsider.

3. **Extract remaining 7 grammars** — Mechanical once both patterns
   proven. Each ports its language processor. One commit per grammar.

4. **Fold Metal into patina-ai, delete patina-metal** — Unify enums,
   move node normalization into plugins, keep Rust fallback, delete
   crate + 52MB of C source.

5. **Publish patina-ai** — `cargo publish -p patina-ai` dry-run
   passes without patina-metal blocking.

6. **Setup grammars command** — `patina setup grammars` installs
   default grammar set from local build artifacts or GitHub releases.
   First-run UX so users don't manually copy WASM files.

7. **Performance benchmark** — Measure plugin dispatch overhead
   using Rust (both paths available). Verify within 5x threshold.
   Document the number.

## Exit Criteria

### Critical
- [x] Cairo grammar works as a pipeline plugin (pure Rust proof)
- [x] At least one tree-sitter grammar works as a pipeline plugin
      (Rust grammar, wasi-sdk proof: build → install → scrape)
- [x] `patina scrape` produces equivalent extraction output with
      grammar plugins vs compiled-in — verified by `grammar-compare.sh all`
      against ref repos. See [TEST-SPEC.md](TEST-SPEC.md) for thresholds
      and method. Tool: `resources/scripts/grammar-compare.sh`
      **VERIFIED 2026-02-14**: All 7 Phase 3 grammars pass with 0% delta
      across all 7 tables. Session [[20260214-205609]].
- [x] patina-metal removed from workspace
      **DONE 2026-02-14**: 585 files deleted (61MB). Session [[20260214-211459]].
- [x] `cargo publish -p patina-ai --dry-run` passes
      **DONE 2026-02-14**: 6.9MB package (2.0MB compressed). Session [[20260214-211459]].

### Important
- [x] All 9 current grammars available as pipeline plugins — each
      passes `grammar-compare.sh <lang>` against its ref repo
      **VERIFIED 2026-02-14**: go, c, cpp, python, javascript, solidity,
      typescript — all 0% delta. Cairo (Phase 1) and Rust (Phase 2)
      verified during their respective sessions.
- [x] Binary size drops significantly (from 69MB)
      **DONE 2026-02-14**: 61MB binary (-12%), 6.9MB package (-89%). Session [[20260214-211459]].
- [ ] `patina setup grammars` installs default grammar set
- [ ] Performance within 5x of compiled-in (acceptable for scrape —
      not real-time editing)
- [x] Version conflict impossible (each plugin bundles own parser)
- [x] Language enums unified (Metal + Language → single enum)
      **DONE 2026-02-14**: Metal deleted with patina-metal. Language enum is single source. Session [[20260214-211459]].

### Nice-to-have
- [ ] At least one new language grammar (Zig, Elixir, Swift) built
      by community pattern
- [ ] Grammar plugin template added to scaffold
      (`patina plugin init grammar-zig --world pipeline`)
- [ ] `patina scrape` reports which grammars are plugin vs fallback

### Pre-push
- [x] `cargo fmt --all`
- [x] `cargo clippy --workspace` (3 benign dead_code warnings)
- [x] `cargo test --workspace`

## Open Risks

1. **cairo-lang dependency tree on wasm32-wasip2.** The Salsa
   incremental computation framework is pure Rust, but the full
   dep tree is large. Possible wasm32 compatibility issues. Mitigation:
   test early (phase 1), fallback to compiled-in if needed.

2. **wasi-sdk installation friction.** Grammar plugin authors need
   wasi-sdk. Not in Homebrew core. Mitigation: document setup clearly,
   consider `zig cc` as alternative C compiler, provide CI build
   scripts.

3. **Plugin binary size.** C++ grammar (17MB parser.c) could produce
   a very large WASM binary. WASM is typically more compact than
   native, but tree-sitter + large grammar + Rust std could still
   be multi-MB. Mitigation: measure in phase 2, acceptable for
   install-once plugins.

4. **Stateless parse overhead.** Each `handle()` call creates a new
   Parser + sets language. Tree-sitter parser creation is cheap but
   not free. Mitigation: benchmark in phase 2, consider caching
   parser in WasmCell if needed (pipeline plugins already use
   singleton pattern via `register_pipeline!`).

5. **File path in ExtractedData.** Language processors populate
   `file` fields in ExtractedData (e.g., `symbol.path`, `function.file`).
   The current request envelope doesn't include the file path — only
   source code and language extension. Either add `path` to the
   envelope or have the host rewrite paths after receiving plugin
   response.

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-14 | draft | Designed from session discussion. Extracts 9 grammars (8 tree-sitter + 1 Cairo) from patina-metal into pipeline plugins. Parser-agnostic design — plugins bring their own parser technology. Blocked by [[patina-sdk]] (plugins need the SDK to build). |
| 2026-02-14 | ready | Code review of patina-metal + scrape pipeline. Key corrections: extraction logic lives in language processors not patina-metal Analyzer, dual Language/Metal enum needs unification, only Rust/Go have .scm queries. Swapped phase order: Cairo first (pure Rust, no wasi-sdk) then Rust (tree-sitter + wasi-sdk). Added parser.c sizes, wasi-sdk setup details, file path envelope gap, Zed as prior art. |
| 2026-02-14 | **phase-3-verified** | `grammar-compare.sh` run against all 7 Phase 3 ref repos. All 7 grammars produce 0% delta across all 7 ExtractedData tables (3,120 files total). Phases 1-3 complete: all 9 grammars available as plugins. Remaining: Phase 4 (fold infrastructure, delete patina-metal). |
| 2026-02-14 | **phase-4-done** | Phase 4 done. patina-metal deleted (61MB, 585 files). Language enums unified (Metal gone, Language survives). Rust fallback uses tree-sitter-rust crate. 8 processors deleted (6,678 lines). `cargo publish --dry-run` passes at 6.9MB. Binary 61MB (was 69MB). All critical exit criteria met. |
| 2026-02-14 | **active** | Reopened: Phase 5 (`patina setup grammars`) and Phase 6 (performance benchmark) designed. Two remaining Important exit criteria need completion before spec can close. |
