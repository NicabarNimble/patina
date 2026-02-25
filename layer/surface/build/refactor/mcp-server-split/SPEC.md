---
type: refactor
id: mcp-server-split
status: active
created: 2026-02-25
sessions:
  origin: 20260225-082251
beliefs:
- dependable-rust
- unix-philosophy
---
# refactor: Split MCP server.rs by domain responsibility

> server.rs is 2,579 lines handling transport, schemas, and 4 unrelated tool domains. Split by concept, not by size.

## Problem

`src/mcp/server.rs` fails the "Do X" test — it does transport + dispatch + schema declaration + retrieval handlers + assay handlers + spec handlers + formatting helpers. That's not one job.

It already exceeded Claude Code's 25K-token read limit during spec-show-mcp — had to be read in sections. Every new tool adds ~50 lines to the same file across two locations (schema + handler).

## Root Cause

Built incrementally — each tool added as a new match arm + inline schema. No splitting point was established, so it grew linearly with tool count.

## Refactor

Flat modules by domain concept. No `handlers/` nesting — there's no shared handler trait, so a directory would imply structure that doesn't exist.

```
src/mcp/
├── mod.rs              # pub use server::run_mcp_server (unchanged)
├── protocol.rs         # Request/Response types (unchanged)
└── server/
    ├── mod.rs          # run_mcp_server, dispatch, handle_initialize, secrets gate (~120 lines)
    ├── tools.rs        # handle_list_tools() — all 20 schema definitions (~430 lines)
    ├── scry.rs         # scry/context/mother handlers + format/orient/recent/why/use/detail + log_mcp_query (~950 lines)
    ├── assay.rs        # assay handler + execute_assay + execute_assay_all_repos (~400 lines)
    └── spec.rs         # all spec.* + schemas.* handlers (~400 lines)
```

**Why this split:**
- Each file passes "Do X": tools.rs declares schemas. scry.rs handles retrieval queries. assay.rs handles structural queries. spec.rs handles spec lifecycle + schema introspection.
- `schemas.*` handlers (40 lines) fold into `spec.rs` — same glue pattern, not worth a standalone file.
- `log_mcp_query` stays in `scry.rs` — only called from retrieval context.
- Flat, not nested. Jon Gjengset's rule: nest when nesting carries meaning, not for grouping.

**What does NOT change:**
- Public API: `run_mcp_server()` stays the only export
- `protocol.rs`: untouched
- `mod.rs`: untouched
- Handler behavior: pure file moves, no logic changes
- JSON schema content: identical

## Steps

1. Create `src/mcp/server/` directory, move `server.rs` → `server/mod.rs`
2. Extract `handle_list_tools()` → `server/tools.rs`
3. Extract scry/context/mother match arms + all retrieval helpers (format_results, handle_orient, handle_recent, handle_why, handle_use, handle_detail, handle_mother_*, log_mcp_query, annotate_impact, format_detail_content) → `server/scry.rs`
4. Extract assay match arm + `execute_assay` + `execute_assay_all_repos` → `server/assay.rs`
5. Extract all spec.* + schemas.* match arms → `server/spec.rs`
6. `server/mod.rs` retains: run_mcp_server, dispatch, handle_initialize, check_secrets_gate, handle_tool_call (now a thin router calling into scry/assay/spec)
7. Verify: `cargo build --release && cargo install --path . && ./resources/git/pre-push-checks.sh`

## Key Files

```
src/mcp/server.rs       — current monolith (becomes server/mod.rs)
src/mcp/mod.rs          — re-export (unchanged)
src/mcp/protocol.rs     — types (unchanged)
```

## Exit Criteria

- [ ] `server.rs` replaced by `server/` directory with 5 files
- [ ] No file exceeds ~1,000 lines
- [ ] `run_mcp_server()` remains the only public export from `src/mcp/`
- [ ] All 20 MCP tools still work (cargo test, live test)
- [ ] Zero logic changes — pure structural refactor
- [ ] Pre-push checks pass
