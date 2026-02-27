---
type: refactor
id: mcp-server-hardening
status: active
created: 2026-02-26
sessions:
  origin: 20260226-152857
related:
- data-db-split-fixes
- data-architecture-v2
beliefs:
- mcp-is-shim-cli-is-product
- correctness-by-construction-not-convention
exit_criteria:
- id: eprintln-replaced-with-structured-logging
  text: all eprintln! calls in src/mcp/server/ replaced with tracing macros (info!/warn!/error!); stderr clean during normal operation
  checked: false
- id: ok-swallowing-eliminated
  text: all .ok() calls in src/mcp/server/ that discard actionable errors replaced with explicit error handling or tracing::warn!
  checked: false
- id: connection-reuse-for-patina-db
  text: patina.db opened once per server lifetime and passed through dispatch — no per-request Connection::open for the hot path
  checked: false
- id: error-codes-differentiated
  text: JSON-RPC errors use -32602 for param validation, -32603 for server errors, -32001 for missing index — not -32603 for everything
  checked: false
- id: server-runs-clean-under-mcp-inspector
  text: patina mcp produces no stderr noise when exercised through initialize, tools/list, scry, assay, context, spec.list, and measure tool calls
  checked: false
---
# refactor: MCP Server Hardening — Logging, Warnings, Connection Reuse

> The MCP server has no structured logging, silent error swallowing via
> `.ok()`, connection-per-request overhead, and undifferentiated error codes.
> Address the operational gaps across the 2,861-line server (5 modules) that
> prevent diagnosing issues in the long-running stdio context.

## Current State

The MCP server (`src/mcp/server/`, 2,861 LOC across 5 modules) works but
has three categories of operational debt:

**1. Logging via `eprintln!` (7 call sites)**

All server output goes to stderr via raw `eprintln!`. No structured logging
framework. Startup diagnostics (`mod.rs:36-57`) print secrets gate status
to stderr before the protocol loop begins. Event recording failures
(`scry.rs:1110`) print warnings mid-session. These are invisible to the
LLM client and ungreppable in any log aggregation.

**2. Silent error swallowing via `.ok()` (19 call sites)**

| Module | `.ok()` calls | Effect |
|--------|--------------|--------|
| `assay.rs` | 12 | Row deserialization failures silently dropped via `filter_map(\|r\| r.ok())` — partial results returned without indication |
| `scry.rs` | 5 | Event recording (`open_events_db().ok()?`), row mapping, query logging — all fail silently |
| `mod.rs` | 1 | `current_dir().ok()` — benign |
| `spec.rs` | 0 | Clean — uses `?` throughout |

The `filter_map(|r| r.ok())` pattern is the worst offender: if a DB schema
changes and rows fail to deserialize, the server returns fewer results with
no error, no warning, and no way to diagnose.

**3. Per-request database connections (6 call sites)**

Every tool call opens a fresh `Connection::open()`:
- `assay.rs:138` — handle_assay main path
- `assay.rs:496,520` — all_repos mode (nested in `if let Ok`)
- `scry.rs:590,709` — orient and recent modes
- `scry.rs:960` — belief grounding query

`QueryEngine` is correctly shared via immutable ref (`mod.rs:72`), but the
underlying SQLite connections are not. Each `Connection::open` pays ~1ms
overhead plus WAL checkpoint costs.

**4. Undifferentiated error codes**

31 error responses use `-32603` (Internal Error) for everything from "DB
not found" to "query failed" to "missing index." Only parameter validation
uses `-32602`. The LLM client cannot distinguish "run `patina scrape`" from
"your query is malformed" from "server bug."

## Target State

A server that is **diagnosable without reproduction** — when something goes
wrong, the structured log tells you what happened, to which request, and why.

Concretely:
- `tracing` crate with JSON subscriber writing to a log file (not stderr)
- Every `.ok()` that discards an actionable error becomes a `warn!` or `?`
- `patina.db` connection opened once at startup, threaded through dispatch
- Error codes that let the LLM client give actionable guidance to the user
- Zero stderr output during normal operation (MCP protocol purity)

**Not in scope:** async I/O, connection pooling for events.db (low-frequency
writes), timeouts (single-user sequential model), signal handling.

