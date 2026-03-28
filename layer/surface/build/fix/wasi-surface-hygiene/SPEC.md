---
type: fix
id: wasi-surface-hygiene
status: ready
created: 2026-03-27
sessions:
  origin: 20260327-104954-066673000
related:
- wit/toys/deps/
- wit/knowledge-child/knowledge-child.wit
- wit/pipeline/pipeline.wit
- sdk/patina-sdk/
- src/child/internal/knowledge_child.rs
beliefs:
- '[[wasi-is-foundation-not-option]]'
- '[[observation-at-the-boundary]]'
exit_criteria:
- id: wsh1-dead-wit-removed
  text: 'Legacy WIT files that are no longer imported by any world are deleted: `patina-log.wit`, `patina-state.wit`, `patina-store.wit`, `patina-events.wit`.'
  checked: true
- id: wsh2-connect-stripped
  text: '`patina:connect` stripped to service binding only: `resolve(name) -> binding`. No `request`, `header`, `response`, `base-url`. HTTP mechanics removed.'
  checked: true
- id: wsh3-wasi-http-linked
  text: '`wasi:http/outgoing-handler` linked in Mother''s child runtime. Children can make standard HTTP calls.'
  checked: true
- id: wsh4-connect-binding-wired
  text: Mother intercepts `wasi:http` calls and injects credentials when a `patina:connect` binding is active for the target host.
  checked: true
- id: wsh5-existing-children-migrated
  text: All children using `patina:connect.request()` migrated to `wasi:http` + `patina:connect.resolve()` binding pattern.
  checked: true
- id: wsh6-workspace-clean
  text: '`cargo check --workspace -q`, `cargo test -q --workspace`, `wasm-tools component wit` all pass.'
  checked: true
- id: wsh7-no-ghost-imports
  text: Every import in `knowledge-child.wit` and `pipeline.wit` has a working host implementation in the linker.
  checked: true
---
# fix: wasi-surface-hygiene

## Problem

The wasi-toy-alignment spec migrated imports to WASI standards but left residual mess:

1. **4 dead WIT files** — `patina-log.wit`, `patina-state.wit`, `patina-store.wit`, `patina-events.wit` are no longer imported by any world but still exist in `wit/toys/deps/`. They confuse the codebase and could be accidentally re-imported.

2. **`patina:connect` still does HTTP** — `request()`, `header`, `response`, `base-url()` are HTTP mechanics that belong in `wasi:http`. Connect should be service binding only: `resolve(name) -> binding`.

3. **`wasi:http` is a ghost import** — declared in `knowledge-child.wit` (line 42) but NOT linked in Mother's runtime. Children see it in the WIT contract but can't use it. This violates the principle: if it's imported, it must work.

4. **`wasi:filesystem` is technically linked but policy-blocked** — `wasmtime_wasi::p2::add_to_linker_sync()` links filesystem types into the runtime, but children have no configured filesystem preopens, so FS operations would fail at runtime. This is a policy decision, not a linker gap. If children don't get FS access by design, remove the import from the world definition to make the policy explicit in the contract rather than relying on runtime failure.

## Goal

Clean the WASI surface so every import is real, every WIT file is active, and `patina:connect` does only what WASI can't.

## Non-Goals

- Adding new toy functionality.
- Redesigning the connect binding mechanism beyond what's needed to separate it from HTTP.

**Note on behavior change:** This IS an API migration. Children currently call `connect.request()` for authenticated HTTP. After this spec, they call `connect.resolve()` + `wasi:http`. The external behavior (authenticated HTTP calls succeed) is preserved, but the SDK API surface changes. This is not "no behavior change" — it's "same capability, different interface."

## Audit: current state of `wit/toys/deps/`

### Dead files (no world imports them)

