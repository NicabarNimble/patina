# Design: Collapse toys to primitives and align with WASI/Cloudflare binding model

## Why This Design

Patina independently arrived at capability-based security — the same architecture behind Cloudflare Workers, Deno permissions, WASI, and browser sandboxes. But we baked domain logic into our toy interfaces (github API shapes, DuckDB operations, session lifecycle), creating 22 toys where 10 primitives (in 3 layers) suffice. We were reinventing wheels — writing custom HTTP, filesystem, logging, and key-value interfaces when WASI already defines these as standard. And we were encoding domain knowledge into host infrastructure where it belongs in children and SDK libraries.

The collapse has three strategic goals:

1. **Embrace WASI as WASI is.** Adopt existing WASI interfaces where they fit — don't reimplement `http` or `filesystem` when the ecosystem already has them. Where WASI interfaces aren't yet stable (`keyvalue`, `logging`), build Patina shims that track WASI shapes so migration is mechanical when stability arrives.

2. **Expand where Patina's system needs it.** `patina:connect` (credential-safe connection resolution), `patina:store` (structured data with host-routed backends), `patina:events` (pub/sub with offset tracking), `patina:task` (deferred work scheduling), `patina:peer` (child-to-child communication), `patina:git` (version control operations) — these exist because real-world data-mover children need capabilities WASI doesn't offer. They're honest extensions born from implementation experience, not speculation. If they prove valuable, they're natural candidates for WASI proposals.

3. **Align with Cloudflare's proven design model.** Cloudflare Workers validates the binding/capability-grant architecture at scale: ~8 primitive binding types serve millions of Workers across every domain. Their bindings = our toybox. Their `wrangler.toml` = our `child.toml`. Domain specificity belongs in the Worker (child), not the binding (toy). Aligning with their model keeps our toy surface honest about what's actually a primitive.

This alignment also enables — but does not require — building children that run on both Mother and Cloudflare Workers. Most children will be Patina-native, using Patina-specific toys like `store`, `events`, and `git`. But children built against only the portable subset (`http`, `log`, `state`, `fs`) can run under any WASI-compatible runtime. The `cloudflare-worker-child` spec will prove this capability exists; it's not a universal requirement for all children.

## Discovery Chain

This design was not planned top-down. It emerged from a session that started with reviewing builder agent progress on plugin vocabulary retirement and ended here through a series of linked discoveries:

**Layout review → crate boundaries → what is core?** Auditing the post-greenfield workspace layout forced the question of what a greenfield crate structure would look like. This surfaced that `src/` is a monolith, which led to asking what `patina-core` should actually contain.

**What is core? → The belief system is the product.** Core verbs (scrape, scry, assay, oxidize, context) all serve the belief layer. Mother/children/toys are the extension surface. Children are data movers, not general-purpose compute.

**Children are data movers → toys should be data primitives.** If children exist to move data through the platform, the toy set should be small and data-oriented: reach data sources, transform data, route it.

**"A github toy is just http with different creds"** — the collapse insight. Most of the 22 toys are the same primitive capability with different domain knowledge. That domain knowledge belongs in children and SDK helpers.

**Trying to add "scope" broke the vocabulary** — scope was carrying capability + credential + target as one concept. Separating them produced the **connection** model: named bindings that Mother resolves, like Cloudflare Workers' `wrangler.toml`.

**Comparing with Cloudflare Workers** — almost 1:1 mapping validated the entire architecture. Their bindings = our toybox. Their wrangler.toml = our child.toml. Their ~8 binding types = our collapsed 10 toys (4 WASI-aligned: 2 adopted + 2 shimmed, plus 1 bridge + 5 Patina-specific).

**Comparing with WASI** — 4 of 10 collapsed toys align to WASI interfaces, with 2 adopted now (`http`, `fs`) and 2 shimmed with sunset (`log`, `state`). Our 5 Patina-specific toys plus `patina:connect` fill real ecosystem gaps. Alignment is free if we design cleanly.

The long road through 5 greenfield specs, vocabulary retirement, layout consolidation, and SDK toybox definition was necessary to see the shape clearly. Each step removed noise. This spec is what the signal looks like.

## Build Target

Phase 0 + 8 execution phases. Phase 0 (protocol lock + feasibility) is the critical decision point — Phase 1 WIT design starts only after Phase 0 is frozen.

