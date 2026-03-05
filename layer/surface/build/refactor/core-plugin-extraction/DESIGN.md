# Design: Extract Spec Subsystem to Extension Plugin

## Why This Is the Hardest Extraction

Forge extraction proves the pattern: take domain code, wrap it in WIT
interfaces, ship as a plugin. Forge only needs `host/http` and
`host/emit` — both are read/write operations against known interfaces.

The spec subsystem needs capabilities that don't exist in WIT yet:
filesystem writes, git operations, and MCP tool registration. Each
requires its own host interface with its own security model. This isn't
just extraction — it's infrastructure design.

See [[spec-core-extraction]] DESIGN.md, "Core Plugin Extraction —
The Hardest Child."

## What the Spec Subsystem Does

The spec subsystem is a full lifecycle management system:

**Types** (`src/spec.rs`, 640 LOC):
- `SpecFrontmatter` — YAML parse/serialize for spec files
- `ExitCriterion` — structured exit criteria with id, text, checked
- `Sessions` — flexible session reference format
- `SpecStatus`, `SpecType` — enums for lifecycle state

**CLI commands** (`src/commands/spec/mod.rs`, 528 LOC):
- 17 subcommands via clap: create, promote, pause, resume, complete,
  abandon, split, block, set, show, list, ready, blocked, next,
  check, history, archive

**Internal implementation** (`src/commands/spec/internal/`, 3,091 LOC):
- `queries.rs` (1,061 LOC) — filesystem scan, frontmatter parsing,
  dependency resolution, list/show/check/history
- `mutations.rs` (880 LOC) — promote, pause, resume, complete,
  abandon, block, set — each creates git tags and commits
- `archive.rs` (410 LOC) — git tag + remove from working tree
- `create.rs` (276 LOC) — scaffold directory, write SPEC.md + DESIGN.md
- `queue.rs` (262 LOC) — next-spec ranking, age computation,
  dependency count loading
- `split.rs` (152 LOC) — complete original + create continuation

**MCP tools** (`src/mcp/server/spec.rs`, 361 LOC):
- 16 MCP tool handlers that call into the internal functions
- This is how AI tools interact with spec lifecycle

**Database** (patina.db):
- `patterns` table — spec metadata cache
- `spec_deps` table — dependency tracking

**Total: ~4,620 LOC across 4 locations.**

## The Capability Gap

Current WIT host interfaces vs what specs need:

| Spec needs | Current WIT | Gap |
|-----------|-------------|-----|
| Read layer/ files | `host/layer` (read-only) | None — reading works |
| Read project config | `host/layer` | None |
| Write SPEC.md files | — | **host/fs-write** needed |
| Create directories | — | **host/fs-write** needed |
| Git tags (spec lifecycle) | — | **host/git** needed |
| Git commits (spec create) | — | **host/git** needed |
| Git staging (spec archive) | — | **host/git** needed |
| Register MCP tools | — | **host/mcp-register** needed |
| Query patina.db | `host/query` (scry/assay/context) | Partial — query is read-only |
| Write to patina.db | — | **host/db-write** or use host/emit |

Three new host interfaces are needed at minimum:

### host/fs-write
Scoped filesystem write access. The plugin can only write within the
project directory (safety boundary from [[safety-boundaries]]). The
host validates paths before execution.

```wit
interface fs-write {
    /// Write content to a file (create or overwrite).
    /// Path must be within project root.
    write-file: func(path: string, content: string) -> result<_, string>;

    /// Create a directory (and parents).
    create-dir: func(path: string) -> result<_, string>;

    /// Remove a file or directory.
    remove: func(path: string) -> result<_, string>;
}
```

### host/git
Git operations for lifecycle management. The host runs git commands;
the plugin never touches the `.git` directory directly.

```wit
interface git {
    /// Create a lightweight tag.
    create-tag: func(name: string) -> result<_, string>;

    /// Stage files for commit.
    add: func(paths: list<string>) -> result<_, string>;

    /// Create a commit with message.
    commit: func(message: string) -> result<_, string>;

    /// Remove files from working tree (for archive).
    rm: func(paths: list<string>) -> result<_, string>;
}
```

### host/mcp-register
Dynamic MCP tool registration. This is the hardest interface because
it changes how Patina exposes tools to LLMs.

```wit
interface mcp-register {
    /// Register a tool that the plugin handles.
    /// The host adds it to the MCP tool list.
    /// When called, the host routes to the plugin's handle() export.
    register-tool: func(
        name: string,
        description: string,
        input-schema: string,  // JSON Schema
    ) -> result<_, string>;
}
```

