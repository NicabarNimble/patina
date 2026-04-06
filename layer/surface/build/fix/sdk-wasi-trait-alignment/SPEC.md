---
type: fix
id: sdk-wasi-trait-alignment
status: draft
created: 2026-04-06
sessions:
  origin: 20260405-133644-511306000
beliefs:
  - "[[wasi-is-foundation-not-option]]"
  - "[[pandos-are-products-children-are-compute]]"
related:
  - sdk/patina-sdk/src/toys.rs
  - sdk/patina-sdk/src/child.rs
  - wit/toys/deps/
  - wit/toys/toybox.wit
blocks:
  - pando-platform
exit_criteria:

  - id: swa1-log-matches-wasi
    text: "LogBackend trait matches `wasi:logging@0.1.0` shape: single `log(level, context, message)` function with level enum (trace, debug, info, warn, error, critical). Convenience methods (`.info()`, `.error()`) are sugar on top, not the trait contract."
    checked: false

  - id: swa2-keyvalue-matches-wasi
    text: "StateBackend trait matches `wasi:keyvalue/store@0.2.0` shape: bucket resource with `open(identifier)`, `get(key) -> list<u8>`, `set(key, list<u8>)`, `delete(key)`, `exists(key)`, `list-keys(cursor)`. Values are bytes not strings. Bucket identifier scopes access."
    checked: false

  - id: swa3-filesystem-matches-wasi
    text: "LayerFsBackend trait matches `wasi:filesystem@0.2.6` shape: descriptor-based access with preopened directories. Not simplified string path functions."
    checked: false

  - id: swa4-messaging-matches-wasi
    text: "Event publishing matches `wasi:messaging/producer@0.2.0` shape: client resource with `connect(name)`, `send(client, message)`. Message has topic, content-type, data (bytes), metadata."
    checked: false

  - id: swa5-http-matches-wasi
    text: "FetchBackend trait matches `wasi:http/outgoing-handler@0.2.6` shape. Not simplified `get(url)/post(url, body)` string functions."
    checked: false

  - id: swa6-patina-delta-documented
    text: "Every `patina:*` toy (`git`, `events-stream`, `measure`, `connect`, `task`, `peer`) has a comment in its WIT file stating: (a) why WASI doesn't cover this, (b) whether a WASI proposal exists that overlaps, (c) if so, how our interface mirrors the proposal shape."
    checked: false

  - id: swa7-children-recompile
    text: "All existing children (6 core + spec-manager stub + grammar plugins) compile against the aligned traits. No child uses the old simplified shapes."
    checked: false

  - id: swa8-capability-enforcement
    text: "A child with `toys = [\"log\"]` cannot call keyvalue, filesystem, or git host functions. Test proves enforcement."
    checked: false

  - id: swa9-compile-proof
    text: "`cargo check --workspace -q` and `cargo test -q --lib` pass."
    checked: false
---
# fix: SDK WASI Trait Alignment

## Problem

The SDK's Rust traits in `toys.rs` don't match the WASI interface shapes
they claim to wrap. Children code against simplified Patina abstractions,
not actual WASI contracts. This means:

- A child built on Patina's `StateBackend` won't run on another WASI host
- We can't honestly claim WASI alignment
- The pando platform builds on a foundation that misrepresents its interface
  contracts

## Divergence Inventory

### `wasi:logging@0.1.0`

WIT:
```
log: func(level: level, context: string, message: string)
level = { trace, debug, info, warn, error, critical }
```

SDK trait:
```rust
trait LogBackend {
    fn debug(message: &str);
    fn info(message: &str);
    fn warn(message: &str);
    fn error(message: &str);
}
```

Gap: no `context` parameter, no `trace`/`critical` levels, split into
separate functions instead of one `log()` with level enum.

### `wasi:keyvalue/store@0.2.0`

WIT:
```
resource bucket {
    get: func(key: string) -> result<option<list<u8>>, error>
    set: func(key: string, value: list<u8>) -> result<_, error>
    delete: func(key: string) -> result<_, error>
    exists: func(key: string) -> result<bool, error>
    list-keys: func(cursor: option<string>) -> result<key-response, error>
}
open: func(identifier: string) -> result<bucket, error>
```

