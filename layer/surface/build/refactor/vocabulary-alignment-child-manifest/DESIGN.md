# Design: refactor: Vocabulary alignment for child/toy architecture

## Why This Design

Vocabulary is architecture. Leaving legacy `plugin/world` terminology in core paths keeps reintroducing old mental models. We need a migration that changes names and contracts without changing runtime behavior.

## Build Target

Canonical child-first naming (`Child*`, `kind`, `child.toml`) with compatibility bridge for legacy names (`Plugin*`, `world`, `plugin.toml`) until follow-on cleanup.

## Resolved Decisions

- Rename is mechanical + contract-level, not behavior-level.
- Keep dual-read compatibility during migration.
- WIT `world` naming remains untouched in component metadata (that is the correct WIT term).

## Commits
1. `rename: add child-first type aliases in runtime` — introduce `ChildManifest`/`ChildKind`/`ChildEngine` and keep `Plugin*` aliases to preserve compile/runtime behavior.
2. `manifest: add kind field with world fallback` — parse `kind` canonically, read legacy `world`, emit deprecation warning for legacy field use.
3. `manifest: support child.toml canonical path` — prefer `child.toml`, fallback to `plugin.toml` for compatibility.
4. `docs: migrate spec and guidance vocabulary` — update AGENTS/spec docs to reserve `world` for WIT contexts only.
5. `test: add vocabulary bridge regression checks` — prove both legacy and canonical forms load/link/instantiate identically.

## Direct Code Targets
- `src/plugin/internal/mod.rs`
- `src/plugin/internal/tests.rs`
- `src/commands/mother/daemon.rs`
- `children/*/plugin.toml` (bridge path)
- `children/template/plugin.toml`
- `AGENTS.md`
- `layer/surface/build/refactor/patina-pre-v1/SPEC.md`

## Verification Plan

- Run targeted manifest/linker tests:
  - `knowledge_child_example_manifests_validate`
  - `knowledge_child_linker_fails_when_lake_not_linked`
  - `knowledge_child_linker_succeeds_when_lake_declared`
- Run full `cargo test -q`.
- Run grep checks for ambiguous vocabulary drift in specs/docs.

## Build Readiness

Ready for execution as a bounded side quest blocker before pre-v1 final EC closure.

## Open Questions

- Whether canonical manifest filename migration (`child.toml`) should be completed in this spec or deferred to a follow-on once field/type rename is stable.