| File | Replaced by | Action |
|---|---|---|
| `patina-log.wit` | `logging.wit` (wasi:logging) | Delete |
| `patina-state.wit` | `keyvalue.wit` (wasi:keyvalue) | Delete |
| `patina-store.wit` | `sql.wit` (wasi:sql) | Delete |
| `patina-events.wit` | `messaging.wit` + `patina-events-stream.wit` | Delete |

### Live Patina deltas (correctly custom)

| File | Reason it's custom |
|---|---|
| `patina-connect.wit` | Service binding authority model — but needs HTTP stripped |
| `patina-events-stream.wit` | Offset cursoring + ack for checkpoint recovery (hard rule 5) |
| `patina-measure.wit` | Domain metric emission — no WASI standard exists |
| `patina-task.wit` | Task enqueuing with dedup — no WASI analog |
| `patina-peer.wit` | Child-to-child calls — no WASI analog |
| `patina-git.wit` | Git operations — no WASI analog |

### Imports needing attention

| Import | In world | Runtime state | Action |
|---|---|---|---|
| `wasi:http/outgoing-handler@0.2.6` | knowledge-child.wit:42 | Declared/imported but no outgoing-handler host adapter wired — `p2::add_to_linker_sync` does NOT cover HTTP | Wire explicit HTTP host support via `wasmtime-wasi-http` crate |
| `wasi:filesystem/types@0.2.6` | knowledge-child.wit:43 | Linked via p2 but no preopens configured — fails at runtime | Remove import from world (make no-FS policy explicit in contract, not runtime failure) |

## Connect/HTTP split

### Before (current)

```wit
package patina:connect@0.1.0;
interface connect {
    use wasi:http/types@0.2.6.{field-key, field-value, status-code};
    record header { name: field-key, value: field-value }
    record response { status: status-code, headers: list<header>, body: list<u8> }
    resource connection;
    resolve: func(name: string) -> result<connection, string>;
    base-url: func(conn: borrow<connection>) -> string;
    request: func(conn: borrow<connection>, method: string, path: string,
        headers: list<header>, body: option<list<u8>>) -> result<response, string>;
}
```

Children call `connect.resolve("github")` then `connect.request(conn, ...)` — all HTTP goes through the Patina toy. `wasi:http` sits unused.

### After (target)

```wit
package patina:connect@0.2.0;
interface connect {
    resource binding;
    resolve: func(name: string) -> result<binding, string>;
}
```

Children call `connect.resolve("github")` to get a binding. Then use standard `wasi:http/outgoing-handler` for HTTP calls. Mother's `wasi:http` host implementation checks active bindings and injects credentials (auth headers, base-url rewriting) transparently.

Children that don't need authenticated services use `wasi:http` directly — no binding needed.

### Binding semantics

- `resolve(name)` returns a `binding` resource scoped to the named service. The binding carries credential, domain allowlist, and injection config as one indivisible grant (this preserves the `[[connector-toy-is-indivisible-authority]]` security invariant).
- **Activation is implicit by request target.** When a child makes a `wasi:http` request, Mother matches the request URL against active bindings by domain. If a binding matches, credentials are injected. If no binding matches, the request proceeds unauthenticated. This is the default — children can make public HTTP calls without any binding. Grant-level restrictions (e.g., "this child may not make any HTTP calls") are a separate manifest concern enforced before the request reaches the binding check.
- **Multiple bindings coexist.** A child can `resolve("github")` and `resolve("slack")` — both bindings are active simultaneously. Mother matches by domain on each HTTP request.
- **Binding lifetime = resource lifetime.** When the binding resource is dropped, Mother stops injecting for that service. No explicit activate/deactivate API.
- **Ambiguity rule:** If two bindings match the same domain, that's a manifest error. Mother rejects at `resolve()` time if it would create a domain collision.
- **Domain normalization:** Domains are lowercased, trailing dots stripped, and matching is exact-host only (no subdomain wildcards). `api.github.com` matches `api.github.com`, not `*.github.com`. This keeps collision and injection checks deterministic.
- **Redirect authority:** Binding match is evaluated against the **original request URL only**, not redirect targets. If a request to `api.github.com` redirects to `cdn.github.com`, the binding for `api.github.com` applies to the initial request but Mother does NOT inject credentials into the redirect. This prevents credential leakage via open redirects.

