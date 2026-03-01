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
exit_criteria:
- id: scry-handlers-delegate
  text: handle_scry dispatches each mode to a _json() or _value() function in src/commands/scry/ — no SQL, no QueryEngine calls, no format_results in src/mcp/server/scry.rs
  checked: false
- id: assay-handlers-delegate
  text: assay handle() dispatches each query type to a _json() function in src/commands/assay/ — no inline SQL in src/mcp/server/assay.rs except for the shared conn passthrough
  checked: false
- id: zero-duplicate-sql
  text: zero SQL statements duplicated between src/mcp/server/ and src/commands/ — verified by grep
  checked: false
- id: zero-duplicate-formatters
  text: format_detail_content (MCP) and format_detail (CLI) collapsed to one function — no parallel formatting code
  checked: false
- id: mcp-server-loc-under-500
  text: src/mcp/server/{scry,assay}.rs combined LOC under 500 (currently ~1,830) — protocol wrapping only
  checked: false
- id: existing-tests-pass
  text: all tests pass, MCP inspector exercised for all tools
  checked: false
---
# refactor: Collapse MCP Handlers to Thin CLI Wrappers

> MCP scry/assay handlers reimplement ~2,500 LOC of business logic that
> exists in CLI internals. Collapse to thin delegation (the spec.rs
> pattern) by creating `_json()` functions in CLI modules. Retire the
> parallel implementations.

## Current State

[[mcp-typed-handlers]] (v0.35.2) typed the parameter extraction layer.
The handlers now receive typed structs instead of `&serde_json::Value`.
But the business logic behind those typed args is still bifurcated:

**spec.rs — the reference pattern (thin wrapper, zero duplication):**
Every match arm calls a `_value()` function from `crate::commands::spec`.
MCP adds only protocol wrapping. Zero SQL. Zero formatting. ~340 LOC.

**scry.rs — parallel implementation (~1,225 LOC):**

| Function | What it does | CLI equivalent |
|----------|-------------|----------------|
| `handle_orient()` | Runs composite-score SQL, formats markdown | `execute_orient()` in scry/mod.rs — identical SQL |
| `handle_recent()` | Runs temporal SQL, deduplicates, formats | `execute_recent()` — identical SQL, different column count |
| `handle_why()` | Queries engine, finds match, formats contributions | `execute_why()` — same logic, more fields |
| `handle_detail()` | Reads events.db + patina.db, formats by type | `execute_detail()` — parallel implementation |
| `handle_use()` | Reads events.db, writes scry.use event | `log_scry_use()` — parallel implementation |
| `format_results()` | Contribution-aware snippet formatting | No CLI equivalent — MCP-only |
| `format_detail_content()` | Type-aware full content formatting | `format_detail()` in scry/mod.rs — identical logic, different name |
| `log_mcp_query()` | Generates query_id, writes scry.query event | `log_scry_query()` — parallel implementation |
| `annotate_impact()` | Computes belief impact, appends section | CLI legacy path has similar — partial overlap |

**assay.rs — mixed delegation (~605 LOC):**

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
result in `Response::success()`. No SQL, no formatting, no business
logic in `src/mcp/server/`.

MCP-unique concerns (expanded_terms augmentation, impact annotation,
query_id feedback loop) move to the CLI internal functions as options,
so they're available to both paths.

```
src/mcp/server/scry.rs    1,225 LOC → ~200 LOC (dispatch + protocol wrapping)
src/mcp/server/assay.rs     605 LOC → ~150 LOC (dispatch + protocol wrapping)
src/commands/scry/          gains _json() functions for orient/recent/why/detail/use
src/commands/assay/         gains _json() functions for structural queries
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

5. **Unify detail/use/why.** Create shared functions in
   `scry::internal::` for detail lookup, use logging, and why
   explanation. Both MCP and CLI call the same functions.

6. **Unify `format_detail_content` / `format_detail`.** One
   function, one location. Both paths use it.

7. **Unify query logging.** `log_mcp_query()` and
   `log_scry_query()` become one function in
   `scry::internal::logging`.

### Phase 3: MCP-unique concerns

8. **Move expanded_terms to QueryOptions.** Add `expanded_terms:
   Vec<String>` to `QueryOptions` so the CLI can use it too.
   Remove the concatenation logic from MCP handler.

9. **Move impact annotation to scry internals.** The
   `annotate_impact()` function moves to `scry::internal::` and
   becomes available via an option. MCP and CLI both use it.

10. **Move `format_results()` to scry internals.** The
    contribution-aware snippet formatter is MCP-only today but
    useful enough to be a CLI `--verbose` format option.

## Non-Goals

- **Removing MCP entirely.** MCP is the LLM-agnostic discovery
  protocol. The `tools/list` schema is valuable. The execution path
  just needs to be thin.
- **Changing spec.rs.** It's already the reference implementation.
- **Changing tools.rs.** Tool schemas are independent of execution.
- **Touching src/commands/measure/.** That's data-measure-surface scope.

## Exit Criteria

- [ ] scry.rs dispatches each mode to a `_json()` / `_value()` function — no SQL, no engine calls, no formatters
- [ ] assay.rs dispatches each query type to a `_json()` function — no inline SQL
- [ ] Zero SQL statements duplicated between `src/mcp/server/` and `src/commands/`
- [ ] `format_detail_content` and `format_detail` collapsed to one function
- [ ] `src/mcp/server/{scry,assay}.rs` combined LOC under 500 (currently ~1,830)
- [ ] All tests pass, MCP inspector exercised
