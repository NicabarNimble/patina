---
type: feat
id: patina-platform
status: draft
created: 2026-02-05
sessions:
  origin: 20260205-102402
blocked_by: []
blocks:
  - spec-as-work-item
  - cli-reorganization
  - system-introspection
related:
  - layer/surface/build/explore/beads-patterns/SPEC.md
beliefs:
  - unix-philosophy
  - simplicity-is-architecture
references:
  - "Obsidian plugin architecture"
  - "steveyegge/beads"
  - "Extism WASM plugin system"
  - "tree-sitter WASM grammars"
  - "Zed ACP (Agent Client Protocol)"
  - "zed-industries/zed (ref repo)"
---

# feat: Patina as Platform

> Small core, WASM plugins, community extensibility.

## Problem

Patina has grown organically. We keep adding features (forge, persona, mother, eval, introspect) that not everyone needs. The result:

- **Bloated binary** — Ships with everything
- **Slow iteration** — Changing eval means releasing patina
- **No community** — Others can't extend patina
- **Spec confusion** — Where does new functionality go?

We're also planning WASM for tree-sitter grammars. If grammars are WASM, why not plugins too?

---

## Vision

**Core Patina:** Small, stable, does one thing well — context orchestration.

**Plugins:** Everything else. WASM modules that extend core.

```
┌─────────────────────────────────────────────────────────────────┐
│                         patina (core)                            │
│                                                                  │
│  layer/     The knowledge layer (patterns, beliefs, sessions)   │
│  scrape     Core scraping engine (pluggable grammars)           │
│  oxidize    Embeddings (pluggable models)                       │
│  scry       Query engine (pluggable oracles)                    │
│  context    Pattern + belief delivery                           │
│  session    Session lifecycle                                    │
│  plugin     Plugin loader and runtime                           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
        │              │              │              │
        ▼              ▼              ▼              ▼
   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐
   │ grammars│   │  work   │   │ science │   │  forge  │
   │ (wasm)  │   │ (wasm)  │   │ (wasm)  │   │ (wasm)  │
   │         │   │         │   │         │   │         │
   │ rust    │   │ beads-  │   │ eval    │   │ github  │
   │ python  │   │ like    │   │ bench   │   │ issues  │
   │ go      │   │ ready   │   │ compare │   │ PRs     │
   │ ...     │   │ queue   │   │ feedback│   │ ...     │
   └─────────┘   └─────────┘   └─────────┘   └─────────┘
```

---

## Why WASM for Plugins?

| Concern | WASM | Native (.dylib) | External Process |
|---------|------|-----------------|------------------|
| **Sandboxed** | Yes | No | Yes |
| **Cross-platform** | Yes | No (recompile) | Depends |
| **Language agnostic** | Yes (Rust, Go, C, etc.) | Limited | Yes |
| **Performance** | Near-native | Native | IPC overhead |
| **Distribution** | Single .wasm file | Platform-specific | Full binary |
| **Already using** | tree-sitter grammars | - | MCP tools |

**Key insight:** We're already planning WASM for grammars. Same runtime can host plugins.

**Rust WASM runtimes:**
- `wasmtime` — Bytecode Alliance, production-proven
- `wasmer` — Also mature, good Rust support
- `extism` — Plugin-focused, handles host↔guest calls nicely

---

## Plugin Architecture

### Plugin Manifest

Each plugin declares what it provides:

```toml
# plugin.toml
[plugin]
name = "patina-work"
version = "0.1.0"
description = "Beads-like work tracking with ready queue"

[provides]
commands = ["work"]           # Adds `patina work` subcommand
scrapers = []                 # No custom scrapers
oracles = []                  # No custom oracles
tables = ["work_items", "work_deps"]  # SQLite tables

[requires]
patina = ">=0.12.0"           # Core version
plugins = []                  # No plugin dependencies
```

### Extension Points

| Extension | What it adds | Example |
|-----------|--------------|---------|
| **commands** | CLI subcommands | `patina work ready` |
| **scrapers** | New scrape targets | `patina scrape github-issues` |
| **oracles** | New search oracles | GitHub issue search |
| **grammars** | Tree-sitter grammars | New language support |
| **tables** | SQLite tables | Work item storage |
| **adapters** | LLM adapters | New AI tool support |

### Host↔Plugin Interface

```rust
// Core provides to plugins:
trait PatinaHost {
    // Storage
    fn db_execute(&self, sql: &str, params: &[Value]) -> Result<()>;
    fn db_query(&self, sql: &str, params: &[Value]) -> Result<Rows>;

    // Layer access
    fn read_layer_file(&self, path: &str) -> Result<String>;
    fn write_layer_file(&self, path: &str, content: &str) -> Result<()>;

    // Eventlog
    fn emit_event(&self, event_type: &str, data: Value) -> Result<()>;

    // Config
    fn get_config(&self, key: &str) -> Result<Option<String>>;
}

// Plugins implement:
trait PatinaPlugin {
    fn manifest(&self) -> PluginManifest;
    fn on_load(&mut self, host: &dyn PatinaHost) -> Result<()>;
    fn on_command(&mut self, args: &[String]) -> Result<i32>;
}
```

