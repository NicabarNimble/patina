# Design: MCP Server Hardening — Logging, Warnings, Connection Reuse

## Approach

6-step sequential implementation, each with its own commit:

1. Add `tracing` + `tracing-subscriber` deps, initialize file-based JSON subscriber
   in `run_mcp_server()` before the protocol loop. Log to `.patina/local/logs/mcp-server.log`.
2. Replace all 7 `eprintln!` calls (6 in mod.rs, 1 in scry.rs) with tracing macros.
3. Eliminate `.ok()` swallowing via `collect_rows()` helper + `serialize_result()` for
   `_warnings` injection. 18 actionable sites across assay.rs (12) and scry.rs (6).
4. Open `patina.db` once, thread `&Connection` through dispatch. 3 hot-path opens removed.
5. Define error code constants (ERR_INTERNAL, ERR_MISSING_INDEX, ERR_DATABASE) and
   categorize all 31 error responses by root cause.
6. Verify: pre-push checks pass, zero stderr, structured JSON in log file.

## Commits
1. `feat(mcp): add tracing subscriber with file-based JSON logging` — deps + subscriber init
2. `refactor(mcp): replace all eprintln! with tracing macros` — 7 sites converted
3. `refactor(mcp): eliminate .ok() swallowing with warn! logging` — collect_rows helper, _warnings field
4. `refactor(mcp): reuse patina.db connection across server lifetime` — thread &Connection
5. `refactor(mcp): differentiate JSON-RPC error codes` — 4 codes for 31 sites
6. `fix: cargo fmt` — formatting normalization

## Key Files
- `src/mcp/server/mod.rs` — server loop, dispatch, error code constants, tracing init
- `src/mcp/server/scry.rs` — retrieval handlers (orient, recent, find, belief, detail)
- `src/mcp/server/assay.rs` — structural queries, collect_rows/serialize_result helpers
- `src/mcp/server/spec.rs` — spec lifecycle (error codes only, no other changes)
- `Cargo.toml` — tracing, tracing-subscriber deps

## Open Questions
None remaining.
