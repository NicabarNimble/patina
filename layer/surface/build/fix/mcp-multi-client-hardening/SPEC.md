---
type: fix
id: mcp-multi-client-hardening
status: draft
created: 2026-03-10
sessions:
  origin: 20260310-064604
related:
- src/mcp/server/mod.rs
- src/mcp/server/spec.rs
exit_criteria:
  - Starting two MCP stdio servers in the same repo does not truncate or overwrite the same log file
  - Each MCP server instance writes to a unique per-process log path
  - MCP SQLite connections use a non-zero busy timeout so normal read/write overlap does not fail immediately
  - Normal read-mostly dual-client usage (Codex + Claude Code) is documented as supported
  - Concurrent mutating spec workflows are explicitly documented as unsupported / non-goal
  - Existing MCP tests pass and new coverage verifies the log-path and DB-open behavior
---
# fix: MCP multi-client hardening

> Running Codex and Claude Code against Patina MCP at the same time should not clobber logs or fail noisily on normal SQLite contention.

## Problem

Patina's MCP server is stdio-based, so each client launches its own process. That is good for transport isolation, but two clients in the same repo still share local operational resources:

- both instances currently write `.patina/local/logs/mcp-server.log`
- both instances open `.patina/local/data/patina.db` with default SQLite behavior
- mutating MCP tools can touch the same worktree and database concurrently

The most obvious current bug is log clobbering: the second MCP server startup truncates the first server's log file. The second issue is operational brittleness under benign overlap: SQLite can fail immediately on lock contention instead of waiting briefly for the other process to finish.

This spec is about making normal dual-client usage safe enough for read-mostly workflows, not about making all concurrent mutations transactional across agents.

## Root Cause

`src/mcp/server/mod.rs` currently does:

- `File::create(".patina/local/logs/mcp-server.log")`
- `Connection::open(".patina/local/data/patina.db")`

That creates two concrete problems:

1. **Shared log path with truncation**
   - `File::create()` truncates an existing file.
   - A second MCP process destroys the first process's log history and mixes future output into the same filename.

2. **Default SQLite connection policy**
   - `Connection::open()` with no busy timeout is brittle under concurrent access.
   - Read-mostly overlap is usually fine, but normal short write windows can still surface as avoidable `database is locked` failures.

There is also a broader concurrency truth:

- `spec.*` MCP tools can mutate files, git state, and database state.
- Two agents running spec mutations at the same time is a coordination problem, not just a transport problem.
- That broader multi-writer workflow is out of scope for this fix and should remain explicitly unsupported.

## Fix

### 1. Give each MCP process its own log file

Change `run_mcp_server()` in `src/mcp/server/mod.rs` so each process writes a unique log filename, for example:

- `mcp-server-<pid>.log`
or
- `mcp-server-<timestamp>-<pid>.log`

Requirements:

- no shared truncating filename
- path is deterministic enough to inspect manually
- startup log line includes the chosen path

Optional convenience:

- maintain `mcp-server.latest.log` as a best-effort copy/symlink/pointer if that can be done safely without reintroducing cross-process clobbering

That convenience is not required for this spec.

### 2. Centralize MCP DB opening with a busy timeout

Add a small helper in the MCP server path for opening `patina.db` with operational settings appropriate for stdio MCP use:

- non-zero busy timeout
- same existing DB path
- no semantic change to query behavior

The goal is not to redesign Patina's database layer. The goal is to avoid immediate failure on short lock overlap between two local MCP clients.

If practical, use the same helper anywhere MCP-specific code opens the same DB in-process.

### 3. Keep scope explicit: support read-mostly concurrency, not concurrent mutation workflows

Document the boundary clearly:

- supported: two MCP clients running `scry`, `context`, `assay`, `spec.show`, `spec.list`, similar read-mostly tools
- not supported: two MCP clients simultaneously performing mutating spec workflows like `spec.complete`, `spec.promote`, `spec.set`, `spec.split`

This keeps the fix aligned with [[safety-boundaries]] and avoids pretending Patina has a multi-writer transaction model across git + filesystem + SQLite when it does not.

### 4. Add focused tests

Add coverage for:

- unique log-path generation across two synthetic server instances / helper calls
- DB open helper applies a busy timeout without error

Prefer small unit tests around extracted helpers instead of trying to stand up two full MCP processes in CI.

## Non-Goals

- Full transactional coordination for concurrent `spec.*` mutations
- Global cross-process locking for the worktree
- A daemonized shared MCP process that multiple clients attach to
- Redesigning all SQLite open sites across the entire codebase
- Solving every possible `database is locked` case in unrelated CLI commands

## Exit Criteria

1. Starting two MCP stdio servers in the same repo no longer points both at `.patina/local/logs/mcp-server.log`
2. Each MCP server instance writes to a unique per-process log path
3. MCP DB opening uses a non-zero busy timeout
4. Normal dual-client read-mostly usage is documented as supported
5. Concurrent mutating spec workflows are documented as unsupported / out of scope
6. Existing MCP tests pass and new tests cover log-path uniqueness plus DB-open behavior
