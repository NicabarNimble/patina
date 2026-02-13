---
type: feat
id: plugin-pipeline-world
status: design
created: 2026-02-13
sessions:
  origin: 20260213-120746
  refined: 20260213-135136
blocked_by:
- plugin-task-world
related:
- layer/surface/build/feat/plugin-ecosystem/SPEC.md
beliefs:
- separate-worlds-for-isolation
- patina-is-knowledge-protocol
---

# feat: Pipeline World (`patina:pipeline`)

> Host-invoked pure-compute plugins. Grammar parsers, chunkers, tokenizers.
> The plugin is a pure function — all side effects stay in the host.

## Problem

Grammar parsing (tree-sitter), chunking, and custom scrapers are compiled
into the binary via `patina-metal` (9 languages). Adding a new language
or scraper means recompiling patina. A pipeline world lets community
plugins extend the knowledge pipeline without touching core code.

## Parent Design

Build order item #4 from [[plugin-ecosystem]] SPEC.md. Pipeline world WIT
and `handle(json)` dispatch pattern are defined there (lines 559-588).
Subsumes the archived `plugin-oracle-scraper` and `plugin-grammars` specs.

## Spec Divergences from Parent

1. **No divergences yet.** Pipeline is the simplest world (log-only import).
   Design is clean and locked in parent spec.
2. **Oracle stays host-side.** Confirmed in ecosystem spec (lines 583-589).
   Not part of this world.

## Scope

### WIT (from ecosystem spec, locked)

```wit
world pipeline {
    import patina:host/log@0.1.0;

    export init: func();
    export name: func() -> string;
    export handle: func(request: string) -> result<string, string>;
}
```

`log` is the **only** import. No query, no layer, no HTTP, no toys.
Compile-time isolation per [[separate-worlds-for-isolation]]. The guest
crate won't even have bindings for capabilities the world doesn't import.

### Versioned Envelope

All dispatch goes through `handle(json)` with a versioned envelope:

```json
{
    "op": "parse",
    "version": "1",
    "payload": {
        "source": "<base64-encoded bytes>",
        "language": "zig"
    },
    "trace_id": "abc123"
}
```

Host checks `[provides]` in manifest to know which ops to dispatch.
Guest API crate offers typed helpers that build the envelope automatically.

### Manifest

```toml
[plugin]
name = "zig-grammar"
world = "pipeline"

[capabilities]
host_log = true

[provides]
pipeline_ops = ["parse"]
languages = ["zig"]
```

`[provides].pipeline_ops` declares which ops this plugin handles.
`[provides].languages` declares which file extensions trigger dispatch.
Host matches files to plugins by language, then dispatches the right op.

### What NOT to Touch

- `src/plugin/internal/command.rs` — different world
- `src/plugin/internal/mother_child.rs` — different world
- `src/plugin/internal/task.rs` — different world (once it exists)
- `wit/command/`, `wit/mother-child/`, `wit/task/` — other worlds
- `patina-metal/` — existing grammars stay compiled-in as fallback
- `src/mcp/` — MCP server unrelated
- Oracle internals in `src/retrieval/` — oracle stays host-side

## Architecture

### Pattern: Simplest Engine

Pipeline is the simplest world. No capabilities to gate (only log).
No query, no HTTP, no toys. `PipelineHostState` is minimal:

```rust
pub struct PipelineHostState {
    pub plugin_name: String,
    pub wasi: wasmtime_wasi::WasiCtx,
    pub wasi_table: wasmtime::component::ResourceTable,
}
```

Same shape as current mother-child `HostState` (before HTTP expansion).
Only trait impl needed: `log::Host`.

### PipelineEngine

```rust
pub struct PipelineEngine {
    linker: Linker<PipelineHostState>,
}

impl PipelineEngine {
    pub fn new() -> Result<Self> { ... }

    /// Invoke a pipeline plugin with a request envelope.
    /// Returns the JSON response or error.
    pub fn handle(
        &self,
        component: &Component,
        manifest: &PluginManifest,
        request: &str,
    ) -> Result<String> { ... }
}
```

### Scrape Integration Point

**File:** `src/commands/scrape/code/extract_v2.rs` (lines 146-198)

Current flow: hardcoded `match language { Rust => ..., Go => ... }`.
With pipeline plugins:

```
1. For each file:
   a. Detect language (file extension)
   b. Check if a pipeline plugin claims this language
   c. If yes: build envelope, call plugin.handle(json)
   d. If no: fall back to built-in patina-metal processor
   e. Parse ExtractedData from response
2. Bulk insert to database (unchanged)
```

The fallback ensures scrape works without any plugins installed.
Per [[graceful-extraction]]: plugin-first dispatch with compiled fallback.

### Plugin Discovery

Pipeline plugins live in `~/.patina/pipeline/` (per ecosystem spec).
At scrape time, host scans for manifests, builds a language→plugin map:

```rust
// Pseudocode — not final API
let pipeline_plugins: HashMap<String, (Component, PluginManifest)> =
    discover_pipeline_plugins()?;  // scan ~/.patina/pipeline/

for file in project_files {
    let lang = detect_language(&file);
    if let Some((component, manifest)) = pipeline_plugins.get(&lang) {
        // Plugin path
        let request = build_parse_envelope(&file, &content);
        let response = engine.handle(component, manifest, &request)?;
        parse_extracted_data(&response, &mut all_data)?;
    } else {
        // Built-in path (patina-metal)
        process_with_builtin(lang, &file, &content, &mut all_data)?;
    }
}
```

