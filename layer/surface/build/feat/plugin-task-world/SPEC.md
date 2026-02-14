---
type: feat
id: plugin-task-world
status: complete
created: 2026-02-13
blocked_by:
- plugin-host-http
sessions:
  origin: 20260213-120746
related:
- layer/surface/build/feat/plugin-ecosystem/SPEC.md
- layer/surface/build/feat/plugin-host-http/SPEC.md
beliefs:
- separate-worlds-for-isolation
- lib-owns-policy-binary-owns-wiring
- two-layer-capability-grants
- plugin-is-agent-plus-skill
---

# feat: Task World (`patina:task`)

> On-demand action plugins. Analyze AND act, then exit.
> The PR-reviewer gap — needs query + toys + HTTP but not a daemon.

## Problem

Command plugins are read-only (inform, don't act). Mother-child plugins
require the daemon. There's no plugin world for on-demand actions that
need both intelligence (query, layer) and side effects (toys, HTTP).

Use cases: PR reviewer (`gh pr review`), one-shot deploy, security scan
with webhook notification.

## Parent Design

Build order item #3 from [[plugin-ecosystem]] SPEC.md. Task world WIT
is defined there (lines 286-298). This spec owns implementation.

## Spec Divergences from Parent

1. **No HttpDispatchFn.** Per host-http spec resolution: `reqwest` is a
   lib crate dependency. Host impl calls `reqwest::blocking::Client`
   directly. Same for task world.
2. **QueryDispatchFn IS needed.** `QueryEngine` lives in the binary crate
   (not lib). Same callback pattern as command world.
3. **Toy execution is host-side, not automatic.** After `run()`, host
   reads `toys()` and filters through `allowed_toy_commands`. Host decides
   execution. Plugin returns intent, not action.

## Scope

New WIT world, new engine, new guest API crate, conformance test.

### WIT (from ecosystem spec, locked)

```wit
world task {
    import patina:host/log@0.1.0;
    import patina:host/query@0.1.0;
    import patina:host/layer@0.1.0;
    import patina:host/types@0.1.0;
    import patina:host/http@0.1.0;

    use patina:host/types@0.1.0.{toy};

    export init: func();
    export name: func() -> string;
    export description: func() -> string;
    export run: func(args: list<string>) -> s32;
    export toys: func() -> list<toy>;
}
```

Note: must `use patina:host/types@0.1.0.{toy}` to bring `toy` into scope
for the `toys()` export return type. Same pattern as mother-child world
(which uses `{child-health, toy}`).

### What NOT to Touch

- `src/plugin/internal/command.rs` — command world stays read-only, no toys
- `src/plugin/internal/mother_child.rs` — separate world, separate engine
- `wit/command/` — command world unaffected
- `wit/mother-child/` — mother-child world unaffected
- `src/mcp/` — MCP server unrelated
- Mother daemon integration — task plugins are CLI-invoked, no daemon

## Architecture

### Pattern: Follow CommandEngine (command.rs)

Task world is structurally closest to command world:
- Same exports: `init`, `name`, `description`, `run(args) -> exit_code`
- Plus: `toys() -> list<toy>` (from mother-child pattern)
- Plus: HTTP import (from host-http spec)

`TaskEngine` follows `CommandEngine` (command.rs:233-327) exactly, with
these additions:
- `TaskHostState` has all of `CommandHostState` fields PLUS `http_client`
- After `run()`, host calls `toys()` and filters through allowed list
- `run_task()` returns both exit code AND filtered toy list

### TaskHostState

Merges `CommandHostState` (command.rs:58-69) + mother-child HTTP expansion:

```rust
pub struct TaskHostState {
    pub plugin_name: String,
    pub wasi: wasmtime_wasi::WasiCtx,
    pub wasi_table: wasmtime::component::ResourceTable,
    pub project_root: Option<PathBuf>,           // from command
    pub grants: GrantedCapabilities,              // from command
    pub query_fn: Option<QueryDispatchFn>,        // from command
    pub http_client: reqwest::blocking::Client,   // from host-http
}
```

Must implement ALL host traits: `log::Host`, `layer::Host`, `query::Host`,
`types::Host`, `http::Host`. Layer and query impls copy directly from
`CommandHostState`. HTTP impl copies from mother-child `HostState` (once
host-http lands). Log and types are trivial.

### Toy Lifecycle

```
Plugin:  run(args)  →  exit_code
         toys()     →  Vec<Toy>

Host:    filter toys through allowed_toy_commands
         execute approved toys (spawn processes)
         return exit_code + toy results to CLI
```

The host never executes toys the manifest didn't approve. Same filtering
pattern as mother-child `WasmChild::tick()` (mother_child.rs:274-302).

### CLI Integration

Task plugins are invoked from CLI, not daemon. Two options:

**Option A: Extend `patina plugin run` with `--world task`**
```bash
patina plugin run pr-reviewer -- --pr 123
```

**Option B: Register as subcommands like command plugins**
```bash
patina review-pr --pr 123
```

Recommend Option A for v1 (simpler, no name collision risk). Option B
can come with plugin-distribution spec.

## Exact Files to Create/Change

### New files

| File | What | Pattern to follow |
|------|------|-------------------|
| `wit/task/task.wit` | Task world WIT definition | `wit/command/command.wit` (33 lines) |
| `wit/task/deps/patina-host/host.wit` | Host interfaces (log + types + layer + query + http) | `wit/command/deps/patina-host/host.wit` + http interface |
| `src/plugin/internal/task.rs` | `TaskEngine` + `TaskHostState` + host trait impls | `src/plugin/internal/command.rs` (327 lines) |
| `patina-task-api/Cargo.toml` | Guest crate manifest | `patina-command-api/Cargo.toml` |
| `patina-task-api/src/lib.rs` | `TaskPlugin` trait + `register_task!` macro | `patina-command-api/src/lib.rs` (207 lines) |
| `patina-task-api/wit/task/task.wit` | WIT copy for guest bindgen | `patina-command-api/wit/command/command.wit` |
| `patina-task-api/wit/task/deps/patina-host/host.wit` | Host WIT copy | sync from `wit/task/deps/` |

### Modified files

| File | What changes |
|------|-------------|
| `src/plugin/internal/mod.rs` | Add `mod task; pub use task::TaskEngine;` |
| `src/plugin/mod.rs` | Add `TaskEngine` to pub use re-export |
| `src/main.rs` | Add task dispatch in plugin subcommand or existing plugin run path |
| `Cargo.toml` | Add `patina-task-api` to workspace members |
| `src/plugin/internal/tests.rs` | Conformance tests for task world |

### Not changing

`command.rs`, `mother_child.rs`, `wit/command/`, `wit/mother-child/`,
`src/mcp/`, `patina-plugin-api/`, `patina-command-api/`

## Implementation Plan (4 commits)

**Commit 1: WIT + TaskEngine skeleton**
- Create `wit/task/task.wit` with all 5 host imports + typed exports
- Create `wit/task/deps/patina-host/host.wit` (sync all interfaces)
- Create `src/plugin/internal/task.rs` with:
  - `task_bindings` module (bindgen + TaskHostState)
  - `TaskEngine::new()` (same Linker setup pattern as CommandEngine)
  - `TaskEngine::run_task()` → returns `(exit_code, Vec<Toy>)`
- All host trait impls for TaskHostState:
  - `log::Host` — copy from command_bindings
  - `layer::Host` — copy from command_bindings
  - `query::Host` — copy from command_bindings
  - `types::Host` — empty impl
  - `http::Host` — copy from mother-child (once host-http lands)
- Wire into `mod.rs` and `plugin/mod.rs`

**Commit 2: Guest API crate**
- Create `patina-task-api/` crate
- `TaskPlugin` trait: `name()`, `description()`, `run(args) -> i32`, `toys() -> Vec<Toy>`
- `register_task!` macro (same pattern as `register_command!`)
- Re-export wrappers: `host_log`, `layer`, `query`, `http`
- Add to workspace in root `Cargo.toml`

**Commit 3: CLI integration**
- Add `Run` variant to `PluginCommands` enum in main.rs
- `patina plugin run <name> [-- args...]` dispatches to TaskEngine
  (or CommandEngine, auto-detect from manifest world field)
- Wire `make_query_dispatch()` for task plugins
- Build HTTP client with redirect policy (same as mother-child)
- After run: filter toys, execute approved ones, report results

**Commit 4: Conformance test**
- Create `hello-task` test fixture (minimal task plugin):
  - Returns exit code 0
  - Returns one toy: `echo "hello"` (command: "echo", args: ["hello"])
  - Manifest: `world = "task"`, `host_log = true`,
    `[capabilities.toys] commands = ["echo"]`
- Build to .wasm, add to tests/fixtures/
- Tests in `tests.rs`:
  - `run_task()` returns exit code 0
  - `toys()` returns filtered toy list
  - Toy with unapproved command is filtered out
  - `name()` and `description()` work

## Dependencies

- `plugin-host-http` must be complete (HTTP interface used by task world)
- `patina:host/query` already complete (build order #1)
- Mother-child HostState expansion (from host-http) provides the HTTP
  impl pattern to copy

## Exit Criteria

- [ ] `wit/task/task.wit` with all 5 host imports + typed exports
- [ ] `TaskEngine` in `src/plugin/internal/task.rs`
- [ ] `TaskHostState` with full grants (query + http + toys)
- [ ] All 5 host traits implemented for TaskHostState
- [ ] `run_task()` returns both exit code and filtered toy list
- [ ] Guest API crate `patina-task-api` with `TaskPlugin` trait
- [ ] `register_task!` macro generates correct init export
- [ ] CLI dispatch via `patina plugin run <name>`
- [ ] Conformance test: `hello-task` proves toy allowlist + exit code
- [ ] `cargo test --workspace` passes
- [ ] `./resources/git/pre-push-checks.sh` passes

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-13 | design | Extracted from [[plugin-ecosystem]] build order item #3. Blocked by HTTP interface. |
| 2026-02-13 | design | Refined in session [[20260213-135136]]. Added exact files list, commit plan, code patterns (cite command.rs + mother_child.rs), CLI integration design, "What NOT to Touch" section. |
