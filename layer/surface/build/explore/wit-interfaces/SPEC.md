---
type: explore
id: wit-interfaces
status: abandoned
created: 2026-02-05
sessions:
  origin: 20260205-102402
related:
- layer/surface/build/feat/patina-platform/SPEC.md
- layer/surface/build/explore/llm-adapter-refactor/SPEC.md
---

# explore: WIT Interface Definitions

> Our Rust traits as WebAssembly Interface Types.

## Package Structure

```
wit/
├── patina-types.wit      # Shared types
├── oracle.wit            # Oracle interface
├── embedding.wit         # Embedding engine interface
├── forge.wit             # Forge reader interface
├── adapter.wit           # LLM adapter interface
├── scraper.wit           # Custom scraper interface
└── work.wit              # Work tracking interface (beads-like)
```

---

## patina-types.wit

Shared types used across interfaces.

```wit
package patina:types@0.1.0;

/// Common types shared across Patina plugins
interface types {
    /// Result with error message
    variant result-error {
        ok,
        err(string),
    }

    /// Key-value metadata
    record metadata {
        key: string,
        value: string,
    }

    /// File path reference
    type path = string;

    /// Timestamp as ISO 8601 string
    type timestamp = string;

    /// JSON blob for extensible data
    type json = string;
}
```

---

## oracle.wit

Maps to `src/retrieval/oracle.rs`.

```wit
package patina:oracle@0.1.0;

use patina:types@0.1.0.{types};

/// Oracle interface for search/retrieval plugins
interface oracle {
    /// Metadata attached to oracle results
    record oracle-metadata {
        file-path: option<string>,
        timestamp: option<string>,
        event-type: option<string>,
        matches: option<list<string>>,
    }

    /// Single result from an oracle query
    record oracle-result {
        /// Unique document identifier (for deduplication)
        doc-id: string,
        /// Content snippet or summary
        content: string,
        /// Source oracle name
        source: string,
        /// Raw score (scale varies by oracle)
        score: f32,
        /// Score type for interpretation (cosine, bm25, etc.)
        score-type: string,
        /// Additional metadata
        metadata: oracle-metadata,
    }

    /// Oracle identity
    name: func() -> string;

    /// Query the oracle, returning ranked results
    query: func(q: string, limit: u32) -> result<list<oracle-result>, string>;

    /// Whether this oracle is available (index exists, etc.)
    is-available: func() -> bool;
}

/// World for oracle plugins
world oracle-plugin {
    /// Plugin exports the oracle interface
    export oracle;
}
```

---

## embedding.wit

Maps to `src/embeddings/mod.rs`.

```wit
package patina:embedding@0.1.0;

/// Embedding engine interface
interface embedding {
    /// Embedding vector (list of f32)
    type embedding-vector = list<f32>;

    /// Generate embedding for text
    embed: func(text: string) -> result<embedding-vector, string>;

    /// Generate embedding for query text (with model-specific prefix)
    embed-query: func(text: string) -> result<embedding-vector, string>;

    /// Generate embedding for passage text (with model-specific prefix)
    embed-passage: func(text: string) -> result<embedding-vector, string>;

    /// Batch embed multiple texts
    embed-batch: func(texts: list<string>) -> result<list<embedding-vector>, string>;

    /// Get embedding dimension (e.g., 384)
    dimension: func() -> u32;

    /// Get model name
    model-name: func() -> string;
}

/// World for embedding plugins
world embedding-plugin {
    export embedding;
}
```

---

## forge.wit

Maps to `src/forge/mod.rs`.

```wit
package patina:forge@0.1.0;

/// Forge reader interface for issue/PR access
interface reader {
    /// Issue state
    enum issue-state {
        open,
        closed,
    }

    /// Issue label
    record label {
        name: string,
        color: option<string>,
    }

    /// Issue representation
    record issue {
        number: s64,
        title: string,
        body: option<string>,
        state: issue-state,
        author: option<string>,
        labels: list<label>,
        created-at: string,
        updated-at: string,
        closed-at: option<string>,
    }

    /// Pull request representation
    record pull-request {
        number: s64,
        title: string,
        body: option<string>,
        state: issue-state,
        author: option<string>,
        labels: list<label>,
        head-ref: string,
        base-ref: string,
        created-at: string,
        updated-at: string,
        merged-at: option<string>,
    }

    /// Get total issue count
    get-issue-count: func() -> result<u32, string>;

    /// Get total PR count
    get-pr-count: func() -> result<u32, string>;

    /// List issues (with optional since filter)
    list-issues: func(limit: u32, since: option<string>) -> result<list<issue>, string>;

    /// List pull requests
    list-pull-requests: func(limit: u32, since: option<string>) -> result<list<pull-request>, string>;

    /// Get single issue by number
    get-issue: func(number: s64) -> result<issue, string>;

    /// Get single PR with full details
    get-pull-request: func(number: s64) -> result<pull-request, string>;

    /// Get highest issue number
    get-max-issue-number: func() -> result<s64, string>;
}

/// World for forge plugins (needs network access)
world forge-plugin {
    /// Import WASI HTTP for API calls
    import wasi:http/outgoing-handler@0.2.0;

    /// Plugin exports the reader interface
    export reader;
}
```