**The MCP registration problem:** Today, MCP tools are hardcoded in
the binary. The tool list is static. Dynamic registration means:
- Tools appear/disappear based on installed plugins
- The MCP server must route tool calls to the correct plugin
- Tool schemas come from plugins, not compiled Rust code
- Plugin load order affects available tools

This is architecturally the biggest shift in the entire extraction
roadmap. It changes the MCP server from a static dispatcher to a
plugin-routed system.

## Sessions Stay in Core

Sessions and adapters are NOT extracted. See [[spec-core-extraction]]
DESIGN.md, "Sessions and Adapters — Staying in Core (For Now)."

Sessions are the primary interaction path — how users work with Patina
through AI tools. Adapters (CLAUDE.md, Cursor rules) are how Patina
connects to those tools. Extracting them before an alternative exists
would leave Patina headless.

**Revisit when:** A native Patina UI exists, or the CLI can stand alone
without an AI tool driving it.

## The "Is Spec Domain-Specific?" Question

This is the deepest open question. [[spec-driven-design]] is a core
value — specs are how Patina governs its own development. But
[[patina-is-domain-agnostic-knowledge-system]] says domain-specific
code doesn't belong in core.

**Arguments for extraction:**
- A law firm doesn't need spec lifecycle management
- Spec is development workflow tooling
- [[code-is-not-core]] logic applies: not every project needs this

**Arguments for keeping in core:**
- Specs might be to Patina what branches are to git — not domain-specific
  but a core workflow concept
- Specs are how the "evolve" verb manifests (specs are actionable beliefs
  per [[specs-are-actionable-beliefs]])
- Spec FILE parsing stays in core regardless (layer/ scraper reads them)

**The extraction doesn't need to answer this question.** Extract the
command system. Leave spec file parsing in the layer scraper. If a
project doesn't install the spec plugin, it can still have spec files
in `layer/surface/build/` — they just can't be managed with lifecycle
commands. Like a git repo without a GUI client.

## Extraction Strategy

Given the capability gaps, this spec may need to be split:

**Phase A: Design host capabilities**
- Design `host/fs-write`, `host/git`, `host/mcp-register`
- Each needs its own security model
- Could be a separate spec focused on WIT interface design

**Phase B: Extract spec subsystem**
- Write the spec plugin using the new host interfaces
- Port 4,620 LOC from compiled Rust to WASM plugin
- The plugin registers 16 MCP tools dynamically
- Remove spec commands from core binary

The phases might be sequential specs rather than one spec with phases,
depending on how much design work Phase A requires.

## Key Files

**Spec subsystem (to be extracted):**
- `src/spec.rs` (640 LOC) — types, frontmatter parse/serialize
- `src/commands/spec/mod.rs` (528 LOC) — CLI dispatch, clap subcommands
- `src/commands/spec/internal/queries.rs` (1,061 LOC) — filesystem scan, dependency resolution
- `src/commands/spec/internal/mutations.rs` (880 LOC) — lifecycle transitions + git ops
- `src/commands/spec/internal/archive.rs` (410 LOC) — git tag + remove
- `src/commands/spec/internal/create.rs` (276 LOC) — scaffold + write
- `src/commands/spec/internal/queue.rs` (262 LOC) — next-spec ranking
- `src/commands/spec/internal/split.rs` (152 LOC) — split logic
- `src/mcp/server/spec.rs` (361 LOC) — 16 MCP tool handlers

**Stays in core (protocol):**
- `src/commands/scrape/layer/` — layer scraper reads spec files
  (reading the declaration store is protocol)
- `patterns` table in patina.db — materialized from spec file scans

## Open Questions

1. **Should Phase A (WIT interfaces) be its own spec?** Designing
   `host/fs-write`, `host/git`, and `host/mcp-register` is substantial
   work that benefits other future extractions (session extraction
   when it happens, other workflow plugins). Separating interface
   design from extraction implementation may be cleaner.

2. **MCP dynamic registration architecture.** Does each plugin register
   tools at load time? Or does the host scan installed plugin manifests
   for declared tools? The manifest approach (declare tools in
   plugin.toml, host registers on startup) is simpler than runtime
   registration and doesn't need a WIT interface.

3. **Spec type parsing.** `src/spec.rs` types (`SpecFrontmatter`,
   `ExitCriterion`) are used by both the spec commands AND the layer
   scraper. If spec commands move to a plugin, do these types stay in
   core (for the scraper) with the plugin importing them? Or does the
   plugin own the types and the scraper gets a simplified parser?

4. **Database writes.** Spec mutations write to `patterns` and
   `spec_deps` tables in patina.db. As a plugin, should specs write
   via `host/emit` (events → materialized views) or via a new
   `host/db-write` interface? The event-sourced approach is more
   consistent with [[events-are-autobiography-not-telemetry]] but
   adds complexity to what's currently direct SQL.
