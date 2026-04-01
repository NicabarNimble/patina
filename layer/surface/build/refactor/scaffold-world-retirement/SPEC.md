---
type: refactor
id: scaffold-world-retirement
status: ready
created: 2026-03-26
sessions:
  origin: 20260326-165149-931909000
related:
- src/child/internal/command.rs
- src/child/internal/task.rs
- src/child/internal/pipeline.rs
- src/child/internal/mod.rs
- sdk/patina-sdk/src/command.rs
- sdk/patina-sdk/src/task.rs
- sdk/patina-sdk/src/pipeline.rs
- sdk/patina-sdk/src/lib.rs
- sdk/patina-sdk/wit/command/
- sdk/patina-sdk/wit/task/
- sdk/patina-sdk/wit/mother-child/
- sdk/patina-sdk/wit/pipeline/
- sdk/patina-sdk/README.md
- wit/command/command.wit
- wit/task/task.wit
- wit/deps/patina-host/host.wit
- wit/pipeline/pipeline.wit
- resources/templates/child/command/
- resources/templates/child/task/
- src/main.rs
- src/child/scaffold.rs
beliefs:
- '[[children-are-wasm]]'
- '[[four-roles-no-overlap]]'
- '[[children-have-agency-toys-are-capabilities]]'
- '[[investigate-before-delete]]'
exit_criteria:
- id: swr0-command-kind-removed
  text: ChildKind::Command variant removed from enum, CommandEngine deleted, and all host-side command-kind code removed.
  checked: true
- id: swr1-task-kind-removed
  text: ChildKind::Task variant removed from enum, TaskEngine deleted, and all host-side task-kind code removed.
  checked: true
- id: swr2-sdk-command-removed
  text: sdk/patina-sdk/src/command.rs deleted, `command` feature removed from SDK Cargo.toml, compile-time exclusion simplified.
  checked: true
- id: swr3-sdk-task-removed
  text: sdk/patina-sdk/src/task.rs deleted, `task` feature removed from SDK Cargo.toml.
  checked: true
- id: swr4-wit-command-removed
  text: wit/command/command.wit deleted. Directory removed.
  checked: true
- id: swr5-wit-task-removed
  text: wit/task/task.wit deleted. Directory removed.
  checked: true
- id: swr6-templates-removed
  text: resources/templates/child/command/ and resources/templates/child/task/ directories deleted.
  checked: true
- id: swr7-pipeline-log-migrated
  text: Pipeline world migrated from patina:host/log import to patina:log/log (per-package), matching knowledge-child convention.
  checked: true
