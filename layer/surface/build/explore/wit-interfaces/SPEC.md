---
type: explore
id: wit-interfaces
status: active
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

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-05 | active | Sketched WIT interfaces for all plugin types. Maps directly from existing Rust traits. |
