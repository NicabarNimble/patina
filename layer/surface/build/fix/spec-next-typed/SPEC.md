---
type: fix
id: spec-next-typed
status: ready
created: 2026-02-25
sessions:
  origin: 20260225-104204
exit_criteria: []
---
# fix: Type next_spec_value return to Vec<Recommendation>

> next_spec_value is the only _value() function returning untyped serde_json::Value — breaks type safety contract

## Problem

`next_spec_value()` in `queue.rs:71` returns `Result<serde_json::Value>` — it is the **only** `_value()` function in the entire spec module that returns untyped JSON. Every other query and mutation function returns a typed struct:

- `promote_spec_value()` → `MutationResult`
- `check_spec_value()` → `CheckResult`
- `show_spec_value()` → `ShowResult`
- `create_spec_value()` → `CreateResult`
- `split_spec_value()` → `SplitResult`

The human-readable path in `next_spec()` then indexes into this `Value` with string keys (`top["id"].as_str().unwrap_or("")`), which is fragile — a typo or schema change silently returns empty strings instead of failing at compile time.

**Flagged by:** Jon Gjengset (type safety — the compiler should catch schema mismatches, not runtime), Rich Sutton (inconsistency — one function breaking the pattern the other 10 follow).

## Root Cause

`Recommendation` is defined as a local struct **inside** `next_spec_value()` and serialized to `serde_json::Value` before return. The struct already exists — it just isn't exposed.

```rust
// queue.rs:84-91 — this struct is local to the function
#[derive(Debug, Serialize)]
struct Recommendation {
    id: String,
    status: String,
    reason: String,
    priority: u32,
    impact: usize,
}
```

## Fix

1. Move `Recommendation` to module scope with `pub(crate)` visibility
2. Change `next_spec_value()` signature to `Result<Vec<Recommendation>>`
3. Remove the `serde_json::to_value()` conversion at the end
4. Update `next_spec()` display function to use typed fields directly
5. Update MCP handler to serialize `Vec<Recommendation>` (already implements `Serialize`)

~15 lines changed, zero behavioral change.

## Key Files

```
src/commands/spec/internal/queue.rs  — move struct, change return type
src/commands/spec/mod.rs             — update re-export if needed
src/mcp/server.rs                    — update spec.next handler (already serializes)
```

## Exit Criteria

- [ ] `next_spec_value()` returns `Result<Vec<Recommendation>>`
- [ ] `Recommendation` is `pub(crate)` and re-exported from `spec/mod.rs`
- [ ] No `serde_json::Value` remains in queue.rs
- [ ] `next_spec()` display uses typed field access (no string indexing)
