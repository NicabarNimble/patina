# Design: scaffold world retirement

## Why This Design

The Patina child architecture converged on two models:

- **knowledge-child** — daemon-resident workers with toys, events, and Mother lifecycle. All 7 production children use this kind. This is the stabilization target.
- **pipeline** — pure-compute plugins for grammars and tokenizers. Log-only, no daemon. Used by `scrape` and `bench` commands.

The `command` and `task` kinds were speculative scaffolding from an earlier three-way split hypothesis (query-only / query+act / pure-compute). That hypothesis was overtaken when knowledge-child proved sufficient for all persistent workloads. The SDK itself labels them "migration scaffolds."

Keeping dead kinds creates three problems:

1. **Naming collision.** `wit/command/command.wit` defines a one-shot CLI plugin. The `child-command-surface` spec uses "command" to mean knowledge-children owning CLI command surfaces. Different concept, same word, guaranteed confusion for build agents and humans.
2. **Stale import namespace.** The dead worlds import from `patina:host/*` (pre-collapse monolithic bundle). The live world imports from per-package toys (`patina:connect/connect`, etc.). Two import lineages in one codebase is confusing.
3. **False optionality.** Four kinds in the SDK suggest four viable paths. Only two are real. New children should not have to evaluate dead options.

## Build Target

1. Migrate pipeline's log import to per-package convention.
2. Delete `command` kind: engine, SDK, WIT world, templates.
3. Delete `task` kind: engine, SDK, WIT world, templates.
4. Delete `patina:host/*` legacy WIT package.
5. Clean up role mappings and scaffold CLI.

## Resolved Decisions

- Delete, don't deprecate. Zero consumers means zero migration cost.
- One release cycle of friendly "kind retired" error for manifests that reference dead kinds.
- Pipeline keeps its own kind. Pure-compute plugins don't benefit from daemon lifecycle.
- This spec is a prerequisite for `child-command-surface` — clearing naming collision before that work starts.

## Commits

1. `refactor(pipeline): migrate log import to patina:log/log` — WIT (runtime + SDK snapshot), host engine binding path, SDK guest re-export path, binding regeneration.
2. `refactor(child): remove command kind` — engine, SDK module, SDK WIT snapshot, runtime WIT world, templates, tests, docs.
3. `refactor(child): remove task kind` — same pattern.
4. `refactor(wit): delete patina:host legacy package` — runtime WIT, SDK mother-child snapshot, no remaining consumers.
5. `refactor(child): clean up role mappings and scaffold` — Extension role, `patina child init` help, SDK feature gates, README.

## Direct Code Targets

Runtime WIT:
- `wit/pipeline/pipeline.wit` — import path change.
- `wit/command/` — delete directory.
- `wit/task/` — delete directory.
- `wit/deps/patina-host/` — delete directory.

SDK WIT snapshots:
- `sdk/patina-sdk/wit/pipeline/pipeline.wit` — import path change.
- `sdk/patina-sdk/wit/command/` — delete directory.
- `sdk/patina-sdk/wit/task/` — delete directory.
- `sdk/patina-sdk/wit/mother-child/` — delete directory (legacy, still imports patina:host/*).

Host engines:
- `src/child/internal/command.rs` — delete (226 lines).
- `src/child/internal/task.rs` — delete (314 lines).
- `src/child/internal/pipeline.rs` — update host binding path for log import.
- `src/child/internal/mod.rs` — remove Command/Task from ChildKind enum + allowed_capabilities. Add retired-kind errors. Update ChildRole::Extension.
- `src/main.rs` — remove Command/Task match arms in `patina child run` dispatch.
- `src/child/scaffold.rs` — remove command/task scaffold logic and tests.

SDK:
- `sdk/patina-sdk/src/command.rs` — delete (232 lines).
- `sdk/patina-sdk/src/task.rs` — delete (263 lines).
- `sdk/patina-sdk/src/pipeline.rs` — update guest re-export path for log.
- `sdk/patina-sdk/src/lib.rs` — remove command/task features, simplify exclusion.
- `sdk/patina-sdk/Cargo.toml` — remove command/task features.

Templates:
- `resources/templates/child/command/` — delete directory.
- `resources/templates/child/task/` — delete directory.

Tests and docs:
- `src/child/internal/tests.rs` — remove command/task test cases, add retired-kind error tests.
- `sdk/patina-sdk/README.md` — update world features, stability table, breaking change notes.

## Verification Plan

Core gates:

```bash
cargo check --workspace -q
cargo test -q --workspace
```

Pipeline survival checks:

```bash
patina bench grammar --help
# grammar plugins load and execute via PipelineEngine
```

Deletion completeness:

```bash
# No patina:host/* imports remain (repo-wide)
grep -r "patina:host/" wit/ sdk/

# No dead kind references remain
grep -r "ChildKind::Command\|ChildKind::Task" src/

# No dead SDK features remain
grep -r 'feature = "command"\|feature = "task"' sdk/
```

## Sequencing

This spec is a prerequisite for `child-command-surface`. After this lands:
- `wit/command/` will not exist — `child-command-surface` must use a new path (e.g., `wit/command-handler/` or extend `wit/knowledge-child/`) for its command surface WIT contract.
- The word "command" in the codebase will refer only to CLI command surfaces, not to the retired one-shot child kind.
- Two child kinds remain: `knowledge-child` and `pipeline`.

## Build Readiness

Ready to execute. Each phase is a clean commit. No blocking decisions remain.

## Open Questions

None. Scope is clear and bounded.