### Plugin Lifecycle

```
1. Discovery:  ~/.patina/plugins/*.wasm
2. Load:       wasmtime instantiates module
3. Init:       plugin.on_load(host) — create tables, etc.
4. Execute:    plugin.on_command(args) — handle CLI
5. Unload:     cleanup on patina exit
```

---

## Core vs Plugin

### Definitely Core

| Component | Why Core |
|-----------|----------|
| `layer/` | Foundation — everything builds on this |
| `scrape` (engine) | Core capability, grammars are plugins |
| `oxidize` | Core capability, models could be plugins |
| `scry` | Core query, oracles could be plugins |
| `context` | Core delivery mechanism |
| `session` | Core workflow |
| `plugin` | Meta — loads everything else |

### Definitely Plugin

| Component | Why Plugin |
|-----------|------------|
| `forge` | Not everyone uses GitHub |
| `work` | Work tracking is optional (beads-like) |
| `science` | Eval/bench is for power users |
| `dev` | Introspect/doctor is for contributors |
| `persona` | Cross-project memory is optional |
| `mother` | Federation is advanced use case |

### Gray Area (Decide Later)

| Component | Considerations |
|-----------|----------------|
| `belief` | Core concept, but management could be plugin |
| `adapter` | Need at least one, but which? |
| `model` | Embedding model management |
| `repo` | External repo management |

---

## Grammars as WASM

Tree-sitter grammars are already designed for WASM:

```
~/.patina/grammars/
├── tree-sitter-rust.wasm
├── tree-sitter-python.wasm
├── tree-sitter-go.wasm
└── ...
```

**Benefits:**
- Add language support without recompiling patina
- Community can contribute grammars
- Same WASM runtime as plugins

**Grammar manifest:**

```toml
[grammar]
name = "rust"
version = "0.21.0"
extensions = [".rs"]
filenames = ["Cargo.toml"]  # Also parse these
```

---

## Plugin Distribution

### Installation

```bash
# From registry (future)
patina plugin install patina-work
patina plugin install patina-forge

# From URL
patina plugin install https://example.com/my-plugin.wasm

# From local file
patina plugin install ./my-plugin.wasm

# List installed
patina plugin list

# Update
patina plugin update patina-work

# Remove
patina plugin remove patina-work
```

### Storage

```
~/.patina/
├── plugins/
│   ├── patina-work.wasm
│   ├── patina-forge.wasm
│   └── patina-science.wasm
├── grammars/
│   ├── tree-sitter-rust.wasm
│   └── ...
└── plugin-config/
    ├── patina-work.toml
    └── patina-forge.toml
```

---

## The Work Plugin (Beads-like)

The `patina-work` plugin would implement beads-like work tracking:

```bash
# Create work items
patina work create "System Introspection" -t epic
patina work create "Define DataContract" -t task --parent <epic>

# Dependencies
patina work dep add <task-b> <task-a>   # B needs A

# Ready queue
patina work ready                        # What can I do now?
patina work blocked                      # What's waiting?

# Lifecycle
patina work status <id> in_progress
patina work close <id> --reason "Done"

# Sync (git-backed like beads)
patina work sync                         # Export to layer/work/*.jsonl
```

**Storage:**
- SQLite: `.patina/local/data/work.db`
- Git-tracked: `layer/work/items.jsonl`

**This replaces:** The `spec-as-work-item` spec. Specs become work items in the work plugin.

---

## Migration Path

### Phase 1: Plugin Infrastructure (v0.12.0)

- [ ] WASM runtime integration (wasmtime or extism)
- [ ] Plugin manifest schema
- [ ] Plugin loader (`patina plugin install/list/remove`)
- [ ] Host interface (db, layer, eventlog access)
- [ ] Grammar loading via WASM (tree-sitter)

### Phase 2: Extract First Plugins (v0.13.0)

- [ ] `patina-forge` — Extract GitHub integration
- [ ] `patina-dev` — Extract introspect, doctor, report
- [ ] Core slimmed down

### Phase 3: Work Plugin (v0.14.0)

- [ ] `patina-work` — Beads-like work tracking
- [ ] Ready queue, dependencies, sync
- [ ] Integration with session workflow

### Phase 4: Community (v0.15.0+)

- [ ] Plugin registry
- [ ] Documentation for plugin authors
- [ ] Community contributions

---

## Impact on Other Specs

This spec **supersedes** several others:

| Spec | Impact |
|------|--------|
| `spec-as-work-item` | Becomes requirements for `patina-work` plugin |
| `cli-reorganization` | Core stays flat, plugins add their commands |
| `system-introspection` | Becomes `patina-dev` plugin |
| `scrape-layer-unify` | Still valid, but scrape becomes pluggable |

