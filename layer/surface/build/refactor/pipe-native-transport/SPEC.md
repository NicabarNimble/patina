---
type: refactor
id: pipe-native-transport
status: draft
created: 2026-03-06
blocked_by:
- pipe-protocol-types
sessions:
  origin: 20260306-171859
related:
- pipe-architecture
beliefs:
- pipes-are-processes-not-wasm
- host-proxied-io-is-the-security-model
exit_criteria:
- id: pipe-crate-compiles
  text: '`patina-pipe` crate compiles with Child trait and run() function — native children can depend on it and implement the trait'
  checked: false
- id: test-child-works
  text: A test child binary can be spawned, initialized (pipe/initialize handshake), and respond to pipe/fetch with streaming pipe/fact notifications
  checked: false
- id: sandbox-profile-exists
  text: OS sandbox profiles exist for native children — macOS sandbox-exec profile restricting filesystem/network to declared domains, Linux Landlock stub (compiles, logs warning if unsupported kernel)
  checked: false
---
# refactor: Pipe Native Transport — Child Trait + stdio JSON-RPC

> Build the native transport binding for pipe protocol. Children are
> normal Rust binaries that speak JSON-RPC 2.0 over stdio, sandboxed
> by the OS. Normal `cargo run`, `cargo test`, `dbg!()` development.

## Context

[[spec-pipe-architecture]] defines two transport bindings: WASM host
calls (existing, via patina-sdk) and native stdio (new, via
patina-pipe). This spec builds the native binding.

**What exists today:**
- `src/mcp/server/mod.rs` — stdio JSON-RPC 2.0 server for MCP.
  The native transport follows this exact pattern: read lines from
  stdin, parse JSON-RPC, dispatch to handler, write response to
  stdout.
- `patina-pipe-types` (from [[spec-pipe-protocol-types]]) — shared
  types (Fact, PipeError, Capabilities, FetchParams)
- Mother-child plugin framework — spawn, heartbeat, health for
  WASM children. Native children need the same lifecycle.

**What this spec delivers:**
- `crates/patina-pipe/` — native transport crate
- `Child` trait that native children implement
- `run()` entry point that handles JSON-RPC protocol
- `FactEmitter` for streaming fact delivery (no Vec<Fact> OOM)
- OS sandbox profile for macOS (sandbox-exec)

## Current State

No native child support exists. All children are WASM plugins running
in wasmtime via mother-child world. Native processes only serve MCP
(which is a different protocol, not pipe protocol).

## Target State

```
crates/patina-pipe/
  src/
    lib.rs          # Child trait, run() entry point, re-exports
    transport.rs    # stdio JSON-RPC read/write (follow MCP pattern)
    emitter.rs      # FactEmitter (streaming, O(1) memory)
    signing.rs      # content_hash + signature stub (until persona-federation)

resources/sandbox/
    macos-child.sb  # macOS sandbox-exec profile
    linux-child.rs  # Linux Landlock stub (compiles, warns if unsupported)
```

A native child looks like:

```rust
use patina_pipe::{Child, run, FactEmitter, FetchParams, PipeError};

struct MyConnector;

impl Child for MyConnector {
    fn capabilities(&self) -> Capabilities { ... }
    fn fetch(&mut self, params: &FetchParams, emitter: &mut FactEmitter)
        -> Result<FetchResult, PipeError> { ... }
    fn health(&self, params: &FetchParams) -> Result<Status, PipeError> { ... }
}

fn main() { run(MyConnector).unwrap(); }
```

Mother spawns native children as OS processes, communicating over
stdin/stdout. stderr is for child logging (not protocol).

## Steps

1. Create `crates/patina-pipe/` with Child trait matching DESIGN.md
   §2.5 (capabilities, fetch with &mut self + FactEmitter, health)
2. Implement `run()` — stdio JSON-RPC dispatcher following
   `src/mcp/server/mod.rs` pattern (BufReader, line-delimited,
   dispatch by method name)
3. Implement `FactEmitter` — writes pipe/fact notifications to
   stdout as JSON-RPC notifications (streaming, no accumulation)
4. Implement pipe/initialize handshake (config delivery, capability
   exchange)
5. Create OS sandbox profiles: macOS sandbox-exec
   (`resources/sandbox/macos-child.sb`) restricting to
   stdin/stdout/stderr + declared network domains; Linux Landlock
   stub that compiles and logs warning on unsupported kernels
6. Build a test child binary (`examples/test-child/`) that implements
   Child trait and responds to pipe/fetch
7. Add minimal Mother-side spawn logic: fork+exec child binary with
   sandbox, connect stdio, send pipe/initialize, dispatch pipe/fetch.
   Scope: enough to test the test-child end-to-end. Production
   lifecycle management (routing, fan-out, scheduling) is
   [[spec-mother-broker]] scope.
8. Verify: test child can be spawned, initialized, and stream facts
   back to Mother

## Key Files

**Read before implementing:**
- `src/mcp/server/mod.rs` — stdio JSON-RPC pattern to follow
- `src/plugin/internal/mod.rs` — WASM child lifecycle (spawn,
  heartbeat, health) — native equivalent needed
- [[spec-pipe-architecture]] DESIGN.md §2 (Child Framework),
  §7.2 (Native Transport), §8.3 (Sandbox Detail)

## Non-Goals

- HTTP+SSE transport (future, when remote children exist)
- WASM transport changes (patina-sdk handles that separately)
- Building a real connector (that's [[spec-github-connector]])
- Mother routing engine (that's [[spec-mother-broker]])