### Phase 0 lock status (2026-03-26)

Phase 0 is now locked with these non-negotiable constraints:
- Credential path is explicit: `connect::request(...)` handles credential-aware HTTP.
- Raw `wasi:http` is opt-in (`http.raw = true`) and never gets credential injection.
- Store routing is host infrastructure resolved from connection metadata.
- `[needs.connections]` and `[needs.scopes]` are additive and merge into one grant object.
- URL-prefix matching is not a credential strategy.

Feasibility snapshot (command-backed):
- `cargo check --workspace -q` passes.
- `cargo tree -p patina-ai --depth 1` confirms Wasmtime 41 + WASIp2 baseline.
- `cargo test -q -p patina-ai --lib` currently has one known failing baseline test (`session::tests::test_find_project_root_not_in_project`), unrelated to toy-collapse design.

WASI adoption lock from Phase 0:
- Adopt now: `wasi:http`, `wasi:filesystem`.
- Stage behind Patina shims until proven in runtime workflow: `wasi:keyvalue` (`patina:state` initially), `wasi:logging` (`patina:log` initially).

## Vocabulary

| Term | Definition |
|------|-----------|
| **Toy** | A primitive capability Mother grants. A door in the WASM sandbox wall. 10 in this spec: 2 WASI adopted now (`http`, `fs`), 2 WASI-aligned Patina shims with sunset (`log`, `state` — migrate to `wasi:*` when stable), 1 Patina bridge (`connect`), 5 Patina-specific (`store`, `events`, `task`, `peer`, `git`). Defined as WIT interfaces. |
| **Toybox** | The sealed capability payload Mother assembles for a child at init — the architectural centerpiece. Not just a list of toy names: resolved connection handles with credentials, endpoints, scopes, rate limits, and policy attached. Mother builds it from `child.toml`, the child receives opaque handles, and the credentials live exclusively in Mother's secret store. The toybox IS the security contract. |
| **Kind** | The child's runtime lifecycle shape: knowledge-child, command, pipeline, task. Determines how Mother manages the child. Not related to toys. |
| **Connection** | A named binding in `child.toml`. The child says a name ("github"), Mother resolves it to toy + credential + endpoint + config. Like a Cloudflare Workers binding in `wrangler.toml`. |
| **World** | WIT implementation detail only. The composed set of imports/exports for wit-bindgen. Derived from kind + toybox. Child authors never use this word. |

## The 10 Toys (3 layers)

### Layer 1: WASI-Aligned (embrace WASI as WASI is)

**Adopt now** (stable, proven in Wasmtime):

**http** — `wasi:http/outgoing-handler`. Standard outbound HTTP. The component constructs requests with scheme, authority, path. The host executes. No connection concept at this layer — that's Layer 2's job.

**fs** — `wasi:filesystem`. File access. Mother scopes paths — a child can only access directories it's granted.

**Shim with sunset** (WASI interface exists but not yet stable in our runtime workflow — start as Patina shims that track the WASI shape, migrate mechanically when WASI reaches Phase 4 standardized + Wasmtime ships stable support):

**log** — `patina:log` initially, tracking `wasi:logging` shape. Structured logging output.

**state** — `patina:state` initially, tracking `wasi:keyvalue` shape. Key-value persistence. Child's working memory.

All four are designed for portability. Any WASI runtime can eventually satisfy them. The shims exist because we embrace WASI as-is rather than force-adopting interfaces that aren't ready.

### Layer 2: Patina Bridge (`patina:connect`)

This is the key innovation — the layer that makes WASI toys safe for a multi-tenant child platform.

```wit
// patina:connect — named connection resolver with opaque credential handles
interface connect {
    resource connection;
    resolve: func(name: string) -> result<connection, string>;
    base-url: func(conn: borrow<connection>) -> string;
    request: func(
        conn: borrow<connection>,
        method: string,
        path: string,
        headers: list<tuple<string, string>>,
        body: option<list<u8>>,
    ) -> result<http-response, string>;
}
```

The `connection` resource is an opaque host-owned handle. The child holds a reference but cannot inspect its internals. `connect::request(...)` is the credential-aware path: Mother resolves endpoint + policy + secret from the handle, injects credentials host-side, and executes transport through its `wasi:http` host implementation.