---

## adapter.wit

Maps to refactored `LLMAdapter` (see llm-adapter-refactor exploration).

```wit
package patina:adapter@0.1.0;

/// LLM adapter interface (simplified, manifest-based)
interface adapter {
    /// Template file to create on init
    record template {
        /// Relative path from project root
        path: string,
        /// Template content
        content: string,
        /// Overwrite if exists?
        overwrite: bool,
    }

    /// Custom command provided by adapter
    record command {
        name: string,
        description: string,
    }

    /// Static manifest describing the adapter
    record manifest {
        /// Adapter name (e.g., "claude", "gemini")
        name: string,
        /// Adapter version
        version: string,
        /// Main context file (e.g., "CLAUDE.md")
        context-file: string,
        /// Config directory (e.g., ".claude/")
        config-dir: option<string>,
        /// Sessions directory
        sessions-dir: option<string>,
        /// Templates to create on init
        templates: list<template>,
        /// Custom commands
        commands: list<command>,
    }

    /// Get adapter manifest
    get-manifest: func() -> manifest;

    /// Launch the adapter (returns command + args to spawn)
    get-launch-command: func(project-path: string) -> result<list<string>, string>;
}

/// World for adapter plugins
world adapter-plugin {
    export adapter;
}
```

---

## scraper.wit

For custom scrapers (new content types).

```wit
package patina:scraper@0.1.0;

/// Custom scraper interface
interface scraper {
    /// Scraped item
    record scraped-item {
        /// Unique ID for this item
        id: string,
        /// Event type for eventlog (e.g., "custom.mytype")
        event-type: string,
        /// Content to index
        content: string,
        /// Source file path (if applicable)
        source-path: option<string>,
        /// Additional metadata as JSON
        metadata: string,
    }

    /// Scraper identity
    name: func() -> string;

    /// File patterns this scraper handles (globs)
    patterns: func() -> list<string>;

    /// Scrape a single file, return items
    scrape-file: func(path: string, content: string) -> result<list<scraped-item>, string>;

    /// Scrape a directory (for non-file sources)
    scrape-directory: func(path: string) -> result<list<scraped-item>, string>;
}

/// World for scraper plugins
world scraper-plugin {
    /// Import filesystem access (read-only)
    import wasi:filesystem/types@0.2.0;
    import wasi:filesystem/preopens@0.2.0;

    export scraper;
}
```

---

## work.wit

Beads-like work tracking (the `patina-work` plugin).

```wit
package patina:work@0.1.0;

/// Work tracking interface (beads-like)
interface tracker {
    /// Work item status
    enum status {
        open,
        in-progress,
        blocked,
        closed,
    }

    /// Work item type
    enum item-type {
        task,
        bug,
        feature,
        epic,
    }

    /// Dependency type
    enum dep-type {
        blocks,
        parent-child,
        related,
    }

    /// Work item
    record work-item {
        id: string,
        title: string,
        description: option<string>,
        status: status,
        item-type: item-type,
        priority: u8,
        assignee: option<string>,
        created-at: string,
        updated-at: string,
        closed-at: option<string>,
    }

    /// Dependency between items
    record dependency {
        from-id: string,
        to-id: string,
        dep-type: dep-type,
    }

    /// Create a new work item
    create: func(title: string, item-type: item-type, priority: u8) -> result<work-item, string>;

    /// Get work item by ID
    get: func(id: string) -> result<option<work-item>, string>;

    /// Update work item
    update: func(id: string, title: option<string>, description: option<string>, status: option<status>, priority: option<u8>) -> result<work-item, string>;

    /// Close work item
    close: func(id: string, reason: option<string>) -> result<work-item, string>;

    /// List items with optional status filter
    list: func(status: option<status>, limit: u32) -> result<list<work-item>, string>;

    /// Get ready items (no open blockers)
    ready: func(limit: u32) -> result<list<work-item>, string>;

    /// Get blocked items
    blocked: func(limit: u32) -> result<list<work-item>, string>;

    /// Add dependency
    add-dep: func(from-id: string, to-id: string, dep-type: dep-type) -> result<dependency, string>;

    /// Remove dependency
    remove-dep: func(from-id: string, to-id: string) -> result<bool, string>;

    /// Get dependencies for item
    get-deps: func(id: string) -> result<list<dependency>, string>;

    /// Sync to git (export JSONL, commit)
    sync: func() -> result<bool, string>;
}

/// World for work tracking plugin
world work-plugin {
    /// Import filesystem for JSONL storage
    import wasi:filesystem/types@0.2.0;
    import wasi:filesystem/preopens@0.2.0;

    /// Import host functions for eventlog, layer access
    import patina:host/eventlog;
    import patina:host/layer;

    export tracker;
}
```

