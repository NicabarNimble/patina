# Design: Collapse toys to primitives and align with WASI/Cloudflare binding model

## Why This Design

Patina independently arrived at capability-based security — the same architecture behind Cloudflare Workers, Deno permissions, WASI, and browser sandboxes. But we baked domain logic into our toy interfaces (github API shapes, DuckDB operations, session lifecycle), creating 22 toys where ~8 primitives suffice.

The collapse aligns us with WASI standards where they exist, positions our Patina-specific interfaces (store, events, task) as potential WASI contributions, and simplifies everything downstream: fewer WIT files, one SDK crate, simpler toy host, cleaner security model.

The Cloudflare Workers binding model validates the approach at scale: ~8 primitive binding types serve millions of Workers across every domain. Domain specificity belongs in the Worker (child), not the binding (toy).

## Build Target

7 phases. The WIT interface design (Phase 1) is the critical decision point — everything else flows from it.

## Vocabulary

| Term | Definition |
|------|-----------|
| **Toy** | A primitive capability Mother grants. A door in the WASM sandbox wall. ~8 of them. Defined as WIT interfaces. |
| **Toybox** | The sealed capability payload Mother assembles for a child at init. Contains granted toys + resolved connections. The capability contract. |
| **Kind** | The child's runtime lifecycle shape: knowledge-child, command, pipeline, task. Determines how Mother manages the child. Not related to toys. |
| **Connection** | A named binding in `child.toml`. The child says a name ("github"), Mother resolves it to toy + credential + endpoint + config. Like a Cloudflare Workers binding in `wrangler.toml`. |
| **World** | WIT implementation detail only. The composed set of imports/exports for wit-bindgen. Derived from kind + toybox. Child authors never use this word. |

## The 8 Toys

### WASI-Aligned (adopt standard interface shapes)

**http** — Outbound HTTP requests. Mirrors `wasi:http/outgoing-handler`.

```wit
interface http {
    record request {
        method: string,
        url: string,
        headers: list<tuple<string, string>>,
        body: option<string>,
    }
    record response {
        status: u16,
        headers: list<tuple<string, string>>,
        body: string,
    }
    request: func(connection: string, req: request) -> result<response, string>;
}
```

The `connection` parameter is the key innovation over raw WASI. Mother resolves it: "github" → base URL + auth headers + rate limits. The child constructs the path and payload; Mother handles the infrastructure.

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
    query: func(connection: string, query: string) -> result<string, string>;
    mutate: func(connection: string, action: string, payload: string) -> result<string, string>;
    list-connections: func() -> list<string>;
}
```

Absorbs: `lake`, `belief`, `graph`, `query`. The `connection` parameter distinguishes targets: "ducklake", "beliefs", "graph". Mother resolves each to its backing store (SQLite, DuckDB, whatever). The child knows how to query its domain; the toy just opens the door.

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
4. Build toybox: toys + resolved connections + limits
5. Grant WIT imports matching toybox (WASM can only call what's granted)
6. Mediate every call at runtime:
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

Wait — that's 9 toys (including `git`). Decision needed: is `git` a toy or an SDK helper over `fs`?

Git operations (create-tag, commit, log, diff) are host commands that WASM cannot execute alone. They require shelling out to git or using libgit2. This is a real host capability, not domain logic. **Keep git as the 9th toy**, or fold it into a general "exec" primitive. For now: keep it. 8 was aspirational; 9 with git is honest.

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

- `wit/toys/*.wit` — rewrite (22 files → 8-9 files)
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

## Verification Plan

After each phase:
```bash
cargo check --workspace -q
cargo test -q
```

After all phases:
```bash
ls wit/toys/*.wit | wc -l                    # 8 or 9
rg "issue|pull-request|review|granted-lake|repo-binding" wit/toys/  # 0
cargo check --workspace -q
cargo test -q
cargo run -q -- child list
cargo run -q -- doctor --json
```

## Build Readiness

Phase 1 (WIT design) is the critical gate. Requires studying WASI interface shapes before committing. Everything else flows from the WIT decisions.

## Open Questions

- Is `git` toy #9, or does it collapse into a general `exec` primitive? Current lean: keep as standalone — git is a real host capability.
- Should `store.query` use SQL strings or a generic query format? Current lean: strings (keep WIT domain-agnostic, let SDK helpers handle query construction).
- How does connection resolution work for local dev vs production? Mother needs a connection registry (like Cloudflare's per-environment bindings).
- Should the `http` toy support streaming responses? Not now — wait for WASI 0.3 async/streams.
- What happens to `child.toml` files that use old toy names? Migration period with compatibility parsing, or hard cut?