### Data Format

Pipeline plugins return `ExtractedData` as JSON. The schema matches
the existing bulk insert format in `extract_v2.rs`:

```json
{
    "symbols": [{"name": "main", "kind": "function", "line": 1, ...}],
    "functions": [{"name": "main", "params": "...", ...}],
    "types": [...],
    "imports": [...],
    "call_edges": [...],
    "constants": [...],
    "members": [...]
}
```

This is defined by `ExtractedData` in `src/commands/scrape/code/`.
The plugin returns this JSON; the host deserializes and bulk-inserts.
No schema changes needed on the database side.

## Exact Files to Create/Change

### New files

| File | What | Pattern to follow |
|------|------|-------------------|
| `wit/pipeline/pipeline.wit` | Pipeline world WIT | `wit/mother-child/mother-child.wit` (simplest world) |
| `wit/pipeline/deps/patina-host/host.wit` | Host interfaces (log only) | Just `interface log` from canonical host.wit |
| `src/plugin/internal/pipeline.rs` | `PipelineEngine` + `PipelineHostState` | `src/plugin/internal/mother_child.rs` (simplest engine) |
| `patina-pipeline-api/Cargo.toml` | Guest crate manifest | `patina-command-api/Cargo.toml` |
| `patina-pipeline-api/src/lib.rs` | `PipelinePlugin` trait + `register_pipeline!` + typed `PipelineOp` enum | `patina-plugin-api/src/lib.rs` |
| `patina-pipeline-api/wit/pipeline/` | WIT copy for guest bindgen | sync from `wit/pipeline/` |

### Modified files

| File | What changes |
|------|-------------|
| `src/plugin/internal/mod.rs` | Add `mod pipeline; pub use pipeline::PipelineEngine;` |
| `src/plugin/mod.rs` | Add `PipelineEngine` to pub use re-export |
| `src/commands/scrape/code/extract_v2.rs` | Add plugin dispatch before built-in processing |
| `Cargo.toml` | Add `patina-pipeline-api` to workspace members |
| `src/plugin/internal/tests.rs` | Conformance tests |

### Not changing

`command.rs`, `mother_child.rs`, `task.rs`, other WIT directories,
`patina-metal/` (stays as compiled-in fallback), `src/mcp/`,
database schema in `scrape/code/database.rs`

## Implementation Plan (4 commits)

**Commit 1: WIT + PipelineEngine skeleton**
- Create `wit/pipeline/pipeline.wit` (log-only import, 3 exports)
- Create `wit/pipeline/deps/patina-host/host.wit` (log interface only)
- Create `src/plugin/internal/pipeline.rs`:
  - `PipelineHostState` (minimal — name + wasi only)
  - `PipelineEngine::new()` (Linker setup)
  - `PipelineEngine::handle()` → invoke plugin, return JSON string
  - `log::Host` impl (copy from any other world)
- Wire into `mod.rs` and `plugin/mod.rs`

**Commit 2: Guest API crate**
- Create `patina-pipeline-api/` crate
- `PipelinePlugin` trait: `name()`, `handle(request: &str) -> Result<String, String>`
- `register_pipeline!` macro
- Typed `PipelineOp` enum: `Parse`, `Chunk`, `Tokenize`
- Typed helpers: `pipeline::parse(source, language)` builds envelope
- Add to workspace in root `Cargo.toml`

**Commit 3: Scrape integration**
- Add pipeline plugin discovery in scrape code path
- Build language→plugin map from `~/.patina/pipeline/` manifests
- Dispatch to plugin before built-in processor (fallback pattern)
- Parse `ExtractedData` from plugin JSON response
- Log which files used plugin vs built-in path

**Commit 4: Conformance test**
- Create `echo-pipeline` test fixture:
  - Handles `{"op":"echo"}` — returns payload unchanged
  - Manifest: `world = "pipeline"`, `host_log = true`,
    `[provides] pipeline_ops = ["echo"]`
- Tests in `tests.rs`:
  - `handle()` round-trip with echo envelope
  - Unknown op returns error
  - Envelope version mismatch returns error

## Dependencies

- Task world (build order #3) proves the third engine pattern
- HTTP interface (build order #2) not needed — pipeline has no HTTP
- `ExtractedData` struct must be serializable to JSON (verify `Serialize`
  derive exists — if not, add it as part of commit 3)

## Exit Criteria

- [ ] `wit/pipeline/pipeline.wit` with log-only import
- [ ] `PipelineEngine` in `src/plugin/internal/pipeline.rs`
- [ ] `PipelineHostState` minimal (no grants, no HTTP, no query)
- [ ] Host-side integration: scrape pipeline dispatches to pipeline plugins
- [ ] Fallback to patina-metal built-in when no plugin claims a language
- [ ] Versioned envelope validated at boundary (op + version + payload)
- [ ] Guest API crate with `PipelineOp` typed enum and helpers
- [ ] `register_pipeline!` macro generates correct exports
- [ ] Conformance test: `echo-pipeline` proves envelope dispatch
- [ ] `cargo test --workspace` passes
- [ ] `./resources/git/pre-push-checks.sh` passes

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-13 | design | Extracted from [[plugin-ecosystem]] build order item #4. Blocked by task world. |
| 2026-02-13 | design | Refined in session [[20260213-135136]]. Added scrape integration point (`extract_v2.rs`), plugin discovery pattern, fallback to patina-metal, ExtractedData JSON format, exact files list, commit plan. |
