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

### ADR-1: Helper signatures — `&Connection` not `&QueryEngine`

**Context:** MCP server holds a long-lived `&QueryEngine` (constructed
at startup, owns semantic oracles). CLI subcommands open fresh
`Connection::open(eventlog::PATINA_DB)` per call (subcommands.rs:39,
171). Both need to call the new `_json()` functions.

**Decision:** All `_json()` helpers for SQL-based queries (orient,
recent, assay 7 types) take `&rusqlite::Connection`. This matches the
existing pattern — every assay `execute_*` function already takes
`&Connection` (inventory.rs:37, functions.rs:33, imports.rs:20,
derive.rs:96). MCP already has a shared `conn` passed through from
`mod.rs` dispatch. CLI opens its own before calling.

For QueryEngine-based functions (why, find, full), the shared
functions take `&QueryEngine` directly. MCP passes its existing
reference. CLI constructs `QueryEngine::new()` — same as
`execute_why()` does today (subcommands.rs:293).

```rust
// SQL-based: takes &Connection (assay, orient, recent, detail, use)
pub fn orient_json(conn: &Connection, path: &str, limit: usize) -> Result<String>
pub fn recent_json(conn: &Connection, query: Option<&str>, days: u32, limit: usize) -> Result<String>
pub fn inventory_json(conn: &Connection, pattern: &str, limit: usize) -> Result<String>
pub fn detail_json(query_id: &str, rank: usize) -> Result<String>  // opens own events.db + patina.db

// QueryEngine-based: takes &QueryEngine (why)
pub fn why_json(engine: &QueryEngine, doc_id: &str, query: &str) -> Result<String>
```

**No wrapper struct.** A `QueryContext { conn, engine }` would couple
every helper to both systems. SQL helpers don't need QueryEngine.
QueryEngine helpers don't need a raw Connection. Keep them separate.

### ADR-2: Verification enforcement

**Context:** Exit criteria include `rg 'SELECT|FROM|WHERE|ORDER BY'
src/mcp/server/{scry,assay}.rs` returns zero and `wc -l` under 700.
Pre-push checks (`resources/git/pre-push-checks.sh`) run fmt, clippy,
and tests — but no structural invariant checks.

**Decision:** Add a step 6/6 to `pre-push-checks.sh`:

```bash
# Step 6: MCP thin handler invariants (post mcp-thin-handlers spec)
echo "📦 [6/6] Checking MCP handler invariants..."
SQL_IN_MCP=$(rg -c 'SELECT|FROM .* WHERE|ORDER BY' src/mcp/server/scry.rs src/mcp/server/assay.rs 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
if [ "$SQL_IN_MCP" -gt 0 ]; then
    echo "   ERROR: $SQL_IN_MCP SQL statements found in MCP handlers"
    echo "   MCP handlers should delegate to src/commands/ _json() functions"
    exit 1
fi
MCP_LOC=$(wc -l < <(cat src/mcp/server/scry.rs src/mcp/server/assay.rs))
if [ "$MCP_LOC" -gt 700 ]; then
    echo "   ERROR: MCP scry+assay = $MCP_LOC lines (target: <700)"
    exit 1
fi
echo "   ✓ MCP handlers are thin ($MCP_LOC LOC, 0 SQL)"
```

This runs on every push, not just during spec execution. No manual
spot checks. No separate test file. Same enforcement path as fmt and
clippy. The check is added in the final commit of Phase 3.

The feedback loop test (scry.query → detail → use) is verified via
MCP inspector during implementation (manual, per exit criterion
`existing-tests-pass`). If fragile, add a `#[test]` in
`src/mcp/server/` that exercises the chain against a temp events.db —
but only if manual verification proves insufficient.

### ADR-3: collect_rows / serialize_result — move first

**Context:** Phase 1 `_json()` functions need `collect_rows()` and
`serialize_result()` for row failure tracking and `_warnings`
injection. These currently live in `src/mcp/server/assay.rs:22-56`.
DESIGN.md flagged this as open question #1.

**Decision: Move first, in step 1.** Before writing any `_json()`
function, move both helpers to `src/commands/assay/internal/util.rs`
(already exists — holds the `truncate()` helper). Re-export via
`internal/mod.rs`. MCP assay.rs imports from the new location.

Commit sequence becomes:
1. Move `collect_rows`/`serialize_result` to `assay/internal/util.rs`
2. Create `_json()` functions (they import from `util`)
3. Collapse MCP handler to call `_json()` functions

This avoids duplication-then-cleanup and ensures `_json()` functions
compile against the real helpers from commit 1.

### ADR-4: QueryOptions.expanded_terms rollout

**Context:** `expanded_terms` is currently MCP-only. Used in two match
arms of `handle_scry()`: find (scry.rs:268-283) and full
(scry.rs:224-238). Both do `format!("{} {}", query, terms.join(" "))`.
No CLI consumers, no plugins, no tests reference it.

**Decision:** Add `expanded_terms: Vec<String>` to
`src/retrieval/engine.rs:QueryOptions` with `#[serde(default)]`.

```rust
pub struct QueryOptions {
    pub repo: Option<String>,
    pub all_repos: bool,
    pub expanded_terms: Vec<String>,  // NEW — default empty
}
```

**Migration:**
- `QueryOptions::default()` already exists implicitly (2 fields).
  Adding a `Vec` with `Default` keeps all existing call sites
  compiling unchanged — `QueryOptions { repo: None, all_repos: false }`
  gains `expanded_terms: vec![]` via `..Default::default()` or
  explicit empty. Verify: `rg 'QueryOptions {' src/` to find all
  construction sites and add `..Default::default()` where missing.
- `engine.query_with_options()` changes: if `expanded_terms` is
  non-empty, concatenate before passing to oracles. Single change
  point — the concatenation logic moves from MCP (2 call sites) to
  engine (1 call site).