### HTTP host implementation path

`wasi:http/outgoing-handler` requires the `wasmtime-wasi-http` crate (not just `wasmtime-wasi`). Implementation:

1. Add `wasmtime-wasi-http` dependency to `Cargo.toml`.
2. Implement the HTTP host view trait on `HostState` — this is where binding-aware credential injection lives. Verify exact trait and function names against `wasmtime-wasi-http` v41 docs before implementation; API names here are tentative.
3. Add the HTTP-specific linker function to `build_linker()`. Verify exact symbol name against crate docs.
4. The `WasiHttpView::send_request()` override checks active `patina:connect` bindings against the outgoing request URL and injects headers before delegating to the actual HTTP client.

## Approach

1. **Delete dead WIT files** — remove 4 legacy files from ALL deps lanes: `wit/toys/deps/`, `wit/knowledge-child/deps/`, `wit/pipeline/deps/` (has `patina-log.wit`), `sdk/patina-sdk/wit/knowledge-child/deps/`, `sdk/patina-sdk/wit/pipeline/deps/`.
2. **Remove `wasi:filesystem` import from knowledge-child lanes only** — delete from `knowledge-child.wit` and `knowledge-child/deps/`. Note: `toybox.wit` still imports `wasi:filesystem/types` as part of the full capability graph — leave it there.
3. **Link `wasi:http`** — add `wasmtime-wasi-http` crate dependency. Implement `WasiHttpView` trait on `HostState` with binding-aware credential injection. Add `wasmtime_wasi_http::add_only_http_to_linker_sync()` to `build_linker()`.
4. **Strip `patina:connect`** — remove `request`, `header`, `response`, `base-url`. Rename `connection` resource to `binding`. Bump to `@0.2.0`.
5. **Wire binding-aware HTTP** — Mother's `wasi:http` host checks if a `patina:connect` binding is active for the target host and injects credentials.
6. **Migrate children** — update any child using `connect.request()` to use `wasi:http` + `connect.resolve()`.
7. **Verify** — full workspace check, WIT validation, integration tests.

## Verification

### Structural checks

```bash
wasm-tools component wit wit/toys
wasm-tools component wit wit/knowledge-child
wasm-tools component wit wit/pipeline
cargo check --workspace -q
cargo test -q --workspace
```

### Dead WIT check (all lanes)

```bash
# Must all return 0. Checks every deps lane, not just wit/toys/deps.
for dir in wit/toys/deps wit/knowledge-child/deps wit/pipeline/deps sdk/patina-sdk/wit/knowledge-child/deps sdk/patina-sdk/wit/pipeline/deps; do
  test $(ls $dir/patina-log.wit $dir/patina-state.wit $dir/patina-store.wit $dir/patina-events.wit 2>/dev/null | wc -l) -eq 0
done
```

### Binding behavior tests

```bash
# These are integration tests, not unit tests. They require a WASM child artifact.
cargo test --test wasi_http_binding -q
```

Required test cases:
1. **No binding, public URL** — child makes `wasi:http` request to public endpoint. Succeeds without credentials.
2. **Bound host, injection** — child resolves `connect("github")` then makes `wasi:http` request to `api.github.com`. Mother injects auth header. Succeeds with authenticated response.
3. **Wrong host, no injection** — child resolves `connect("github")` then makes `wasi:http` request to `api.slack.com`. No injection (binding doesn't match). Request proceeds unauthenticated.
4. **Domain collision rejection** — child resolves two bindings that overlap on domain. Mother returns error on second `resolve()`.
5. **Binding drop stops injection** — child resolves binding, drops it, then makes request to same host. No injection.

## Build Readiness

Ready to start. Scoped tightly — delete dead files, strip connect, link wasi:http, migrate callers.
