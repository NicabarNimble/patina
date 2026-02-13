---
type: feat
id: plugin-task-world
status: design
created: 2026-02-13
sessions:
  origin: 20260213-120746
blocked_by:
- plugin-host-http
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

## Scope

New WIT world, new engine (or extended `CommandEngine`), new guest API
crate, conformance test.

### WIT (from ecosystem spec, locked)

```wit
world task {
    import patina:host/log@0.1.0;
    import patina:host/query@0.1.0;
    import patina:host/layer@0.1.0;
    import patina:host/types@0.1.0;
    import patina:host/http@0.1.0;

    export init: func();
    export name: func() -> string;
    export description: func() -> string;
    export run: func(args: list<string>) -> s32;
    export toys: func() -> list<toy>;
}
```

After `run()` returns, the host checks `toys()` and executes them with
capability gating. The plugin says "here's what to do." The host decides
whether to actually run the toys.

### Key Architecture Decisions

- Task = command shape (`run(args) -> exit_code`) + toys + HTTP
- Separate WIT world, not an extension of command (per [[separate-worlds-for-isolation]])
- `TaskEngine` in `src/plugin/internal/task.rs` — follows `CommandEngine` pattern
- `TaskHostState` carries full `GrantedCapabilities` (query + http + toys)
- Guest API: `patina-task-api` crate (or module in future `patina-guest` umbrella)
- `QueryDispatchFn` + `HttpDispatchFn` (or direct reqwest) injected per [[lib-owns-policy-binary-owns-wiring]]

### Conformance Test

`hello-task` — runs `echo "hello"` via toy allowlist. Proves: toys gating
works, exit code propagation, task lifecycle. From ecosystem spec conformance
table.

### Dependencies

- `plugin-host-http` must be complete (HTTP interface used by task world)
- `patina:host/query` already complete (build order #1)

## Exit Criteria

- [ ] `wit/task/task.wit` with all 5 host imports
- [ ] `TaskEngine` or extended `CommandEngine` in `src/plugin/internal/`
- [ ] `TaskHostState` with full grants (query + http + toys)
- [ ] Guest API crate with typed helpers
- [ ] Host executes `toys()` after `run()` with capability gating
- [ ] Conformance test: `hello-task` proves toy allowlist + exit code
- [ ] `patina plugin list` shows task-world plugins
- [ ] Pre-push checks pass

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-13 | design | Extracted from [[plugin-ecosystem]] build order item #3. Blocked by HTTP interface. |
