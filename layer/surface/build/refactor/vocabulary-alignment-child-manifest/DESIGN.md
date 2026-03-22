# Design: refactor: Vocabulary alignment for child/toy architecture

## Why This Design

Vocabulary is architecture. Leaving legacy `plugin/world` terminology in core paths keeps reintroducing old mental models. We need a migration that changes names and contracts without changing runtime behavior.

## Build Target

Canonical child-first naming (`Child*`, `kind`, `child.toml`) with compatibility bridge for legacy names (`Plugin*`, `world`, `plugin.toml`) until follow-on cleanup.

## Resolved Decisions

- Rename is mechanical + contract-level, not behavior-level.
- Keep dual-read compatibility during migration.
- WIT `world` naming remains untouched in component metadata (that is the correct WIT term).
- Canonical runtime key is `kind`; legacy `world` is read-only compatibility.
- Canonical manifest filename is `child.toml`; legacy `plugin.toml` is read-only compatibility.
- This spec completes canonical file/key migration (no deferral to follow-on).

## Commits
1. `rename: add child-first type aliases in runtime` — introduce `ChildManifest`/`ChildKind`/`ChildEngine` and keep `Plugin*` aliases to preserve compile/runtime behavior.
2. `manifest: add kind field with world fallback` — parse `kind` canonically, read legacy `world`, emit exact warning `deprecated manifest key 'world'; use 'kind'`.
3. `manifest: support child.toml canonical path` — prefer `child.toml`, fallback to `plugin.toml`, emit exact warning `deprecated manifest filename 'plugin.toml'; use 'child.toml'` when fallback is used.
4. `docs: migrate spec and guidance vocabulary` — update AGENTS/spec docs to reserve `world` for WIT contexts only.
5. `test: add vocabulary bridge regression checks` — prove both legacy and canonical forms load/link/instantiate identically.

## Direct Code Targets
- `src/plugin/internal/mod.rs`
- `src/plugin/internal/tests.rs`
- `src/commands/mother/daemon.rs`
- `children/*/child.toml` (canonical path)
- `children/template/child.toml`
- `AGENTS.md`
- `layer/surface/build/refactor/patina-pre-v1/SPEC.md`

## Verification Plan

- Run targeted manifest/linker tests:
  - `knowledge_child_example_manifests_validate`
  - `knowledge_child_linker_fails_when_lake_not_linked`
  - `knowledge_child_linker_succeeds_when_lake_declared`
- Run full `cargo test -q`.
- Run grep checks for ambiguous vocabulary drift in specs/docs.

Grep gates:

- `grep -R "PluginManifest\|PluginWorld\|PluginRole" layer/surface/build/refactor -n` -> no new public-facing spec/docs usage from this spec.
- `grep -R "world = \"knowledge-child\"" children/template/child.toml -n` -> zero matches after scaffold migration (`Cargo.toml` WIT world entries remain valid).
- `grep -R "plugin.toml" children/template -n` -> zero canonical write references (compat-read references allowed in runtime loader code only).

## Build Readiness

Ready for execution as a bounded side quest blocker before pre-v1 final EC closure.

## Open Questions

- None for execution. Remaining choices are implementation details inside the locked bridge policy.
