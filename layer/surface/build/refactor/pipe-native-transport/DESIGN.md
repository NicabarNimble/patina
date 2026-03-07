# Design: Pipe Native Transport — Why Children Should Be Processes

## Why This Work Exists

WASM children work. The forge connector proves it. But WASM development
is painful: no `dbg!()`, no direct HTTP, no standard library networking,
every syscall proxied through host functions. When a developer wants to
build a Slack connector or RSS reader, they shouldn't need to learn
WASM tooling. They should write normal Rust, use normal crates, run
normal tests.

[[pipes-are-processes-not-wasm]] captures the insight from Session 5:
"Native processes give you reqwest, tokio, normal debugging, cargo test,
the entire crates.io ecosystem. WASM gives you a sandbox. The sandbox
is valuable — but OS-level sandboxing gives you the same security
guarantees for native processes."

This crate is the native transport binding for pipe protocol. It
provides the `Child` trait and `run()` entry point so that a developer
can write a connector in ~50 lines of Rust and have it participate
fully in the pipe architecture.

**Origin:** [[session-20260305-224446]] (discovered pipes should be
processes, designed OS sandbox model), [[session-20260306-123021]]
(Vec<Fact> OOM risk — streaming delivery), [[session-20260306-174214]]
(audit: sandbox must fail loud, `&mut self` for mutable state,
explicit error handling for all protocol errors).

## The Pattern We're Following

The native transport follows the exact same stdio JSON-RPC 2.0 pattern
as `src/mcp/server/mod.rs` + `src/mcp/protocol.rs`: BufReader on stdin,
line-delimited JSON-RPC, dispatch by method name, write responses to
stdout. This isn't coincidence — it's [[unix-philosophy]] at work. The
MCP server proved the pattern. The pipe transport reuses it.

Separate from MCP types because pipe protocol has different methods and
the crates shouldn't depend on each other. Same shape, different
vocabulary.

## Design Decisions

### 1. Streaming Delivery via FactEmitter

The Session 7 audit found the Vec<Fact> OOM risk: "A child fetching
100K GitHub issues would accumulate all of them in memory before
returning." The fix is streaming delivery.

Each `emitter.emit()` call writes a `pipe/fact` JSON-RPC notification
to stdout immediately — no buffering, O(1) memory. Mother processes
each fact as it arrives. The final `pipe/fetch` response is just a
summary (count + cursor).

```
Mother --> child:  pipe/fetch {since, types}
child  --> Mother: pipe/fact {fact_1}         (notification, no id)
child  --> Mother: pipe/fact {fact_2}         (notification, no id)
child  --> Mother: {"id":1,"result":{"emitted":2}}  (response)
```

Mother reads lines and dispatches: lines with `"method"` and no `"id"`
are notifications, lines with `"id"` and `"result"`/`"error"` are the
response.

**Why clone data in emit()?** `emit()` takes `&serde_json::Value` and
clones into the Fact struct. The alternative (taking ownership) would
force children to build values they can't reuse. Since facts are
written to stdout immediately and dropped, the clone lifetime is brief.
For GitHub issues (~1-5KB each), negligible.

### 2. Child Trait with &mut self

The audit found `&self` prevents mutable state. Connectors need
connection pools, rate limit tracking, cached auth across calls.
`&mut self` allows it. Object safety preserved — `run()` works with
concrete types (monomorphized `<C: Child>`), no trait objects needed.

```rust
pub trait Child {
    fn capabilities(&self) -> Capabilities;
    fn initialize(&mut self, params: &InitializeParams) -> Result<(), PipeError>;
    fn fetch(&mut self, params: &FetchParams, emitter: &mut FactEmitter) -> Result<FetchResult, PipeError>;
    fn health(&self) -> Result<HealthStatus, PipeError>;
}
```

