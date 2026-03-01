---
type: refactor
id: mcp-thin-handlers
status: draft
created: 2026-03-01
sessions:
  origin: 20260301-100052
related:
- mcp-typed-handlers
- mcp-server-hardening
beliefs:
- mcp-is-discovery-cli-is-execution
- mcp-is-shim-cli-is-product
- bridges-become-permanent
- ground-assertions-or-pay-review-tax
exit_criteria:
- id: scry-handlers-delegate
  text: handle_scry dispatches each mode to _json()/_value() functions in src/commands/scry/ — no direct SQL in src/mcp/server/scry.rs. MCP-specific formatting (format_results, annotate_impact) stays in MCP.
  checked: false
- id: assay-handlers-delegate
  text: assay handle() dispatches each query type to a _json() function in src/commands/assay/ — no inline SQL in src/mcp/server/assay.rs
  checked: false
- id: zero-duplicate-sql
  text: "zero SQL statements duplicated between src/mcp/server/ and src/commands/ — verified: `rg 'SELECT|FROM|WHERE|ORDER BY' src/mcp/server/{scry,assay}.rs` returns zero"
  checked: false
- id: zero-duplicate-functions
  text: format_detail_content/format_detail collapsed to one; log_mcp_query/log_scry_query unified — no parallel implementations of the same logic
  checked: false
- id: feedback-loop-preserved
  text: query_id feedback loop (scry.query → scry.detail → scry.use) works through all 3 operations; scry.use includes mark_edge_usage_from_query for graph routing feedback
  checked: false
- id: mcp-server-loc-under-700
  text: "src/mcp/server/{scry,assay}.rs combined LOC under 700 (currently 1,858) — verified: `wc -l src/mcp/server/{scry,assay}.rs`"
  checked: false
- id: existing-tests-pass
  text: all tests pass, MCP inspector exercised for all tools
  checked: false
---
# refactor: Collapse MCP Handlers to Thin CLI Wrappers

> MCP scry/assay handlers contain ~1,858 LOC (1,255 + 603) of which
> ~1,200 duplicates business logic that exists in CLI internals.
> Collapse to thin delegation (the spec.rs pattern) by creating
> `_json()` functions in CLI modules. Retire the parallel implementations.

## Current State

[[mcp-typed-handlers]] (v0.35.2) typed the parameter extraction layer.
The handlers now receive typed structs instead of `&serde_json::Value`.
But the business logic behind those typed args is still bifurcated:

**spec.rs — the reference pattern (thin wrapper, zero duplication):**
Every match arm calls a `_value()` function from `crate::commands::spec`.
MCP adds only protocol wrapping. Zero SQL. Zero formatting. ~340 LOC.

**scry.rs — parallel implementation (1,255 LOC):**

| Function | LOC | What it does | CLI equivalent | Duplication |
|----------|-----|-------------|----------------|-------------|
| `handle_orient()` | ~125 | Composite-score SQL, markdown output | `execute_orient()` in subcommands.rs | **TRUE** — byte-identical SQL (lines 642-671 vs 70-99) |
| `handle_recent()` | ~115 | Temporal SQL, dedup, markdown output | `execute_recent()` in subcommands.rs | **TRUE** — identical core query; CLI adds unused window function |
| `handle_why()` | ~60 | Calls QueryEngine, formats contributions | `execute_why()` in subcommands.rs | **Formatting only** — both call shared `engine.query_with_options()` |
| `handle_detail()` | ~70 | Reads events.db + patina.db, formats | `execute_detail()` in scry/mod.rs | **TRUE** — parallel event lookup, both call format_detail variants |
| `handle_use()` | ~55 | Reads events.db, writes scry.use event | `log_scry_use()` in logging.rs | **TRUE** — but MCP version MISSING `mark_edge_usage_from_query()` |
| `format_results()` | ~80 | Contribution-aware snippet formatting | `execute_semantic()` in semantic.rs | **Partial** — header formatting logic overlaps; `snippet()` is shared |
| `format_detail_content()` | ~75 | Type-aware full content formatting | `format_detail()` in scry/mod.rs | **IDENTICAL** — character-for-character same code, different name |
| `log_mcp_query()` | ~50 | Generates query_id, writes scry.query | `log_scry_query()` in logging.rs | **TRUE** — same structure, different result types (FusedResult vs ScryResult) |
| `annotate_impact()` | ~40 | FusedResult→ScryResult, calls shared fn | CLI uses `find_belief_impact()` directly | **Wrapper** — delegates to shared `find_belief_impact()` |

