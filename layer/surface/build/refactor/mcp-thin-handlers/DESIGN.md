# Design: Collapse MCP handlers to thin CLI wrappers

## Approach

Follow the spec.rs reference pattern: MCP handlers receive typed args,
call a `_json()` or `_value()` function in `src/commands/`, wrap the
result in `Response::success()`. Three phases, lowest-risk first.

### The `_json()` pattern

Each duplicated code path becomes a function in CLI internals that
returns `Result<String>` (JSON). MCP calls it and wraps in Response.
CLI calls it when `--json` is passed, or calls a sibling function
that prints human-formatted output from the same data.

```rust
// src/commands/assay/internal/inventory.rs
pub fn inventory_json(conn: &Connection, pattern: &str, limit: usize) -> Result<String> {
    let (modules, failures) = query_inventory(conn, pattern, limit)?;
    serialize_result(serde_json::json!({ "modules": modules, ... }), failures)
}

// src/mcp/server/assay.rs — thin wrapper
QueryType::Inventory => {
    let pattern = options.pattern.as_deref().unwrap_or("%");
    match inventory_json(conn, pattern, limit) {
        Ok(text) => Response::success(req.id.clone(), ...),
        Err(e) => Response::error(req.id.clone(), ERR_DATABASE, &e.to_string()),
    }
}
```

### What stays in MCP vs what moves to CLI

**Moves to CLI internals (business logic):**
- All SQL queries (orient composite-score, recent temporal, assay structural)
- Data retrieval (detail event lookup, use event logging, query logging)
- `format_detail` / `format_detail_content` (identical — keep one)
- `collect_rows()` / `serialize_result()` (shared helpers)

**Stays in MCP (consumer-specific presentation):**
- `format_results()` / `format_result_header()` — LLM-optimized output with
  contribution strings, oracle ranks, structural annotations, source tags.
  CLI has its own formatting in `semantic.rs`. These are NOT duplicates —
  they serve different consumers with different information density needs.
- `annotate_impact()` — thin bridge: converts FusedResult→ScryResult then
  calls shared `find_belief_impact()`. The conversion is MCP-specific.
- `emit_usage_event()` — shared helper, stays in MCP for now (only MCP
  handlers call it; CLI logs usage through its own paths).

### Phase 1 strategy: Assay (mechanical)

The 7 inline SQL query types in `src/mcp/server/assay.rs` have
byte-identical SQL to CLI internals. The existing delegation pattern
(Search/Cochange/Belief already call `_json()` functions) proves the
approach. Extend it to the remaining 7.

File organization in `src/commands/assay/internal/`:
- `inventory.rs` — already has `execute_inventory()`, add `inventory_json()`
- `imports.rs` — already has `execute_imports()`, add `imports_json()`, `importers_json()`
- `functions.rs` — already has `execute_functions()`, add `functions_json()`, `callers_json()`, `callees_json()`
- `derive.rs` — already has `execute_derive()`, add `derive_json()`

The `all_repos` path gets `inventory_all_repos_json()` — currently
duplicated with different JSON structure (MCP adds `"repo"` field).
Unify by always including `"repo"` field (CLI can ignore it or use it).

### Phase 2 strategy: Scry (nuanced)

Scry functions fall into three categories requiring different treatment:

**Category A — True SQL duplication (orient, recent):**
Same strategy as assay. Extract SQL to `_json()` functions. Both
consumers format independently. ~240 LOC removed from MCP.

**Category B — Feedback loop operations (detail, use, log_query):**
These share events.db and form a chain: query→detail→use. Must be
collapsed as a group to preserve the feedback loop.

Key fix: MCP `handle_use()` currently MISSING `mark_edge_usage_from_query()`
that CLI's `log_scry_use()` has. After collapse, both paths go through
the unified function, fixing the graph routing feedback gap.

For `log_query` unification, the type split is the main design decision
(see below).

**Category C — Formatting-only duplication (why):**
`handle_why()` already shares `QueryEngine` — no parallel search logic.
Only formatting differs (~60 LOC). Extract `why_json()` returning
structured data; each consumer formats. Lightest-touch change.

### FusedResult vs ScryResult — the type split decision