Three required methods, one with a default (initialize does nothing
for children that don't need config). No generics on the trait —
concrete types mean no monomorphization bloat, and children parse
provider-specific params from `params.params` (the `serde_json::Value`
field), matching how MCP tools work.

### 3. run() Handles All Protocol Errors Explicitly

Every error path is enumerable. No panics. No `unwrap()` on protocol
messages.

| Error | Detection | Response |
|-------|-----------|----------|
| Malformed JSON | `serde_json::from_str` fails | JSON-RPC -32700 Parse error |
| Wrong JSON-RPC version | `request.jsonrpc != "2.0"` | JSON-RPC -32600 Invalid request |
| Unknown method | `match` default arm | JSON-RPC -32601 Method not found |
| Invalid params | `serde_json::from_value` fails | JSON-RPC -32602 Invalid params |
| Fetch before initialize | `!initialized` flag | Fatal error |
| Child returns PipeError | `child.fetch()` returns Err | PipeError mapped to JSON-RPC code |
| stdin closed | `reader.lines()` ends | Loop exits, process exits 0 |
| stdout broken pipe | `writeln!` fails | Process exits with error |

### 4. Stdout Sharing Between run() and FactEmitter

During `pipe/fetch`, both `run()` and FactEmitter need stdout. The
emitter borrows `&mut stdout` for the duration of the fetch call.
After fetch returns, `run()` reclaims stdout to write the response.

This means a child can't emit facts AND respond to health checks
concurrently. Fine for poll mode (single fetch, then exit). Stream
mode will need a channel-based writer — deferred to a future spec.

### 5. Sandbox Must Fail Loud

The Session 12 audit established this constraint. When a sandboxed
child tries to connect on a non-allowed port, it gets EACCES/EPERM on
`connect()` immediately — a connection error, NOT a silent timeout.
The developer sees a clear error, not mysterious hangs.

Current sandbox restricts to port 443 + DNS (port-level only — OS
sandboxes cannot filter by hostname). [[spec-pipe-mother-io]] tightens
this to deny ALL outbound sockets, with children using Mother's
`pipe/http` proxy for domain-enforced networking.

## OS Sandbox Model

Native children run inside OS-level sandboxes. The sandbox denies
filesystem access and restricts network to port 443 + DNS. Domain-level
enforcement happens in Mother via `pipe/http` ([[spec-pipe-mother-io]]),
not in the OS sandbox.

### macOS: Sandbox via C API (`sandbox_init`)

The `sandbox-exec` CLI tool is deprecated (since ~2019, still works on
macOS 15). The kernel sandbox mechanism itself is NOT deprecated. To
avoid depending on a tool that may vanish, Mother uses the C API
directly: `sandbox_init()` from `libsandbox`.

**Invocation path:** Fork → in child process, call `sandbox_init()`
with the compiled profile → exec the child binary. The profile is
applied before exec, so the child binary starts already sandboxed.
No external tool dependency.

```rust
// src/broker/sandbox/macos.rs (thin FFI layer)
extern "C" {
    fn sandbox_init(
        profile: *const c_char,
        flags: u64,
        errorbuf: *mut *mut c_char,
    ) -> c_int;
    fn sandbox_free_error(errorbuf: *mut c_char);
}

const SANDBOX_INLINE: u64 = 0x0000;  // interpret profile as inline SBPL

/// Apply sandbox profile in the current process (call after fork,
/// before exec). Returns Ok(()) on success, Err with Apple's error
/// message on failure.
pub fn apply_sandbox(profile: &str) -> Result<()> { /* ... */ }
```

Profile format is inline SBPL (Scheme syntax). Current profile
restricts network to port 443 + DNS (port-level). [[spec-pipe-mother-io]]
will tighten to deny all network-outbound once Mother proxies HTTP.

```scheme
(version 1)
(deny default)
(allow file-read*  (literal "/dev/stdin"))
(allow file-write* (literal "/dev/stdout"))
(allow file-write* (literal "/dev/stderr"))
(allow network-outbound (remote ip "*:443"))   ;; HTTPS — removed by pipe-mother-io
(allow network-outbound (remote ip "*:53"))    ;; DNS — removed by pipe-mother-io
```

When `allowed_domains` is empty, the port 443 rule is omitted —
denying all network except DNS.

**Cost:** ~2ms startup, ~0ns runtime (kernel-enforced). Same Chrome
renderer process pattern, but without the deprecated CLI wrapper.

**Fail behavior:** If `sandbox_init()` returns an error (e.g., invalid
profile syntax), Mother refuses to spawn the child and surfaces the
Apple error message. `--no-sandbox` opt-out is [[spec-mother-broker]]
scope.

### Linux: Landlock Enforcement (ABI v4+)

Landlock ABI v4 (kernel 6.7+, Jan 2024) added network restriction —
the last piece needed for parity with macOS sandbox-exec.

Implementation uses the `landlock` crate to restrict:
- Filesystem: deny all access (child communicates only via stdio)
- Network: allow only port 443 (HTTPS) and port 53 (DNS) outbound
- Process: no spawning child processes

Port-level only — Landlock cannot filter by hostname. Domain-level
enforcement happens in Mother via `pipe/http` ([[spec-pipe-mother-io]]).
When pipe-mother-io lands, the 443/53 rules are removed — children
use Mother's proxy for all HTTP and get EPERM on direct sockets.

**Fail behavior:** If the running kernel does not support Landlock v4,
`check_landlock_support()` returns an error (uses `HardRequirement`
probe — `SoftRequirement` would silently downgrade and always succeed).
`--no-sandbox` opt-out is [[spec-mother-broker]] scope.

**Testing:** Requires a Linux 6.7+ system. Integration tests or manual
validation notes documenting the tested kernel version and observed
enforcement behavior.

### Debug Mode

`--sandbox-debug` or `PATINA_SANDBOX_DEBUG=1` skips sandbox
enforcement and logs the profile that would have been applied. For
debugging sandbox-related connection failures.

## Protocol Sequence Diagrams

### Poll Mode (github-connector)

```
Mother                                     Child
  |                                          |
  |--- spawn child (fork+exec in sandbox) -->|
  |                                          |
  |--- pipe/initialize {auth, version} ----->|
  |<-- result {capabilities} ---------------|
  |                                          |
  |--- pipe/fetch {types, since, limit} ---->|
  |<-- pipe/fact {github, issue, data1} ----|  (notification)
  |<-- pipe/fact {github, issue, data2} ----|  (notification)
  |<-- pipe/fact {github, pr, data3} -------|  (notification)
  |<-- result {emitted: 3, cursor: "..."} --|
  |                                          |
  |--- pipe/shutdown ----------------------->|
  |<-- result {} ----------------------------|
  |                                          |
  |--- (child process exits) ---              |
```

### Error During Fetch (Partial)

```
Mother                                     Child
  |                                          |
  |--- pipe/fetch {types, since} ----------->|
  |<-- pipe/fact {github, issue, data1} ----|  (emitted before error)
  |<-- pipe/fact {github, issue, data2} ----|
  |<-- error {Partial, emitted: 2} ---------|
  |                                          |
  |--- (Mother keeps 2 facts, retries later)  |
```

### Rate Limited

```
Mother                                     Child
  |                                          |
  |--- pipe/fetch {types, since} ----------->|
  |<-- error {RateLimited, retry: 60000} ---|
  |                                          |
  |--- (Mother waits 60s, then retries) ----  |
```

## Crate Structure

```
crates/patina-pipe/
  Cargo.toml                # depends on patina-pipe-types, serde, serde_json
  src/
    lib.rs                  # Child trait + run() + re-exports
    protocol.rs             # JSON-RPC 2.0 types (Request, Response, Notification)
    emitter.rs              # FactEmitter (streaming fact delivery)
  examples/
    test-child.rs           # Minimal child for integration testing
```

No new deps beyond what patina-pipe-types brings. The child author
adds their own deps (reqwest, etc.) in their binary crate.

## What's NOT In Scope

- **Mother-side spawn logic** — that's mother-broker scope. This crate
  provides a test harness for protocol verification, not production
  lifecycle management.
- **Stream mode** — this transport handles poll mode (single fetch,
  exit). Stream mode needs concurrent health checks during fetch,
  which requires a channel-based writer. Deferred.
- **HTTP transport** — future. Same JSON-RPC messages, different wire.
  Isolated to a transport binding, not the Child trait.
- **WASM transport** — that's patina-sdk. Same protocol, different
  binding. They share types via patina-pipe-types.

## Belief Anchors

- [[pipes-are-processes-not-wasm]] — native children give developers
  the full Rust ecosystem. OS sandbox provides equivalent security.
- [[host-proxied-io-is-the-security-model]] — sandbox-exec prevents
  filesystem access and undeclared network. Credentials arrive via
  stdin, not environment or files.
- [[pipe-protocol-is-transport-agnostic]] — this crate is one
  transport binding. The Child trait and run() are specific to native
  stdio. The protocol (methods, types, semantics) is shared.

## Resolved Questions

1. **sandbox-exec deprecation.** → Use `sandbox_init()` C API directly
   instead of the deprecated `sandbox-exec` CLI tool. The kernel sandbox
   mechanism isn't deprecated — only the CLI wrapper is. Thin FFI layer
   (~30-50 lines) calls `sandbox_init()` after fork, before exec. Same
   profile format, no external tool dependency. (Session 15 audit)

## Open Questions

1. **Stdout contention in stream mode.** FactEmitter borrows
   `&mut stdout` during fetch, preventing concurrent health checks.
   Fine for poll mode. Stream mode needs a different approach — likely
   a channel-based writer with a dedicated stdout thread.

## Commits

1. `pipe: add patina-pipe crate with protocol types` — Create crate,
   Cargo.toml, protocol.rs (Request, Response, Notification). Follow
   MCP protocol.rs pattern.

2. `pipe: implement Child trait and FactEmitter` — lib.rs with Child
   trait, emitter.rs with streaming FactEmitter.

3. `pipe: implement run() dispatcher` — Full run() function: stdin
   reader, method dispatch, explicit error handling for all paths.

4. `pipe: add test-child example` — examples/test-child.rs implementing
   Child trait with fake data. Verify protocol end-to-end.

5. `pipe: add macOS sandbox via sandbox_init() C API` — Thin FFI layer
   for sandbox_init(), profile template with domain placeholder,
   fork→sandbox→exec spawn path. No sandbox-exec CLI dependency.

6. `pipe: add Linux Landlock enforcement` — Landlock ABI v4+ network
   and filesystem restrictions. Kernel detection via HardRequirement
   probe, fail-hard on unsupported kernels. Requires 6.7+ for
   testing. `--no-sandbox` opt-out is mother-broker scope.

7. `pipe: add Mother-side test harness` — spawn_child_test(),
   ChildConnection for integration testing.

## Key Files

- `crates/patina-pipe/src/lib.rs` — Child trait + run() entry point
- `crates/patina-pipe/src/protocol.rs` — JSON-RPC types
- `crates/patina-pipe/src/emitter.rs` — FactEmitter (streaming)
- `crates/patina-pipe/examples/test-child.rs` — protocol test binary
- `resources/sandbox/macos-child.sb` — OS sandbox profile template
- `src/mcp/server/mod.rs` — pattern to follow (stdio JSON-RPC)
- `src/mcp/protocol.rs` — pattern to follow (Request/Response types)