**assay.rs — mixed delegation (603 LOC):**

| Query Type | Current | Should be |
|-----------|---------|-----------|
| Search | Delegates to `assay_search_json()` | Already correct |
| Cochange | Delegates to `execute_cochange_json()` | Already correct |
| Belief | Delegates to `execute_belief_grounding_json()` | Already correct |
| Inventory | Inline SQL (~40 LOC) | Delegate to `_json()` |
| Imports | Inline SQL (~20 LOC) | Delegate to `_json()` |
| Importers | Inline SQL (~20 LOC) | Delegate to `_json()` |
| Functions | Inline SQL (~50 LOC) | Delegate to `_json()` |
| Callers | Inline SQL (~20 LOC) | Delegate to `_json()` |
| Callees | Inline SQL (~20 LOC) | Delegate to `_json()` |
| Derive | Inline SQL (~40 LOC) | Delegate to `_json()` |
| `all_repos` | Inline SQL (~80 LOC) | Delegate to `_json()` |

## Target State

Every MCP handler follows the spec.rs pattern: receive typed args,
call a `_json()` or `_value()` function in `src/commands/`, wrap the
result in `Response::success()`. No direct SQL in `src/mcp/server/`.

MCP-specific presentation (format_results, annotate_impact) stays in
`src/mcp/server/scry.rs` — this is consumer-specific formatting, not
business logic. The distinction: SQL queries and data retrieval move
to CLI internals; result formatting for LLM consumption stays in MCP.

```
src/mcp/server/scry.rs    1,255 LOC → ~400 LOC (dispatch + protocol wrapping + MCP formatting)
src/mcp/server/assay.rs     603 LOC → ~150 LOC (dispatch + protocol wrapping)
src/commands/scry/          gains _json() functions for orient/recent/detail/use + unified logging
src/commands/assay/         gains _json() functions for 7 structural query types
```

## Steps

### Phase 1: Assay structural queries (lowest risk, clearest duplication)

1. **Create `_json()` functions for 7 assay query types.** In
   `src/commands/assay/internal/`, add functions like
   `inventory_json(conn, pattern, limit) -> Result<String>` that
   contain the SQL currently in `src/mcp/server/assay.rs`. Keep
   `collect_rows()` and `serialize_result()` — move them to the
   shared internal module.

2. **Collapse assay MCP handler.** Each query type branch calls the
   new `_json()` function. Remove inline SQL from assay.rs.

3. **Collapse `execute_assay_all_repos()`.** The MCP all_repos
   implementation duplicates the CLI's. Create a shared
   `inventory_all_repos_json()` function.

### Phase 2: Scry mode handlers (higher value, more nuance)

4. **Create `_json()` functions for orient/recent.** Move
   `handle_orient()` SQL to `scry::internal::orient_json()`. Same
   for `handle_recent()`. The CLI `execute_orient()` and
   `execute_recent()` can then call these too, eliminating their
   own copies.

5. **Unify detail/use.** Create shared functions in
   `scry::internal::` for detail lookup and use logging. Both MCP
   and CLI call the same functions. **Note:** MCP `handle_use()`
   currently MISSING `mark_edge_usage_from_query()` that CLI's
   `log_scry_use()` has — collapse fixes this gap. The query_id
   feedback loop (scry.query → scry.detail → scry.use) shares
   events.db; test all three operations end-to-end after collapse.

6. **Unify `format_detail_content` / `format_detail`.** These are
   character-for-character identical. Delete one, keep the other
   in `scry::internal::` or `scry/mod.rs`. Both paths use it.

7. **Simplify why handler.** `handle_why()` already shares
   `QueryEngine` with CLI — the duplication is formatting only
   (~60 LOC). Extract a shared `why_json()` that returns structured
   data; MCP and CLI format independently.

