---
type: refactor
id: mcp-typed-handlers
status: draft
created: 2026-03-01
sessions:
  origin: 20260301-090927
related:
- mcp-server-hardening
- data-architecture-v2
beliefs:
- correctness-by-construction-not-convention
- question-mark-on-option-is-silent-swallower
exit_criteria:
- id: zero-get-chains-in-mcp-handlers
  text: zero .get("key").and_then().as_*() chains in src/mcp/server/ — all parameter access via typed struct fields
  checked: false
- id: invalid-params-fail-closed
  text: missing or wrong-type required parameters return JSON-RPC -32602 (Invalid Params) with field name — not silent empty-string fallback
  checked: false
- id: handler-signatures-typed
  text: every handler function accepts a typed args struct, not &serde_json::Value
  checked: false
- id: serde-deserialize-at-boundary
  text: serde_json::from_value() called once in dispatch, before handler — handlers never touch serde_json::Value
  checked: false
- id: existing-tests-pass
  text: all 251+ tests pass, MCP inspector exercised for scry/assay/context/spec/measure
  checked: false
---
# refactor: MCP Typed Handlers — Eliminate Value Soup at Protocol Boundary

> MCP server handlers receive `serde_json::Value` and manually extract
> parameters via 400+ `.get()/.as_*()/.unwrap_or()` chains across 2,965
> lines in 4 modules. Replace with `#[derive(Deserialize)]` structs per
> handler, deserializing once at the dispatch boundary.

## Current State

The MCP server (`src/mcp/server/`, 2,965 LOC across 5 modules) uses
`serde_json::Value` as the universal parameter type for all tool handlers.
Every parameter is extracted manually:

```rust
// Current: src/mcp/server/spec.rs — repeated 31 times
let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
let major = args.get("major").and_then(|v| v.as_bool()).unwrap_or(false);
let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
```

**Measured type soup burden by file (from structural audit 2026-03-01):**

| Module | `.get()` chains | `.as_*()` calls | `.unwrap_or()` | Total |
|--------|----------------|-----------------|----------------|-------|
| `scry.rs` (1,225 LOC) | 28 | 33 | 42 | **103** |
| `spec.rs` (446 LOC) | 31 | 28 | 22 | **81** |
| `assay.rs` (604 LOC) | 6 | 4 | 6 | **16** |
| `tools.rs` (491 LOC) | 0 | 0 | 0 | **0** |
| **Total** | **65** | **65** | **70** | **200** |

### Problems

1. **Silent defaults for required parameters.** `unwrap_or("")` means a
   missing required `id` parameter silently becomes empty string, which
   then fails deeper in the call stack with an unhelpful "spec not found"
   error instead of "missing parameter: id."

2. **No compile-time safety.** Typo in `args.get("qurey_type")` compiles
   fine, silently returns None, falls through to default. The compiler
   cannot help.

3. **Duplicated extraction logic.** The pattern
   `args.get("X").and_then(|v| v.as_str()).unwrap_or("default")` is
   copy-pasted per parameter. Each copy is a chance to get the type wrong
   (`.as_str()` vs `.as_bool()` vs `.as_i64()`).

4. **Inconsistent optionality.** Some parameters use `.unwrap_or("")`
   (silently optional), others use `args.get("X")` returning `Option`
   (explicitly optional). The intent is invisible to readers and callers.

### What mcp-server-hardening already fixed

The predecessor spec (v0.35.1) addressed a different layer:
- `.ok()` swallowing → `collect_rows()` helper with warnings
- `eprintln!` → tracing macros
- Connection reuse for patina.db
- Error code differentiation (-32001/-32002/-32603)

This spec targets the **parameter handling** layer that hardening left
untouched.

## Target State

Every MCP handler receives a typed Rust struct. `serde_json::from_value()`
happens once at the dispatch boundary. Invalid parameters fail with
`-32602` and a message naming the bad field.

