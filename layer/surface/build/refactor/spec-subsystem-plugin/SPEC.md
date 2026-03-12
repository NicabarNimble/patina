---
type: refactor
id: spec-subsystem-plugin
status: draft
created: 2026-03-10
blocked_by:
- scrape-simplification
sessions:
  origin: 20260310-064604
related:
- agentic-surface-architecture
- session-narrative-system
- layer/surface/build/refactor/core-plugin-extraction/SPEC.md
- layer/surface/build/refactor/core-plugin-extraction/DESIGN.md
- layer/surface/build/refactor/mother-maturation/SPEC.md
- layer/surface/build/refactor/continuous-operation/SPEC.md
- src/commands/spec/mod.rs
- src/mcp/server/spec.rs
beliefs:
- patina-is-domain-agnostic-knowledge-system
- wit-is-contract-wasm-is-one-runtime
- mother-is-connection-and-continuity
- safety-boundaries
exit_criteria:
- Spec lifecycle execution moves out of core and into a Mother-hosted WASM plugin
- Core retains spec file parsing/indexing needed for reading the declaration store
- CLI and MCP spec entrypoints become thin routing layers, not lifecycle implementations
- Mutating spec operations execute under Mother authority, not per-client direct core execution
- Required host/plugin interfaces are explicitly defined with project-scoped safety boundaries
- Patina can still read/list/show spec files even when the spec plugin is not installed
---
# refactor: Extract spec subsystem to Mother-hosted WASM plugin

> Move spec lifecycle management out of Patina core into a Mother/child WASM plugin, while keeping spec file parsing and indexing in core.

## Current State

The spec subsystem is spread across core in four places:

- `src/spec.rs` — shared spec types and frontmatter serialization
- `src/commands/spec/` — CLI command tree and lifecycle mutations
- `src/mcp/server/spec.rs` — MCP tool handlers for `spec.*`
- `patina.db` tables / filesystem interactions driven directly from core code

Today, core owns both halves of the problem:

1. **Protocol reading**
   - parse spec files from `layer/surface/build/`
   - index/show/list/check them

2. **Workflow mutation**
   - create spec directories
   - edit frontmatter
   - create tags and commits
   - archive specs
   - expose mutation tools over MCP

That mixes declaration-store reading with development-workflow tooling. It also means each MCP/CLI host process can execute spec mutations directly instead of routing them through a shared authority.

## Target State

### Core keeps the protocol responsibilities

Core still:

- parses spec frontmatter/types needed by the layer scraper
- indexes specs into `patterns` / related projections
- supports read-only views like list/show/check/history through shared query paths

This preserves the rule that reading the declaration store is always part of core.

### Mother hosts the spec workflow plugin

The spec subsystem becomes a WASM plugin (role=`extension`) hosted by Mother/child infrastructure:

- Mother loads the plugin
- the plugin handles spec lifecycle operations
- mutating operations run under Mother authority
- CLI and MCP frontends route spec commands/tool calls to Mother instead of executing lifecycle logic in-process

This gives Patina a cleaner architecture:

- **core** = protocol + stores
- **spec plugin** = workflow behavior
- **Mother** = execution authority and broker
- **session system** = narrative/handoff layer adjacent to spec
  governance, not a hidden side channel

### Design decisions

- **Keep spec-file reading in core.** A project without the plugin must still be able to scrape/index/list specs because spec files live in the declaration store.
- **Move mutation logic, not the existence of specs.** `create`, `promote`, `pause`, `resume`, `set`, `split`, `complete`, `abandon`, `archive` belong in the plugin.
- **Mother is the host for mutation authority.** Do not replace “core executes spec ops directly” with “each frontend launches its own spec plugin host.” That repeats the same architectural mistake in a different packaging.
- **Prefer manifest-declared MCP tools over runtime tool registration if possible.** If the host can register plugin-declared tools on startup, that is simpler and safer than free-form runtime registration.
- **Safety boundaries stay host-side.** The plugin requests filesystem/git actions; the host enforces path scoping, git command policy, and project boundaries.

## Steps

1. Audit the spec subsystem into three buckets:
   - stays in core (types/parsing needed for declaration-store reading)
   - moves to plugin (lifecycle workflow)
   - becomes thin routing in CLI/MCP
2. Define the host capabilities the plugin needs:
   - scoped filesystem write/create/remove
   - git operations needed for lifecycle
   - routing contract for CLI/MCP invocation through Mother
3. Decide tool exposure model:
   - manifest-declared `spec.*` tools loaded by host at startup
   - or explicit registration interface if manifest metadata is insufficient
4. Build the spec plugin as a Mother-hosted extension plugin
5. Convert core CLI and MCP spec handlers into thin routing layers
6. Remove lifecycle implementation from core while preserving read-only spec parsing/indexing
7. Verify the pluginless case:
   - specs still exist as files
   - core can still read/index/show them
   - mutation commands fail clearly if the spec plugin is unavailable

## Key Files

- `src/spec.rs` — determine what types stay in core
- `src/commands/spec/mod.rs` — becomes thin CLI routing or shrinks heavily
- `src/commands/spec/internal/` — primary extraction target
- `src/mcp/server/spec.rs` — becomes thin routing or plugin dispatch glue
- `src/plugin/internal/` — likely host-side loading/routing work
- `src/commands/mother/` and Mother daemon paths — host authority for plugin execution

## Open Questions

- Should read-only `spec.show`/`spec.list` stay fully in core, or should all `spec.*` commands route through Mother for consistency?
- Does the plugin need a dedicated `host/git` interface, or can lifecycle operations be expressed through a narrower workflow API?
- Are manifest-declared MCP tools enough, or do extension plugins need richer runtime registration?
- Should spec DB writes happen directly via a host DB interface, or should core remain responsible for materializing spec state from files/events?

## Non-Goals

- Extracting sessions or adapters
- Moving spec file parsing out of core
- Redesigning the spec lifecycle semantics before extraction
- Solving all multi-agent coordination in this spec; the goal is to put mutation authority in the right place first

## Exit Criteria

1. Spec lifecycle execution moves out of core and into a Mother-hosted WASM plugin
2. Core retains spec file parsing/indexing needed to read the declaration store
3. CLI and MCP spec entrypoints are thin routing layers rather than lifecycle implementations
4. Mutating spec operations execute under Mother authority, not per-client direct core execution
5. Required host/plugin interfaces are defined with project-scoped safety boundaries
6. Patina can still read/list/show spec files when the spec plugin is not installed
