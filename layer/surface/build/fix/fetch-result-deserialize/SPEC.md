---
type: fix
id: fetch-result-deserialize
status: draft
created: 2026-03-09
related:
- pipe-contract-safety
exit_criteria:
- id: from-value-deserialize
  text: 'NativeChild::fetch() deserializes the pipe/fetch response result via serde_json::from_value::<FetchResult>() instead of manual .get()/.as_u64() field plucking.'
  checked: false
  verify: 'lifecycle.rs contains `serde_json::from_value` for FetchResult. No manual .get("emitted") or .get("cursor") calls remain in the fetch response path.'
beliefs:
- cross-crate-json-contracts-need-shared-types
---
# fix: FetchResult response parsing should use serde_json::from_value()

> broker::lifecycle::NativeChild::fetch() manually plucks fields from pipe/fetch response JSON instead of deserializing via shared FetchResult type — silently defaults missing emitted to 0

## Problem

[[spec-pipe-contract-safety]] fixed the request side: broker now constructs
`FetchParams` and `InitializeParams` from shared types and serializes via
`serde_json::to_value()`. But the response side was left asymmetric.

In `src/broker/lifecycle.rs:144-155`, `NativeChild::fetch()` reads the
`pipe/fetch` response by manually plucking JSON fields:

```rust
let emitted = result.get("emitted").and_then(|e| e.as_u64()).unwrap_or(0);
let cursor = result.get("cursor").and_then(|c| c.as_str()).map(|s| s.to_string());
Ok(FetchResult { emitted, cursor })
```

This silently defaults `emitted` to 0 if the field is missing or
malformed, rather than failing with a deserialization error. The struct
literal does catch missing fields at compile time (adding a field to
`FetchResult` causes a compile error here), but the runtime behavior
is lenient where it should be strict.

## Root Cause

The response parsing predates [[spec-pipe-contract-safety]]. When the
broker had its own `FetchResult` struct, manual parsing was the only
option. Now that both sides share `patina_pipe_types::FetchResult`
(with `Deserialize` derived), `serde_json::from_value()` is available
but wasn't applied during the spec — it was out of scope (the spec
targeted request-side drift, not response parsing).

## Fix

Replace the 8 lines of manual field extraction with:

```rust
let result = response
    .get("result")
    .with_context(|| "pipe/fetch response missing result")?;
let fetch_result: FetchResult = serde_json::from_value(result.clone())
    .with_context(|| "pipe/fetch response: invalid FetchResult")?;
Ok(fetch_result)
```

One file, one commit. Wire format unchanged — only how the response
is parsed.

## Non-Goals

- Changing `FetchResult` fields or serde attributes
- Changing the JSON-RPC envelope parsing (the `.get("result")` extraction stays)
- Touching request-side code (already fixed by [[spec-pipe-contract-safety]])

## Exit Criteria

See frontmatter.