- id: swr8-host-wit-removed
  text: wit/deps/patina-host/host.wit deleted. No WIT files in wit/ OR sdk/patina-sdk/wit/ import patina:host/* namespace.
  checked: true
- id: swr8b-sdk-wit-snapshots-removed
  text: 'SDK-local WIT snapshots for dead worlds deleted: sdk/patina-sdk/wit/command/, sdk/patina-sdk/wit/task/, sdk/patina-sdk/wit/mother-child/. Pipeline snapshot updated for per-package log import.'
  checked: true
- id: swr9-role-extension-updated
  text: ChildRole::Extension `expected_worlds()` no longer returns dead kinds. Updated to reflect post-retirement valid worlds.
  checked: true
- id: swr10-child-run-updated
  text: '`patina child run` dispatch updated: knowledge-child arm intact, command/task arms removed. Catch-all error for unsupported kinds reads: "child ''{name}'' has kind ''{kind}'' — only ''knowledge-child'' is supported by `child run`".'
  checked: true
- id: swr11-scaffold-updated
  text: '`patina child init` only offers knowledge-child and pipeline templates. No broken template references. Help text updated.'
  checked: true
- id: swr12-retired-kind-error
  text: ChildKind::from_str for 'command' and 'task' returns a clear retired-kind error message (not generic 'unknown kind') guiding migration to knowledge-child.
  checked: true
- id: swr13-docs-updated
  text: sdk/patina-sdk/README.md stability table, world features list, and breaking change notes updated. No references to dead kinds remain in docs.
  checked: true
- id: swr14-gates-green
  text: cargo check --workspace -q and cargo test -q --workspace pass. No dead code warnings introduced.
  checked: true
---
# refactor: scaffold world retirement

## Problem

The `command` and `task` child kinds exist as speculative scaffolding from before the knowledge-child doctrine landed. They have:

- **Zero production consumers.** All 7 children in `children/` are `kind = "knowledge-child"`.
- **226 + 314 lines of host engines** (`CommandEngine`, `TaskEngine`) called only from a CLI fallback path.
- **232 + 263 lines of SDK modules** with guest traits, macros, and re-exports that no child implements.
- **Two WIT worlds** using the pre-collapse `patina:host/*` import namespace instead of per-package toy imports.
- **Six scaffold template files** for kinds nobody builds.

The `command` kind name specifically creates a naming collision with the upcoming `child-command-surface` spec, which is about knowledge-children owning CLI command surfaces — a completely different concept.

The `patina:host/*` WIT package (413 lines) exists only to serve these dead worlds plus the pipeline world's `log` import. Once pipeline is migrated to `patina:log/log`, the entire legacy namespace is deletable.

## Goal

Remove the `command` and `task` child kinds, their engines, SDK modules, WIT worlds, and templates. Migrate pipeline's sole `patina:host/log` import to the per-package `patina:log/log`. Delete the `patina:host/*` WIT package entirely.

Result: two child kinds remain — `knowledge-child` (daemon workers) and `pipeline` (pure compute) — both using per-package WIT imports.

## Status

Draft. Scope and targets are clear. Implementation has not started.

## Non-Goals

- Retiring the `pipeline` kind — it has real consumers (grammar plugins used by `scrape` and `bench` commands).
- Retiring `ChildRole` — advisory-only, harmless, and still meaningful for knowledge-child roles.
- Changing the `child-command-surface` spec — that spec adds command *surfaces* to knowledge-children; this spec removes the naming collision by deleting the dead `command` kind first.
- Rewriting the pipeline engine or SDK — only the import path changes (`patina:host/log` → `patina:log/log`).

## Solution

### Phase 1 — Pipeline log import migration

Migrate pipeline from `patina:host/log` to `patina:log/log` so it no longer depends on the legacy `patina:host/*` package. This touches WIT, host engine, and SDK bindings.

- Update `wit/pipeline/pipeline.wit`: change `import patina:host/log@0.1.0` to `import patina:log/log@0.1.0`
- Update `sdk/patina-sdk/wit/pipeline/pipeline.wit`: same import change
- Update `src/child/internal/pipeline.rs`: host trait impl changes from `patina::host::log::Host` to `patina::log::log::Host` (binding path change from WIT regeneration)
- Update `sdk/patina-sdk/src/pipeline.rs`: guest re-export path changes from `patina::host::log` to `patina::log::log`
- Regenerate bindings
- Verify grammar plugins still compile and run (`cargo test`, `patina bench grammar`)

### Phase 2 — Delete command kind

Host-side:
- Delete `src/child/internal/command.rs`
- Remove `ChildKind::Command` from enum in `src/child/internal/mod.rs`
- Add retired-kind error: `"command"` in `ChildKind::from_str` returns `"child kind 'command' is retired; migrate to 'knowledge-child'"`
- Remove `Command` match arm from `src/main.rs` (`patina child run` dispatch, line ~1527)
- Remove `Command` match arms from `src/child/scaffold.rs`
- Remove command-kind test cases from `src/child/internal/tests.rs`

SDK-side:
- Delete `sdk/patina-sdk/src/command.rs`
- Remove `command` feature from `sdk/patina-sdk/Cargo.toml`
- Remove `command` compile-time exclusion arms from `sdk/patina-sdk/src/lib.rs`
- Delete `sdk/patina-sdk/wit/command/` directory (SDK-local WIT snapshot)

WIT and templates:
- Delete `wit/command/` directory (runtime WIT)
- Delete `resources/templates/child/command/` directory

Docs:
- Update `sdk/patina-sdk/README.md` — remove command from world features, stability table

### Phase 3 — Delete task kind

Same pattern as Phase 2:

Host-side:
- Delete `src/child/internal/task.rs`
- Remove `ChildKind::Task` from enum, add retired-kind error in `from_str`
- Remove `Task` match arm from `src/main.rs` (line ~1483)
- Remove `Task` match arms from `src/child/scaffold.rs`
- Remove task-kind test cases from `src/child/internal/tests.rs`

SDK-side:
- Delete `sdk/patina-sdk/src/task.rs`, remove `task` feature from Cargo.toml and lib.rs
- Delete `sdk/patina-sdk/wit/task/` directory

WIT and templates:
- Delete `wit/task/` directory
- Delete `resources/templates/child/task/` directory

Docs:
- Update `sdk/patina-sdk/README.md` — remove task from world features, stability table

### Phase 4 — Delete patina:host/* legacy package

Scope: repo-wide, both runtime WIT and SDK-local WIT snapshots.

- Delete `wit/deps/patina-host/` directory (runtime)
- Delete `sdk/patina-sdk/wit/mother-child/` directory (legacy SDK snapshot of retired `mother-child` world, still imports `patina:host/*`)
- Verify: `grep -r "patina:host/" wit/ sdk/` returns nothing
- Delete any `patina-host` dep entries in WIT package resolution files if present

### Phase 5 — Role, scaffold, and docs cleanup

- Update `ChildRole::Extension` allowed kinds (remove Command, Task references)
- Update `patina child init` (not `child new` — the CLI command is `Init`) help text and `--world` arg to only accept knowledge-child and pipeline
- Simplify SDK feature gate in `sdk/patina-sdk/src/lib.rs` (two worlds, not four)
- Update `sdk/patina-sdk/README.md`: remove command/task from world features list, stability table, breaking change notes, and migration guidance
- Update any scaffold test assertions that reference command/task kinds (`src/child/scaffold.rs` tests)

### Guardrails

1. **Pipeline must keep working.** Grammar plugins are production code. `patina bench grammar` and `patina scrape` with grammar children must pass before and after.
2. **`patina child run` dispatch must be updated.** Currently `patina child run` dispatches task, command, and knowledge-child — but rejects pipeline with "unsupported world." After this spec: knowledge-child arm stays, command/task arms removed. Pipeline support in `child run` is a non-goal (pipeline children are invoked by grammar/scrape subsystems, not by `child run` directly).
3. **No silent breakage of external children.** If any `child.toml` declares `kind = "command"` or `kind = "task"`, `ChildKind::from_str` must return a clear retired-kind error: `"child kind '{name}' is retired; migrate to 'knowledge-child'"`. Not a generic "unknown kind" error.
4. **Retired-kind error must be tested.** Add test cases in `src/child/internal/tests.rs` asserting the exact error message for `"command"` and `"task"` kind strings (same pattern as existing `"mother-child"` retired-kind test).

## Implementation Order

1. Pipeline log migration (unblocks Phase 4)
2. Command kind deletion
3. Task kind deletion
4. `patina:host/*` package deletion
5. Role and scaffold cleanup
6. Test cleanup and gate verification

## Resolved Decisions

- Delete rather than deprecate — these kinds have zero consumers and create naming confusion.
- Pipeline keeps its own kind — pure-compute plugins don't need daemon lifecycle.
- Add friendly error for retired kinds in manifest parsing — one release cycle of guidance.
- This spec lands before `child-command-surface` to remove the naming collision.

## Terminology Note

This spec uses "kind" (the manifest/Rust enum term) and "world" (the WIT composition term) interchangeably where context is clear. Post-retirement, the mapping is:

- `kind = "knowledge-child"` → WIT world `patina:knowledge-child`, SDK feature `knowledge-child`, `[needs].toys` for capability grants
- `kind = "pipeline"` → WIT world `patina:pipeline`, SDK feature `pipeline`, log-only

The retired kinds (`command`, `task`) used `patina:host/*` imports and did not participate in the `[needs].toys` grant model.

## Verification

```bash
cargo check --workspace -q
cargo test -q --workspace
patina bench grammar --help
patina child init --help
patina spec check scaffold-world-retirement --json
```

Post-deletion verification:

```bash
# No remaining patina:host/* imports (repo-wide)
grep -r "patina:host/" wit/ sdk/  # should return nothing

# No remaining command/task kind references in production code
grep -r "ChildKind::Command\|ChildKind::Task" src/  # should return nothing

# No remaining dead SDK features
grep -r 'feature = "command"\|feature = "task"' sdk/  # should return nothing

# Pipeline grammar plugins still work
patina bench grammar --help

# Retired-kind errors work
# (covered by unit tests, not CLI verification)
```

## Exit Criteria

See frontmatter `exit_criteria` (`swr0`-`swr14`, 15 total).

## Build Readiness

Ready for implementation. No blocking dependencies. Phases are independent and can be committed incrementally.
