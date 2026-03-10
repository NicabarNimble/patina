# Design: Extract spec subsystem to Mother-hosted WASM plugin

## Approach

Split the current spec subsystem into three layers:

1. **Core read path**
   - Keep spec types/parsing needed to read the declaration store.
   - Core can still list/show/check specs from files even without the plugin.

2. **Mother-hosted workflow plugin**
   - Move lifecycle mutation logic into a WASM extension plugin.
   - Mother hosts the plugin and becomes the execution authority for spec mutations.

3. **Thin frontends**
   - CLI and MCP no longer implement lifecycle behavior directly.
   - They route requests to Mother, which invokes the plugin.

This avoids the bad intermediate state where the code is “a plugin” but every frontend still launches its own isolated host and mutates the repo independently.

## Commits
1. `refactor(spec): classify core-vs-plugin responsibilities` — freeze the architectural split before moving code
2. `feat(plugin-host): add host capabilities for spec workflow` — scoped fs/git/routing interfaces
3. `feat(spec-plugin): add Mother-hosted spec extension plugin` — port lifecycle logic
4. `refactor(spec): route CLI and MCP through Mother` — make frontends thin
5. `refactor(core): remove in-core spec lifecycle implementation` — leave read-only parsing/indexing in place

## Key Files
- `src/spec.rs` — shared types; likely partially retained in core
- `src/commands/spec/mod.rs` — CLI routing boundary
- `src/commands/spec/internal/` — extraction target
- `src/mcp/server/spec.rs` — MCP routing boundary
- `src/plugin/internal/` — host/plugin execution plumbing
- `src/commands/mother/` — Mother daemon/authority side
- `layer/surface/build/refactor/core-plugin-extraction/SPEC.md` — parent architectural context

## Open Questions

- Should read-only spec queries stay entirely in core for simplicity?
- What is the narrowest safe host interface for git-backed lifecycle operations?
- Are manifest-declared MCP tools enough, or is runtime registration still needed?
- How should the plugin signal “mutation unavailable / Mother not running” to CLI and MCP callers?