**This is stronger than Cloudflare's model.** Cloudflare Workers see secret values as strings (`env.GITHUB_TOKEN`). A Worker constructs its own auth headers. A malicious Worker can exfiltrate the token.

In Patina, the credential never enters WASM memory:
1. Child calls `connect::resolve("github")` → gets opaque handle + base URL (not secret).
2. Child calls `connect::request(...)` with the handle — no auth headers in the request.
3. Mother's host sees the handle, injects `Authorization: Bearer <pat>` host-side.
4. Mother makes the HTTP call, returns response to child.
5. The PAT never crossed the WASM wall.

A compromised child can USE the connection (within granted scope) but cannot STEAL the credential.

Raw `wasi:http` remains available only as an explicit opt-in capability (`http.raw = true`) and never participates in credential injection. URL-prefix matching is not an allowed credential strategy.

**This shapes the toybox.** The toybox isn't just a list of toy names — it's a resolved set of connection handles with credentials, endpoints, and policy attached. Mother builds it at init from `child.toml`. The child receives opaque handles. The credentials live exclusively in Mother's secret store.

**fs** — File access within granted paths. Mirrors `wasi:filesystem`.

```wit
interface fs {
    read: func(path: string) -> result<string, string>;
    write: func(path: string, contents: string) -> result<_, string>;
    list: func(path: string) -> result<list<string>, string>;
    delete: func(path: string) -> result<_, string>;
    exists: func(path: string) -> result<bool, string>;
}
```

Mother scopes paths — a child can only access directories it's granted. No connection parameter needed; path scoping is Mother-side policy.

**log** — Structured logging. Mirrors `wasi:logging`.

```wit
interface log {
    enum log-level { debug, info, warn, error }
    log: func(level: log-level, message: string);
}
```

Unchanged from current. Already a clean primitive.

**state** — Key-value persistence. Mirrors `wasi:keyvalue`.

```wit
interface state {
    get: func(key: string) -> option<string>;
    set: func(key: string, value: string) -> result<_, string>;
    delete: func(key: string) -> result<_, string>;
    list: func(prefix: string) -> list<string>;
}
```

Child's own working memory. Absorbs `checkpoint` (which was just state with a stream-scoped key — the child can scope its own keys).

### Patina-Built (designed to be WASI-proposable)

**store** — Structured data query/mutate. Candidate for future `wasi:store`.

```wit
interface store {
    query: func(connection: borrow<connect.connection>, query: string) -> result<string, string>;
    mutate: func(connection: borrow<connect.connection>, action: string, payload: string) -> result<string, string>;
}
```

Absorbs: `lake`, `belief`, `graph`, `query`. The connection handle distinguishes targets ("ducklake", "beliefs", "graph"). Mother resolves each handle to the backing store engine and policy. Backend routing is host infrastructure, not child payload convention.

Design note: `query` and `payload` are strings (JSON). This keeps the WIT interface domain-agnostic. The child and SDK helpers handle serialization.

**events** — Pub/sub with offset tracking. Candidate for future `wasi:events`.

```wit
interface events {
    record event {
        stream: string,
        offset: u64,
        event-type: string,
        payload: string,
        occurred-at: string,
    }
    publish: func(stream: string, event-type: string, payload: string) -> result<u64, string>;
    subscribe: func(stream: string, after: option<u64>, limit: u32) -> result<list<event>, string>;
    ack: func(stream: string, offset: u64) -> result<_, string>;
}
```

Absorbs: `emit`, `measure`, `peer` (event emission). Measure becomes `events.publish("measure", "health-check", metrics_json)`. Emit becomes `events.publish("facts", "github-issues", data_json)`. The event type and stream name carry the domain semantics, not the toy interface.

**task** — Deferred work scheduling. Candidate for future `wasi:task`.

```wit
interface task {
    enqueue: func(kind: string, payload: string, dedupe-key: option<string>) -> result<string, string>;
}
```

Unchanged from current. Already a clean primitive. The `kind` parameter is a string the child defines — Mother routes it.

**peer** — Child-to-child communication via Mother. Like Cloudflare Service Bindings.

```wit
interface peer {
    call: func(child: string, action: string, payload: string) -> result<string, string>;
}
```

Mother routes the call. The caller cannot reach the target child's toybox — only its exported `handle()` API. This preserves capability boundaries, exactly like Cloudflare Service Bindings.