SDK trait:
```rust
trait StateBackend {
    fn get(key: &str) -> Option<String>;
    fn put(key: &str, value_json: &str) -> Result<(), String>;
    fn delete(key: &str) -> Result<(), String>;
    fn list_prefix(prefix: &str) -> Vec<String>;
}
```

Gaps: no bucket resource (no scoped access), values are strings not bytes,
no `exists()`, `list_prefix` vs cursor-based `list-keys`, no error type
on get.

### `wasi:filesystem@0.2.6`

WIT: descriptor-based API with preopened directories, streams, directory
entries, file metadata, permissions.

SDK trait:
```rust
trait LayerFsBackend {
    fn read_file(path: &str) -> Result<String, String>;
    fn write_file(path: &str, contents: &str) -> Result<(), String>;
    fn list_dir(path: &str) -> Result<Vec<String>, String>;
    fn delete_file(path: &str) -> Result<(), String>;
    fn move_path(from: &str, to: &str) -> Result<(), String>;
    fn exists(path: &str) -> Result<bool, String>;
}
```

Gaps: no descriptors, no preopened directories, no streams, string content
not bytes, flat string paths not scoped to preopens.

### `wasi:messaging/producer@0.2.0`

WIT:
```
resource client
connect: func(name: string) -> result<client, string>
send: func(client: borrow<client>, message: message) -> result<u64, string>
message = { topic, content-type, data: list<u8>, metadata }
```

SDK: no direct messaging trait — event publishing goes through Mother's
event bus via custom `EmitBackend` and `EventBackend` traits.

### `wasi:http/outgoing-handler@0.2.6`

WIT: full HTTP with method variants, headers, trailers, streams, status
codes, TLS config.

SDK trait:
```rust
trait FetchBackend {
    fn get(url: &str) -> Result<String, String>;
    fn post(url: &str, body: &str, content_type: &str) -> Result<String, String>;
}
```

Gaps: only GET/POST, no headers, no status codes, string body not streams,
no method variants.

## Patina Delta Toys (correctly custom)

These have no WASI equivalent and are correctly Patina-specific:

- `patina:git@0.1.0` — version control ops (no WASI proposal exists)
- `patina:events-stream@0.1.0` — event consumption (WASI messaging only
  covers producing; consumption is Patina's delta)
- `patina:measure@0.1.0` — structured metrics (no WASI proposal exists)
- `patina:task@0.1.0` — task queue (no WASI proposal exists)
- `patina:connect@0.2.0` — authenticated connectors (extends wasi:http
  with credential injection)
- `patina:peer@0.1.0` — P2P events (no WASI proposal exists)

## Root Cause

SDK traits were designed for developer ergonomics, not WASI conformance.
The WIT files reference WASI packages, `wit_bindgen` generates bindings
from them, but the hand-written trait layer in `toys.rs` simplifies the
shapes. Children code against the simplified traits, not the generated
bindings.

## Fix

For each WASI toy: align the SDK trait to match the WIT interface shape.
Children code against WASI shapes. Convenience helpers can exist as
extension methods but the trait contract matches WASI.

For each Patina toy: document in the WIT file why it exists, whether a
WASI proposal overlaps, and how we mirror the proposal if one exists.

## Implementation Order

1. **swa1** — Log: add level enum + context parameter. Keep `.info()` etc
   as convenience sugar on top of the real `log()` function.
2. **swa2** — Keyvalue: add bucket resource, bytes values, cursor-based
   list. Biggest change — every child using state needs updating.
3. **swa3** — Filesystem: add descriptor model with preopened dirs.
4. **swa4** — Messaging: align event publishing with wasi:messaging
   producer shape.
5. **swa5** — HTTP: align with outgoing-handler shape.
6. **swa6** — Document Patina delta toys in WIT files.
7. **swa7** — Recompile all children against aligned traits.
8. **swa8** — Prove capability enforcement.
9. **swa9** — Compile/test proof.

## Verification

```bash
cargo check --workspace -q
cargo test -q --lib
cargo check -q -p patina-sdk --features child
```
