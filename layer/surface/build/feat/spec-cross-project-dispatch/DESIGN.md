# Design: Spec cross-project dispatch

## Why This Design

We need immediate multi-project operator capability without breaking current spec workflow guarantees:
- specs remain git-tracked files in target repositories,
- all spec lifecycle commands can be routed cross-project,
- create path remains safe by default (session lock),
- provenance is explicit (`origin-project:<uid>`).

## Build Target

Implement cross-project routing for the full `patina spec` command family via a top-level project selector.

## Resolved Decisions

1. **Target selection**: `patina spec --project` accepts either absolute/relative path or registered project uid.
2. **Source of truth**: keep SPEC.md files in target repo (`layer/surface/build/...`).
3. **Safety gate**: cross-project create requires active session in target project unless `--force-cross-project`.
4. **Provenance**: cross-project create appends `origin-project:<uid>` to `related`.
5. **Execution strategy**: route non-create commands through a subprocess pinned to target project cwd with direct spec execution mode (no Mother recursion).

## Direct Code Targets

- `src/main.rs`
  - add top-level `patina spec --project` selector
- `src/spec.rs`
  - add route context support (`execute_command_value_with_route`)
  - resolve target project/uid and dispatch non-create commands into target context
  - preserve create guardrails/provenance behavior
- `src/commands/spec/mod.rs`
  - send spec-dispatch envelope (`command + project + origin_project`)
  - add direct mode for subprocess execution (`PATINA_SPEC_DIRECT`)
- `src/commands/mother/daemon.rs`
  - parse envelope and call routed spec execution
- `src/commands/spec/internal/create.rs`
  - target-aware create write + `git -C` commit
  - update target project `patina.db` path via `resolve_patina_db_path`
- `src/commands/spec/internal/mutations.rs`
  - stage spec files with `git add -f` for compatibility with broad ignore patterns

## Verification Plan

1. CLI parse test for create cross-project flags.
2. Run targeted compile/tests for spec command modules.
3. Manual smoke in two local Patina projects:
   - `spec --project <target> list/show/check` returns target state,
   - `spec --project <target> set ...` commits in target repo,
   - `spec create --project <target>` denies without active session unless forced.

## Build Readiness

- MVP complete for cross-project query + mutation routing via top-level project selector.
- Future slice can harden cross-project policy semantics uniformly across all mutating commands.

## Open Questions

- Should we persist cross-project spec index into Mother state tables now, or wait for Slate migration?
- Should target-session lock become a shared policy helper reused by all cross-project mutating commands (beyond create)?