```rust
// Target: typed args structs
#[derive(Deserialize)]
struct ScryArgs {
    query: Option<String>,
    mode: Option<String>,        // "find", "detail", "full", etc.
    limit: Option<usize>,
    #[serde(default = "default_true")]
    impact: bool,
    query_id: Option<String>,
    rank: Option<usize>,
    // ... all fields with correct types and serde defaults
}

// Handler signature
pub(super) fn handle(req: &Request, args: ScryArgs, conn: &Connection) -> Response {
    // Direct field access — no .get()/.as_*()/.unwrap_or()
    let mode = args.mode.as_deref().unwrap_or("find");
    let limit = args.limit.unwrap_or(10);
    // ...
}
```

```rust
// Dispatch boundary — single deserialization point
fn dispatch(method: &str, req: &Request, conn: &Connection) -> Response {
    let args = req.params.get("arguments")
        .cloned()
        .unwrap_or(serde_json::Value::Object(Default::default()));

    match tool_name {
        "scry" => {
            let typed: ScryArgs = match serde_json::from_value(args) {
                Ok(a) => a,
                Err(e) => return error_response(req.id, -32602, &format!("Invalid params: {e}")),
            };
            scry::handle(req, typed, conn)
        }
        // ...
    }
}
```

## Steps

1. **Define args structs for each handler.** Create `ScryArgs`, `AssayArgs`,
   `SpecArgs` (with per-subcommand variants), `ContextArgs`, `MeasureArgs`
   in each handler module. Derive `Deserialize`. Use `Option<T>` for
   optional fields, bare `T` with `#[serde(default)]` for fields with
   defaults. Add `#[serde(rename = "snake_case")]` where needed for JSON
   field names.

2. **Update dispatch to deserialize at boundary.** In `mod.rs`, after
   extracting the tool name, call `serde_json::from_value(args)` for the
   appropriate struct. On `Err`, return `-32602` with the serde error
   message (it names the field). On `Ok`, pass the typed struct to the
   handler.

3. **Rewrite scry.rs handlers.** Replace all 103 type soup operations.
   `ScryArgs` covers find/detail/full/orient/recent/why/use/belief modes.
   Mode-specific fields (e.g., `query_id` + `rank` for detail mode) remain
   `Option<T>` — runtime validation checks they're present for the mode
   that needs them.

4. **Rewrite spec.rs handlers.** Replace all 81 type soup operations.
   `SpecArgs` has a `subcommand` field and per-subcommand optional fields.
   Alternatively, a flat struct with all fields optional plus runtime
   validation per subcommand — simpler than an enum for JSON deserialization.

5. **Rewrite assay.rs handlers.** Replace all 16 type soup operations.
   `AssayArgs` covers inventory/imports/functions/callers/derive/search/
   cochange/belief query types.

6. **Verify under MCP Inspector.** Exercise all tool calls through
   `patina mcp` → inspector. Confirm: valid params work, missing required
   params return `-32602` with field name, wrong-type params return
   `-32602`, optional params default correctly.

## Non-Goals

- **Measure internal type soup** (`commands/measure/internal.rs`, 78
  operations). That's in the `data-measure-surface` spec's scope —
  measure's JSON structure is part of the LLM query surface redesign.
- **Plugin manifest parsing** (`plugin/internal/mod.rs`, 45+ operations).
  That uses `toml::Value`, not MCP params — different boundary.
- **Event type enum.** Tracked in data-architecture-v2 with its own trigger.
- **Error type system overhaul.** Anyhow is fine for a CLI. The MCP
  boundary already has differentiated JSON-RPC codes from hardening.

## Exit Criteria

- [ ] Zero `.get("key").and_then().as_*()` chains in `src/mcp/server/`
- [ ] Missing/wrong-type required params return `-32602` with field name
- [ ] Every handler function accepts a typed args struct, not `&serde_json::Value`
- [ ] `serde_json::from_value()` called once in dispatch, before handler
- [ ] All 251+ tests pass, MCP inspector exercised