## child.toml Target Shape

```toml
name = "ducklake"
kind = "knowledge-child"
version = "0.2.0"

[needs]
toys = ["http", "store", "events", "log"]

[needs.connections]
github = { toy = "http" }
gitlab = { toy = "http" }
ducklake = { toy = "store" }

[needs.scopes.task]
intents = ["fetch-source"]

[provides]
child = "ducklake"

[relationships]
emits = ["data-ingested"]
listens = ["sync-requested"]
```

Read this file and you know everything the child can do. The manifest is the security boundary.

## Mother's Toybox Assembly

```
1. Read child.toml
2. Validate: are all requested toys allowed for this child?
3. Resolve connections:
   github → { toy: http, base_url: "https://api.github.com", auth: secrets/github-pat, rate_limit: 5000/hr }
   ducklake → { toy: store, backend: "duckdb", path: "~/.patina/lakes/ducklake.db", access: "read-write" }
4. Merge scopes + connections into a single grant object
5. Build toybox: toys + resolved grants + limits
6. Grant WIT imports matching toybox (WASM can only call what's granted)
7. Mediate every call at runtime:
   - Check toybox grants
   - Inject credentials (host-side, never crosses WASM wall)
   - Enforce rate limits
   - Log for audit
   - Return result to child
```

## SDK Helper Design

Domain logic moves from toys to SDK library modules. These use toys internally.

```rust
// sdk/src/helpers/github.rs
// This is LIBRARY CODE, not a toy. It uses the http toy.
pub fn list_issues(connection: &str, owner: &str, repo: &str) -> Result<Vec<Issue>, String> {
    let url = format!("/repos/{}/{}/issues", owner, repo);
    let response = http::request(connection, &HttpRequest {
        method: "GET".into(),
        url,
        headers: vec![("Accept".into(), "application/vnd.github.v3+json".into())],
        body: None,
    })?;
    // Parse GitHub response — domain knowledge lives HERE, not in the toy
    serde_json::from_str(&response.body).map_err(|e| e.to_string())
}
```

Third-party child authors can use these helpers or write their own. The helpers are convenience, not capability. The capability is the `http` toy.

## Collapse Map

| Retired Toy | Absorbed By | Domain Logic Moves To |
|-------------|-------------|----------------------|
| `github` (7 funcs) | `http` | `sdk::helpers::github` |
| `connector` (4 funcs) | `http` | `sdk::helpers::connector` |
| `ingress` (2 funcs) | `http` | `sdk::helpers::ingress` |
| `lake` (7 funcs) | `store` | `sdk::helpers::lake` |
| `belief` (2 funcs) | `store` | `sdk::helpers::belief` |
| `graph` (2 funcs) | `store` | `sdk::helpers::graph` |
| `query` (1 func) | `store` | child code (trivial) |
| `emit` (1 func) | `events` | child code (trivial) |
| `measure` (1 func) | `events` | child code (trivial) |
| `checkpoint` (2 funcs) | `state` | child code (key scoping) |
| `session` (8 funcs) | `fs` + `events` | `sdk::helpers::session` |
| `layer` (varies) | `fs` | child code |
| `layer-fs` (6 funcs) | `fs` | renamed |
| `schema` (0 funcs) | deleted | types absorbed where needed |
| `types` (0 funcs) | deleted | types absorbed where needed |
| `git` (6 funcs) | `git` (kept) | stays — real host capability |

**Decision (closed):** `git` is `patina:git`, the 10th toy (5th Patina-specific). Git operations (tag, commit, log, diff) require host-level execution that WASM cannot do alone. This is a real host capability, not domain logic or an SDK helper. Passes the toy litmus test: "Why can't the child do this itself from pure WASM compute?" — because it needs to shell out to `git` or use `libgit2`, which requires host access.

## SDK Crate Consolidation

### Current (4 crates)

```
sdk/patina-sdk/          — umbrella re-export
sdk/patina-sdk-core/     — core tier (log, state, types, task, events, peer)
sdk/patina-sdk-data/     — data tier (lake, checkpoint, measure, github, connector)
sdk/patina-sdk-agent/    — agent tier (query, emit, session)
```

### Target (1 crate)