8. **Unify query logging.** `log_mcp_query()` and
   `log_scry_query()` become one function in
   `scry::internal::logging`. **Design decision:** MCP operates on
   `FusedResult` (fields: `doc_id`, `fused_score`,
   `metadata.event_type`), CLI operates on `ScryResult` (fields:
   `source_id`, `score`, `event_type`). Options: (a) trait with
   `doc_id()` + `score()` + `event_type()` methods, (b) convert
   FusedResult→ScryResult at call site, (c) accept
   `Vec<serde_json::Value>` for the results array. Option (b) is
   simplest — annotate_impact already does this conversion.

9. **Move expanded_terms to QueryOptions.** Add `expanded_terms:
   Vec<String>` to `QueryOptions` so the CLI can use it too.
   Remove the concatenation logic from MCP find/full handlers.
   This is a 3-line change per call site.

### Phase 3: Cleanup and verification

10. **Move `collect_rows()` / `serialize_result()` to shared module.**
    These assay.rs helpers (row collection with failure tracking,
    JSON result serialization with `_warnings` injection) are useful
    in the `_json()` functions created in Phase 1. Move to
    `src/commands/assay/internal/` so both paths use the same
    failure-handling discipline.

11. **Verify MCP-specific code stays in MCP.** After collapse,
    `src/mcp/server/scry.rs` should retain: `format_results()` /
    `format_result_header()` (LLM-optimized presentation),
    `annotate_impact()` (FusedResult→ScryResult bridge to shared
    `find_belief_impact()`), and `emit_usage_event()` (shared
    helper). These are consumer-specific, not business logic.

## Risks

1. **FusedResult vs ScryResult type split.** MCP scry operates on
   `FusedResult` (from QueryEngine), CLI legacy paths on `ScryResult`.
   Different field names (`doc_id`/`source_id`, `fused_score`/`score`).
   Query logging unification (step 8) must handle this. Recommended:
   convert at call site (annotate_impact already does this).

2. **Edge marking gap.** MCP `handle_use()` does NOT call
   `mark_edge_usage_from_query()` — graph routing feedback only works
   from CLI. Collapse should fix this by routing both through
   `log_scry_use()` which includes edge marking.

3. **Output format divergence.** MCP returns markdown; CLI prints
   ASCII tables. The `_json()` functions return structured data — both
   consumers add their own formatting layer. For orient/recent this
   means both sides gain a ~30 LOC formatter. Total code increases
   slightly in the short term but duplication drops to zero.

4. **QueryEngine lifetime.** MCP receives `&QueryEngine` from server
   startup. CLI orient/recent open their own DB connections. Shared
   `_json()` functions take `&Connection` parameter — CLI opens before
   calling, MCP passes its shared connection.

## Non-Goals

- **Removing MCP entirely.** MCP is the LLM-agnostic discovery
  protocol. The `tools/list` schema is valuable. The execution path
  just needs to be thin.
- **Changing spec.rs.** It's already the reference implementation.
- **Changing tools.rs.** Tool schemas are independent of execution.
- **Touching src/commands/measure/.** That's data-measure-surface scope.

## Exit Criteria

- [ ] scry.rs dispatches each mode to `_json()`/`_value()` functions — no direct SQL; MCP-specific formatting stays in MCP
- [ ] assay.rs dispatches each query type to a `_json()` function — no inline SQL
- [ ] Zero SQL duplicated between `src/mcp/server/` and `src/commands/` — `rg 'SELECT|FROM|WHERE|ORDER BY' src/mcp/server/{scry,assay}.rs` returns zero
- [ ] `format_detail_content`/`format_detail` collapsed; `log_mcp_query`/`log_scry_query` unified — no parallel implementations
- [ ] query_id feedback loop preserved: scry.query → scry.detail → scry.use all work; scry.use includes `mark_edge_usage_from_query`
- [ ] `src/mcp/server/{scry,assay}.rs` combined LOC under 700 (currently 1,858)
- [ ] All tests pass, MCP inspector exercised