## Steps

1. **Add `tracing` dependency** — Add `tracing` and `tracing-subscriber`
   to Cargo.toml. Initialize a file-based JSON subscriber in
   `run_mcp_server()` before the protocol loop. Ensure
   `.patina/local/logs/` directory exists (create if missing). Log file at
   `.patina/local/logs/mcp-server.log`. Emit `info!("MCP server ready,
   log: {path}")` so the path is discoverable in the log itself.

2. **Replace `eprintln!` with tracing** — Convert all 7 `eprintln!` calls:
   - `mod.rs:36-57` (secrets gate) → `info!` spans
   - `mod.rs:74` (server ready) → `info!`
   - `scry.rs:1110` (event recording failure) → `warn!`

3. **Eliminate `.ok()` swallowing** — Address all 19 call sites:
   - `filter_map(|r| r.ok())` (12 sites in assay, 3 in scry) → log each
     deserialization failure with `warn!`. If all rows fail, return a
     JSON-RPC error (`-32002`) instead of an empty result. If some rows
     fail, return results with a `"_warnings"` field noting the count
     (e.g., `"_warnings": ["3 rows failed deserialization"]`).
     The `"_warnings"` field lives inside the `content[0].text` JSON
     payload alongside results — not a new protocol-level field. MCP
     clients parse `content[].text` as opaque text; this adds structured
     metadata within that text. No MCP schema change required.
   - `open_events_db().ok()?` (scry.rs:403) → `open_events_db().map_err(|e| warn!(...)).ok()?`
   - `mod.rs:24` (`current_dir().ok()`) — leave as-is, benign

4. **Connection reuse for patina.db** — Open `patina.db` once in
   `run_mcp_server()`, pass `&Connection` through `dispatch()` →
   handler functions. The 3 hot-path opens (`assay.rs:138`,
   `scry.rs:590,709`) use the shared connection. The all_repos mode
   (`assay.rs:496,520`) keeps per-call opens (different DB paths).
   Event recording (`scry.rs:960`, `open_events_db`) stays per-call
   (different DB, low frequency). **Threading model:** the server is
   single-threaded (`for line in reader.lines()` loop in `mod.rs:76`).
   `&Connection` is safe because all handlers execute on the caller's
   thread. `rusqlite::Connection` is not `Send`/`Sync` — any future
   async work would need to revisit this (see Non-Goals).

5. **Differentiate error codes** — Introduce error code constants and
   use them consistently:
   - `-32602`: Invalid params (already used correctly for validation)
   - `-32603`: Server internal error (bugs, unexpected failures)
   - `-32001`: Missing index ("run `patina scrape` first")
   - `-32002`: Database error (connection, query, schema mismatch)

6. **Verify clean operation** — Test with MCP inspector and Claude Code:
   - No stderr output during normal tool calls
   - Log file captures structured events
   - Error responses include actionable codes
   - Simulate a deserialization failure (e.g., alter a DB column name)
     and confirm the response includes `"_warnings"` with the count

## Non-Goals

- **Async I/O.** The sequential blocking model is correct for single-user
  MCP (one IDE, one server process). Async adds complexity without benefit.
- **Connection pooling for events.db.** Event writes are low-frequency
  (one per tool call). Per-call `open_events_db()` is fine.
- **Timeouts.** Hung queries are a theoretical concern in single-user mode.
  If this becomes real, it's a separate spec.
- **Signal handling (SIGTERM/SIGINT).** The server exits when stdin closes.
  Graceful shutdown is nice-to-have, not a hardening priority.
- **Metrics/SLOs.** No runtime to export to. The tracing log file is the
  observability layer.

## Key Files

- `src/mcp/server/mod.rs` — Server loop, dispatch, initialization (174 LOC)
- `src/mcp/server/scry.rs` — Semantic search, context, event recording (1,191 LOC)
- `src/mcp/server/assay.rs` — Structural queries, most `.ok()` sites (559 LOC)
- `src/mcp/server/spec.rs` — Spec lifecycle, cleanest module (446 LOC)
- `src/mcp/server/tools.rs` — Tool schemas (491 LOC, read-only for this spec)
- `src/eventlog.rs` — Events DB, `open_events_db()` (read-only for this spec)