```
sdk/patina-sdk/          — everything
  features: knowledge-child, command, pipeline, task,
            http, fs, log, state, store, events, task, peer, git
  src/
    lib.rs               — kind traits, registration macros
    http.rs              — http toy bindings
    fs.rs                — fs toy bindings
    store.rs             — store toy bindings
    events.rs            — events toy bindings
    ...
    helpers/
      github.rs          — GitHub API helper (uses http toy)
      lake.rs            — DuckDB helper (uses store toy)
      session.rs         — Session lifecycle helper (uses fs + events)
      connector.rs       — Sync protocol helper (uses http toy)
```

### Migration Path

1. Move all tier crate code into `patina-sdk`
2. Keep tier crates temporarily as thin re-exports (backward compat)
3. Remove tier crates from workspace after all children migrate

## Child Migration Map

| Child | Current Toys | Target Toys | Notes |
|-------|-------------|-------------|-------|
| ducklake | lake, connector, checkpoint, events, log, state | http, store, events, log, state | Uses `sdk::helpers::lake` + `sdk::helpers::connector` |
| belief-verifier | belief, events, checkpoint, log, state | store, events, log, state | Uses `store` with connection "beliefs" |
| spec-manager | layer-fs, belief, session | fs, store, events | Uses `sdk::helpers::session` |
| session-writer | layer-fs, session, events | fs, events | Uses `sdk::helpers::session` |
| doctor | (stub) | store, log | Minimal |
| lake-manager | lake, connector | store, http | Uses `sdk::helpers::lake` |

## Direct Code Targets

- `wit/toys/*.wit` — rewrite (22 files → 10 files)
- `wit/worlds/*.wit` — regenerate from collapsed toys
- `src/child/toy_host/*.rs` — rewrite host implementations
- `src/child/internal/*.rs` — update engine to handle connections
- `sdk/patina-sdk/` — consolidate tiers, add helpers
- `sdk/patina-sdk-core/` — retire
- `sdk/patina-sdk-data/` — retire
- `sdk/patina-sdk-agent/` — retire
- `children/*/` — migrate all children
- `child.toml` schema — add `[needs.connections]`
- `mother/src/` — update toybox assembly + credential injection

## Breaking Impact Summary

This changes every layer between WIT definitions and child application code. Nothing about the core platform (belief system, core verbs, database, CLI structure) is affected.

**Full rewrite:** `wit/toys/` (22→10 files), `wit/worlds/` (regenerated), `src/child/toy_host/` (every host file), SDK crate structure (4→1 crate)

**Must migrate:** Every in-tree child's source code and `child.toml`. Every test that exercises toy dispatch.

**Untouched:** `patina-core`, `patina-protocol`, Mother services, CLI commands, belief system, layer, database, grammars, git history.

**Risk profile:** High scope, low uncertainty. Every change has a clear before/after. The Phase 0 + 8-phase plan ensures the workspace stays green between phases. The real risk is stamina across multiple sessions, not design ambiguity.

**Migration strategy for `child.toml`:** Open question — hard cut (old toy names fail immediately) vs. compatibility window (Mother parses old names with deprecation warnings). Hard cut is simpler and honest. Compat window is safer for any third-party children.

## Verification Plan

After each phase:
```bash
cargo check --workspace -q
cargo test -q
```

After all phases:
```bash
ls wit/toys/*.wit | wc -l                    # 10
rg "issue|pull-request|review|granted-lake|repo-binding" wit/toys/  # 0
cargo check --workspace -q
cargo test -q
cargo run -q -- child list
cargo run -q -- doctor --json
```

## Build Readiness

Phase 0 (protocol lock + WASI/tooling fit check) is the critical gate. Phase 1 (WIT design) begins only after Phase 0 is frozen.

## Open Questions

- ~~Is `git` toy #9?~~ **Closed.** `patina:git` is the 10th toy. Real host capability, passes litmus test.
- Should `store.query` use SQL strings or a generic query format? Current lean: strings (keep WIT domain-agnostic, let SDK helpers handle query construction).
- How does connection resolution work for local dev vs production? Mother needs a connection registry (like Cloudflare's per-environment bindings).
- Should the `http` toy support streaming responses? Not now — wait for WASI 0.3 async/streams.
- What happens to `child.toml` files that use old toy names? Migration period with compatibility parsing, or hard cut?