---

## host.wit

Host functions that Patina exposes to plugins.

```wit
package patina:host@0.1.0;

/// Eventlog access for plugins
interface eventlog {
    /// Emit an event to the eventlog
    emit: func(event-type: string, data: string) -> result<s64, string>;

    /// Query events by type prefix
    query: func(type-prefix: string, limit: u32) -> result<list<string>, string>;
}

/// Layer access for plugins
interface layer {
    /// Read a layer file
    read: func(path: string) -> result<option<string>, string>;

    /// Write a layer file (git-tracked)
    write: func(path: string, content: string) -> result<bool, string>;

    /// List files matching glob in layer
    glob: func(pattern: string) -> result<list<string>, string>;
}

/// Database access for plugins (plugin-scoped)
interface database {
    /// Execute SQL (CREATE, INSERT, UPDATE, DELETE)
    execute: func(sql: string, params: list<string>) -> result<u64, string>;

    /// Query SQL (SELECT), returns JSON rows
    query: func(sql: string, params: list<string>) -> result<string, string>;
}
```

---

## Summary

| Interface | Methods | WASI Imports | Host Imports |
|-----------|---------|--------------|--------------|
| oracle | 3 | none | none |
| embedding | 6 | none | none |
| forge/reader | 7 | wasi:http | none |
| adapter | 2 | none | none |
| scraper | 4 | wasi:filesystem | none |
| work/tracker | 13 | wasi:filesystem | eventlog, layer |

**Pure plugins** (oracle, embedding, adapter): No capabilities needed, sandboxed.

**Capability plugins** (forge, scraper, work): Need explicit WASI or host imports.

---

## Next Steps

1. Validate WIT syntax with `wit-bindgen`
2. Generate host bindings for one interface (oracle)
3. Create example plugin
4. Test round-trip

---

## Zed Analysis (2026-02-05)

Studied zed-industries/zed extension system (216 crates, 77 WIT files).

### Zed Architecture

1. **Versioned WIT directories**: `wit/since_v0.0.1/`, `wit/since_v0.1.0/`, etc.
   - Host has `Extension` enum with variant per version
   - Method dispatch matches on version, uses `.into()` for type conversion
   - Expensive but supports backwards compatibility

2. **Single world with capability imports**:
   ```wit
   world extension {
       import http-client;
       import github;
       import platform;
       import process;
       import nodejs;

       export init-extension: func();
       export language-server-command: func(...);
   }
   ```

3. **Resources for host handles**:
   ```wit
   resource worktree {
       id: func() -> u64;
       root-path: func() -> string;
       read-text-file: func(path: string) -> result<string, string>;
       which: func(binary-name: string) -> option<string>;
       shell-env: func() -> env-vars;
   }
   ```

4. **Two-layer capability grants**:
   - `extension.toml` manifest declares: `capabilities = [{ process:exec = { command = "ls", args = ["-la"] }}]`
   - `CapabilityGranter` checks manifest AND host grants before allowing

5. **Extension manifest** (`extension.toml`):
   ```toml
   id = "glsl"
   name = "GLSL"
   version = "0.2.0"
   schema_version = 1

   [language_servers.glsl_analyzer]
   name = "GLSL Analyzer LSP"
   language = "GLSL"

   [grammars.glsl]
   repository = "https://github.com/theHamsta/tree-sitter-glsl"
   commit = "31064ce..."
   ```

6. **Extension API crate** (`zed_extension_api`):
   - Provides `Extension` trait with default impls
   - `register_extension!` macro handles WASI setup + exports
   - Re-exports WIT-generated types for ergonomic Rust API

### What Patina Should Adopt

| Pattern | Adopt? | Rationale |
|---------|--------|-----------|
| Versioned WIT directories | Later | Start with package version, add compat when needed |
| Single world | No | Keep separate worlds for stricter capability isolation |
| Resources for handles | Yes | Add `result-cursor` for streaming, `worktree` equivalent |
| Two-layer grants | Yes | Critical for security |
| `plugin.toml` manifest | Yes | Define our format |
| API crate | Yes | `patina_plugin_api` wrapping WIT bindings |

### Key Implementation Detail: Sync/Async Transparency

From Zed Decoded video - extensions see **synchronous** APIs but host runs **async**:

```rust
// Host side bindgen with async: true
wasmtime::component::bindgen!({
    async: true,  // host methods are async fn
    path: "./wit/since_v0.8.0",
});

// Extension sees sync call:
fn download_file(url: &str, path: &str) -> Result<()>;

// Host implements async:
async fn download_file(&mut self, url: String, path: String) -> Result<()> {
    // When this awaits, WASM runtime suspends
    let response = self.http_client.get(&url).await?;
    // ...
}
```

When host yields (e.g., for I/O), the **entire WASM runtime suspends**. This is transparent to the extension - no async rust complexity.

> "We didn't want to have async rust in extensions... it takes the complexity up"

**For Patina**: Apply this pattern. Plugins see sync APIs for `query()`, `embed()`, etc. Host handles async I/O internally.

### Key Implementation Detail: WASI Sandboxing

Extensions use WASI for filesystem access but see a **virtual path**:

```rust
// Extension thinks it's writing to:
"/work/model-cache/model.onnx"

// Host translates to real path:
"~/.patina/plugins/my-oracle/work/model-cache/model.onnx"

fn path_from_extension(extension: &Extension, virtual_path: &str) -> PathBuf {
    extension.work_dir.join(virtual_path.strip_prefix("/work/").unwrap_or(virtual_path))
}
```

**Benefits**:
- Extensions can't escape their sandbox
- Standard `fs` APIs work (no special wrappers needed)
- Host controls real storage location

**For Patina**: Apply this pattern. Each plugin gets isolated work directory.

### Parallelism Options for WASM Host (Respecting sync-first)

Patina has a [[sync-first]] belief: no async in codebase. When adding WASM plugins, we need parallelism without async infection.

**The Problem** (from No Boilerplate "Async Rust Is A Bad Language" video):
- `tokio::spawn` requires `'static` lifetimes → infects entire codebase
- Lose ability to reason about one function at a time
- Concurrency ≠ Parallelism (tokio conflates them)

**The Solution**: "If you scope the async part tighter than the whole program, your life will be better."

| Approach | Async Spread Risk | When to Use | When to Avoid |
|----------|-------------------|-------------|---------------|
| `std::thread::scope` | **Zero** | Plugin calls, bounded parallel work. Preserves borrowing. | Thousands of concurrent I/O tasks |
| `rayon` | **Zero** | CPU-bound parallel (oracles, embeddings, batch processing) | I/O-bound work |
| `smol` | **Low** | If async truly needed, tiny runtime (1k LOC) | Unless actually needed |
| `tokio` contained | **Medium** | WASM host I/O if scoped threads insufficient | Never let it escape module |
| `tokio` everywhere | **☠️** | **Never** | **Always avoid** |

**Recommended: Scoped Threads for Plugin Calls**

```rust
// Good: No async, borrowing works, compiler helps
fn call_plugin_oracle(plugin: &WasmPlugin, query: &str) -> Result<Vec<OracleResult>> {
    std::thread::scope(|s| {
        s.spawn(|| {
            plugin.call_query(query)  // Blocks inside, fine for CLI
        }).join().unwrap()
    })
}
```

The `'static` infection only happens with unscoped `spawn`. With `scope`, we keep borrowing.

**How Zed Contains Async** (if we ever need it):

```
┌─────────────────────────────────────────────────────────────┐
│                 Zed App (sync code)                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   gpui_tokio bridge                          │
│            tokio runtime (2 threads ONLY)                    │
│              CONTAINED - never escapes                       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                 wasmtime with epoch                          │
│         config.epoch_interruption(true);                     │
│         store.epoch_deadline_async_yield_and_update(1);      │
│                                                              │
│         Extensions see SYNC APIs                             │
│         Host yields → WASM suspends → host does I/O          │
└─────────────────────────────────────────────────────────────┘
```

Key constraints if adding tokio:
1. Create small runtime (2 threads max)
2. Contain to ONE module (e.g., `src/wasm_host/runtime.rs`)
3. Never export async types
4. Plugins always see sync APIs
5. Use epoch interruption for cooperative yielding

**Decision for Patina**: Start with `std::thread::scope`. Only add contained tokio if we need many concurrent I/O-heavy plugins and threads don't scale.

### Gaps to Address

1. **Define `plugin.toml`** format for Patina plugins
2. **Add capability grant system** (manifest + host)
3. **Add streaming resource** for large result sets
4. **Create `patina_plugin_api`** crate

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-05 | active | Sketched WIT interfaces for all plugin types. Maps directly from existing Rust traits. |
| 2026-02-05 | active | Analyzed Zed's extension system. Added comparison notes, identified gaps. |
| 2026-02-05 | active | Added Zed Decoded video insights: sync/async transparency, WASI sandboxing, threading model, historical context. |
| 2026-02-05 | active | Added parallelism options respecting sync-first: scoped threads recommended, tokio contained if needed. No Boilerplate video context. |
