---
type: refactor
id: mcp-server-split
status: draft
created: 2026-02-25
sessions:
  origin: 20260225-082251
---
# refactor: Split MCP server.rs into domain-grouped modules

> server.rs is 2,579 lines with 20 tools — inline JSON schemas + handler match arms. Split into domain modules following dependable-rust pattern.

## Problem

`src/mcp/server.rs` is a single 2,579-line file containing:
- **Tool schemas** (lines 141–561): 20 inline JSON schema definitions in `handle_list_tools()`
- **Handler dispatch** (lines 563–1360): one giant match arm in `handle_tool_call()` routing all 20 tools
- **Query helpers** (lines 1362–2579): `execute_assay()`, `handle_orient()`, `handle_recent()`, `handle_why()`, format helpers, etc.

Every new MCP tool (like spec.show) adds ~30 lines of schema + ~20 lines of handler to the same file. The file already exceeded Claude Code's 25K-token read limit during the spec-show-mcp session — it had to be read in sections.

The handler match arms for spec tools are highly repetitive: extract `id` from args, validate non-empty, call `_value()` function, serialize result. This repetition is a sign of missing abstraction.

## Root Cause

The MCP server was built incrementally — each tool was added to the single file as a new match arm + inline schema. No splitting point was established early, so it grew linearly with the tool count.

## Refactor

Split `server.rs` into the `internal/` pattern:

```
src/mcp/
├── mod.rs              # pub use server::run_mcp_server (unchanged)
├── protocol.rs         # Request/Response types (unchanged)
└── server/
    ├── mod.rs          # run_mcp_server, dispatch, handle_initialize
    ├── tools.rs        # handle_list_tools() — all schema definitions
    ├── handlers/
    │   ├── mod.rs      # handle_tool_call() dispatch routing
    │   ├── scry.rs     # scry + context + mother handlers + format/orient/recent/why/use/detail helpers
    │   ├── assay.rs    # assay handler + execute_assay + execute_assay_all_repos
    │   ├── spec.rs     # all spec.* handlers (query + mutation)
    │   └── schema.rs   # schemas.list + schemas.show handlers
    └── log.rs          # log_mcp_query (query logging)
```

**Line count estimates:**
- `server/mod.rs`: ~120 lines (entry, dispatch, initialize, secrets gate)
- `tools.rs`: ~430 lines (all schema JSON)
- `handlers/scry.rs`: ~950 lines (scry/context/mother + all retrieval helpers)
- `handlers/assay.rs`: ~400 lines (assay + execute functions)
- `handlers/spec.rs`: ~350 lines (14 spec tools)
- `handlers/schema.rs`: ~40 lines
- `log.rs`: ~50 lines

**What does NOT change:**
- Public API: `run_mcp_server()` stays the only export
- `protocol.rs`: untouched
- `mod.rs`: untouched (still re-exports `run_mcp_server`)
- Handler behavior: pure file moves, no logic changes
- JSON schema content: identical, just in a different file

## Steps

1. Create `src/mcp/server/` directory, move `server.rs` → `server/mod.rs`
2. Extract `handle_list_tools()` → `server/tools.rs`
3. Create `server/handlers/mod.rs` with `handle_tool_call()` dispatch routing
4. Extract scry/context/mother handlers + all format/orient/recent/why/use/detail helpers → `handlers/scry.rs`
5. Extract assay handler + `execute_assay` + `execute_assay_all_repos` → `handlers/assay.rs`
6. Extract all spec.* handlers → `handlers/spec.rs`
7. Extract schemas.* handlers → `handlers/schema.rs`
8. Extract `log_mcp_query` → `server/log.rs`
9. Verify: `cargo build --release`, `cargo test`, pre-push checks

## Key Files

```
src/mcp/server.rs       — current monolith (becomes server/mod.rs)
src/mcp/mod.rs          — re-export (unchanged)
src/mcp/protocol.rs     — types (unchanged)
```

## Exit Criteria

- [ ] `server.rs` replaced by `server/` directory with 7+ files
- [ ] No file exceeds ~1,000 lines
- [ ] `run_mcp_server()` remains the only public export from `src/mcp/`
- [ ] All 20 MCP tools still work (cargo test, live test via MCP client)
- [ ] Zero logic changes — pure structural refactor
- [ ] `cargo clippy` clean, `cargo fmt` clean, pre-push checks pass