---

## Open Questions

1. **Which WASM runtime?** ✅ **DECIDED: wasmtime**
   - Zed uses wasmtime (v19+), production-proven at scale
   - Bytecode Alliance, industry standard
   - Component Model support for WIT interfaces
   - tree-sitter already uses wasmtime for grammars

2. **Plugin sandboxing level?** ✅ **DECIDED: Capability-based with two-layer grants**
   - Pattern from Zed: manifest declares wants, host decides to allow
   - `plugin.toml` declares `[capabilities]` section
   - Host checks against granted capabilities before allowing operations
   - Explicit capability types: `process:exec`, `download-file`, `filesystem`, etc.

3. **Plugin CLI integration?**
   - `patina work ready` (subcommand per plugin)
   - `patina --plugin work ready` (explicit)
   - Both?

4. **Backward compatibility?**
   - Zed uses versioned WIT directories (`since_v0.1.0/`) + enum dispatch
   - Start simple with package versions, add directory versioning when needed
   - Ship core with default plugins bundled initially

5. **Performance budget?**
   - WASM has overhead vs native
   - Acceptable for plugins?
   - Grammars need to be fast for scrape

6. **Future: Patina as agent host?**
   - Current adapters are config generators, not runtime
   - Zed's ACP shows "editor as host" pattern
   - See [[agent-protocol]] explore (parked, speculative)

## Zed Learnings (2026-02-05)

Studied Zed's extension system (77 WIT files, 10 version directories):

### Adopted Patterns

1. **wasmtime + WIT/Component Model** - not Extism
2. **Two-layer capability grants** - manifest + host
3. **Resources for host handles** - `resource worktree`, etc.
4. **Extension API crate** - ergonomic Rust wrapper over WIT bindings
5. **`register_extension!` macro** - handles WASI setup + exports

### Divergence from Zed

1. **Separate worlds per plugin type** (vs Zed's single `extension` world)
   - Stricter capability isolation: oracle plugins can't see HTTP imports
   - Trade-off: less flexible, more secure

2. **Package-level versioning** (vs directory-based `since_v0.x.0/`)
   - Simpler to start, add compat versioning when needed
   - Zed's approach is expensive (enum dispatch + type conversions)

See [[wit-interfaces]] explore for detailed analysis.

### Zed Decoded Insights (Video Transcript)

From "Zed Decoded: Extensions" video with Marshall and Max:

**Sync/Async Transparency**: Extensions see sync APIs, host runs async. When host yields for I/O, the entire WASM runtime suspends. Transparent to extension.

> "We didn't want to have async rust in extensions... it takes the complexity up"

**Historical context**: They initially considered JavaScript/V8. Prior WASM attempt fizzled because "it wasn't mature enough." WASM Component Model had just become usable - right place, right time (Jan 2024).

**WASI Sandboxing**: Extensions use WASI for filesystem. Can only see inside a certain folder (virtual path). Host translates via `path_from_extension()` helper.

**Threading**: Extensions run on dedicated background thread. Every interaction sent over channel. Some APIs callback to main thread.

**Pain Points**: Version dispatch boilerplate ("not in love with it"), Rust-only for now (WASM GC will enable other languages), manual registry curation.

**Future**: Editor interaction APIs, extension UI, self-serve publishing

---

## Non-Goals

- **Plugin marketplace** — Start with manual install, registry later
- **Plugin monetization** — Open source first
- **Hot reloading** — Restart patina to load new plugins
- **Plugin dependencies** — Keep it simple, no dependency hell

---

## References

- [zed-industries/zed](https://github.com/zed-industries/zed) — WIT-based extension system (studied in detail)
  - `crates/extension_api/wit/` — 77 WIT files across 10 versions
  - `crates/extension_host/` — wasmtime-based plugin host
  - `crates/extension_api/src/extension_api.rs` — Extension trait + macro
- [wasmtime](https://wasmtime.dev/) — Bytecode Alliance WASM runtime (Zed's choice)
- [Obsidian Plugin API](https://docs.obsidian.md/Plugins/Getting+started/Build+a+plugin)
- [tree-sitter WASM](https://tree-sitter.github.io/tree-sitter/playground)
- [steveyegge/beads](https://github.com/steveyegge/beads) — Work tracking model

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-05 | draft | Created from session discussion. Key insight: Patina should be a platform with WASM plugins, not a monolith. Work tracking (beads-like) becomes a plugin, not a spec hack. Grammars already planned for WASM, plugins use same runtime. |
| 2026-02-05 | draft | Deep-dive into Zed's extension system. Decided: wasmtime (not Extism), two-layer capability grants, separate worlds per plugin type (diverging from Zed's single world). Added detailed patterns to adopt and diverge. |