- **No CLI flag yet.** CLI has no `--expanded-terms` flag. The field
  exists for MCP and future use. CLI call sites pass empty vec via
  default. No CLI behavior changes.
- **Tests:** Existing retrieval tests pass because default is empty
  vec (no behavior change). Add one test in `engine.rs` verifying
  that non-empty expanded_terms are concatenated into the query text.
  No search relevance regression testing needed — the concatenation
  logic is identical to what MCP does today, just relocated.

### ADR-5: Shared formatting boundaries — JSON schemas

**Context:** orient/recent `_json()` functions return structured data.
Both MCP and CLI format independently. Without a defined schema, the
two sides will drift.

**Decision:** Each `_json()` function returns a typed Rust struct with
`#[derive(Serialize)]`, not raw `serde_json::Value`. The struct IS the
contract. Both consumers call `serde_json::to_string_pretty()` or
access fields directly.

**Orient schema** (extends existing `OrientResult` in subcommands.rs:20-28):

```rust
#[derive(Debug, Serialize)]
pub struct OrientEntry {
    pub path: String,
    pub composite_score: f64,
    pub importer_count: i64,
    pub activity_level: String,
    pub is_entry_point: bool,
    pub is_test_file: bool,
    pub commit_count: i64,
}

#[derive(Debug, Serialize)]
pub struct OrientResult {
    pub directory: String,
    pub entries: Vec<OrientEntry>,
}
```

CLI formats `OrientResult` as ASCII table (existing style). MCP
formats it as markdown (existing style). Both consume the same struct.
If a field is added, both consumers see it at compile time.

**Recent schema:**

```rust
#[derive(Debug, Serialize)]
pub struct RecentEntry {
    pub path: String,
    pub date: String,          // YYYY-MM-DD (extracted from timestamp)
    pub author: String,
    pub message_preview: String, // first 50 chars
}

#[derive(Debug, Serialize)]
pub struct RecentResult {
    pub query: Option<String>,
    pub days: u32,
    pub entries: Vec<RecentEntry>,
}
```

**Assay schemas** already exist as typed structs — `ModuleStats`,
`InventoryResult`, `InventorySummary` in inventory.rs:14-33. The 6
remaining query types follow the same pattern: define a result struct
with `#[derive(Serialize)]`, return it from the `_json()` function.

**Rule:** No `_json()` function returns `serde_json::Value` directly.
All return `Result<String>` after serializing a typed struct. The
struct definition is the schema. Drift requires changing the struct,
which breaks compilation for any consumer that accesses a missing field.

### ADR-6: Edge logging — testing the feedback loop

**Context:** The scry.query → scry.detail → scry.use chain shares
events.db. After collapse, both MCP and CLI route through unified
functions. The spec requires verifying this chain works. The chain also
includes `mark_edge_usage_from_query()` for graph routing feedback.

**Decision: Integration test in `src/commands/scry/internal/logging.rs`.**

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn feedback_loop_query_detail_use() {
        // 1. Create temp events.db
        // 2. Call unified log_scry_query() → returns query_id
        // 3. Call get_query_results(query_id) → verify results stored
        // 4. Call log_scry_use(query_id, doc_id, rank) → verify event written
        // 5. Verify events.db contains scry.query and scry.use events
        //    with correct query_id linkage
    }
}
```

This test covers the data chain. It does NOT test
`mark_edge_usage_from_query()` (requires a Graph instance with mother
DB) — that remains a manual MCP inspector verification. If it proves
fragile, add a second test that mocks Graph.

The test runs in `cargo test --workspace` and `pre-push-checks.sh`
step 5. No new CI configuration needed. No separate test binary.

MCP inspector exercise (exit criterion `existing-tests-pass`) is
manual during implementation: call scry find → note query_id → call
scry detail with query_id+rank → call scry use with query_id+rank →
verify response. Document the inspector sequence in the commit message
for reproducibility.

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

Revised per ADR-3 (move helpers first) and ADR-4 (expanded_terms):

1. `assay: move collect_rows/serialize_result to internal/util.rs` — ADR-3
2. `assay: extract _json() functions for 7 structural query types` — Phase 1 steps 1-2
3. `assay: unify all_repos between MCP and CLI` — Phase 1 step 3
4. `scry: extract orient_json() and recent_json()` — Phase 2 step 4
5. `scry: unify detail/use with edge-marking fix` — Phase 2 step 5
6. `scry: collapse format_detail_content/format_detail` — Phase 2 step 6
7. `scry: extract why_json(), unify query logging` — Phase 2 steps 7-8
8. `retrieval: add expanded_terms to QueryOptions` — Phase 2 step 9, ADR-4
9. `ci: add MCP thin handler invariant checks to pre-push` — ADR-2
10. `test: add feedback loop integration test` — ADR-6

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

## Resolved Questions

1. **collect_rows / serialize_result move order.**
   **Resolved: move first (ADR-3).** Commit 1 moves both to
   `assay/internal/util.rs` before any `_json()` functions are written.
   MCP assay.rs imports from new location. No temporary duplication.

2. **Where does `format_detail` live after collapse?**
   **Resolved: keep in `scry/mod.rs`.** Delete `format_detail_content`
   from MCP scry.rs. Make `format_detail` in `scry/mod.rs` `pub(crate)`
   so MCP can import it. `execute_detail()` is already in the same file.
   One function, one location.

3. **Should `emit_usage_event()` move to a shared location?**
   **Resolved: stays in `src/mcp/server/`.** It's called by MCP
   handlers (scry context, assay) for MCP-specific usage tracking. CLI
   has its own eventlog paths. Moving it gains nothing — it's already
   `pub(super)` scoped to the MCP server module.