MCP scry operates on `FusedResult` (from QueryEngine). CLI legacy
paths operate on `ScryResult`. Key field differences:

| Field | FusedResult | ScryResult |
|-------|------------|------------|
| doc identifier | `.doc_id` | `.source_id` |
| score | `.fused_score` | `.score` |
| event type | `.metadata.event_type` (Option) | `.event_type` (String) |

`log_mcp_query()` and `log_scry_query()` both build the same JSON
structure from these different types. Three options considered:

**(a) Trait with `doc_id()` + `score()` + `event_type()` methods.**
Clean but adds a trait just for logging. Over-engineered for 3 fields.

**(b) Convert FusedResult→ScryResult at call site.**
`annotate_impact()` already does this exact conversion (scry.rs:483-496).
Extract the conversion to a shared helper. Both callers convert before
calling the unified log function.

**(c) Accept `Vec<serde_json::Value>` for the results array.**
Caller builds the JSON array, function just logs it. Loses type safety
but is simplest. Each caller already builds this array today.

**Decision: Option (b).** Reuse the existing conversion pattern. The
FusedResult→ScryResult conversion is 14 lines and already proven.

## Commits

Plan (will be refined during implementation):

1. `assay: extract _json() functions for 7 structural query types` — Phase 1 steps 1-2
2. `assay: unify all_repos between MCP and CLI` — Phase 1 step 3
3. `scry: extract orient_json() and recent_json()` — Phase 2 step 4
4. `scry: unify detail/use with edge-marking fix` — Phase 2 step 5
5. `scry: collapse format_detail_content/format_detail` — Phase 2 step 6
6. `scry: extract why_json(), unify query logging` — Phase 2 steps 7-8
7. `scry: move expanded_terms to QueryOptions` — Phase 2 step 9
8. `cleanup: move collect_rows/serialize_result to shared module` — Phase 3

## Key Files

**MCP handlers (being collapsed):**
- `src/mcp/server/assay.rs` (603 LOC) — 7 query types with inline SQL + all_repos
- `src/mcp/server/scry.rs` (1,255 LOC) — orient/recent/why/detail/use + formatters + logging

**CLI internals (gaining `_json()` functions):**
- `src/commands/assay/internal/inventory.rs` — inventory + all_repos
- `src/commands/assay/internal/imports.rs` — imports + importers
- `src/commands/assay/internal/functions.rs` — functions + callers + callees
- `src/commands/assay/internal/derive.rs` — derive signals
- `src/commands/scry/internal/subcommands.rs` — orient + recent (has execute_ functions today)
- `src/commands/scry/internal/logging.rs` — query logging + use logging (unification target)
- `src/commands/scry/mod.rs` — detail + format_detail (keep here or move to internal/)

**Reference implementation:**
- `src/mcp/server/spec.rs` (348 LOC) — the target pattern: every arm calls `_value()`
- `src/commands/spec/mod.rs` — the CLI side that spec.rs delegates to

**Shared infrastructure:**
- `src/retrieval/` — QueryEngine, FusedResult, snippet() — already shared
- `src/commands/scry/internal/enrichment.rs` — find_belief_impact() — already shared

## Open Questions

1. **Should `collect_rows()` / `serialize_result()` move in Phase 1 or Phase 3?**
   Phase 1 `_json()` functions need these helpers. Moving them first
   (Phase 1 step 1) is more natural than the spec's Phase 3 placement.
   Likely resolve: move in Phase 1, adjust step ordering.

2. **Where does `format_detail` live after collapse?**
   Currently in `src/commands/scry/mod.rs` (as `format_detail`) and
   `src/mcp/server/scry.rs` (as `format_detail_content`). Options:
   keep in `scry/mod.rs` (MCP imports it), or move to
   `scry/internal/` (both import via re-export). Preference: keep in
   `scry/mod.rs` since `execute_detail()` is also there.

3. **Should `emit_usage_event()` move to a shared location?**
   Currently in MCP scry.rs, called by both scry and assay MCP handlers.
   If assay collapses fully, the MCP handler may still want usage events.
   Likely stays in `src/mcp/server/` as a module-level helper.
