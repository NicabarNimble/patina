---
type: refactor
id: workspace-layout-consolidation
status: complete
created: 2026-03-25
sessions:
  origin: 20260325-150227-161735000
related:
- Cargo.toml
- plugins/
- children/
- wit/mother-child/
- scripts/
- resources/scripts/
exit_criteria:
- id: wlc1-plugins-removed
  text: '`plugins/` directory deleted entirely — zero files on disk. `children/` is the sole home for in-tree WASM children.'
  checked: true
- id: wlc2-workspace-clean
  text: Cargo.toml workspace members reference only `children/*`, `crates/*`, `sdk/*`, `mother` — no `plugins/*` entries.
  checked: true
- id: wlc3-dead-wit-removed
  text: '`wit/mother-child/` deleted. All tooling/script references (`resources/git/pre-push-checks.sh` world loop, WIT mirror checks) updated or removed. No WIT world references a deleted runtime path.'
  checked: true
- id: wlc4-scripts-unified
  text: One script location exists (`resources/scripts/` or `scripts/`, not both). All references updated.
  checked: true
- id: wlc5-builds-pass
  text: '`cargo check --workspace`, `cargo test -q`, and `check-plugin-vocab-guard.sh` all pass.'
  checked: true
---
# refactor: Consolidate workspace layout after architecture retirement

> Remove dead directories left behind by the greenfield/PVR retirement arc. Make the physical layout match the architecture.

## Problem

The greenfield spec arc (vocabulary-alignment, greenfield-mother-rebuild, greenfield-mother-clean-continued, sdk-mother-child-retirement, plugin-vocabulary-retirement) deleted runtime code and migrated vocabulary, but left dead directories on disk:

- `plugins/` contains 6 subdirectories. Only `plugins/doctor` is a workspace member. `plugins/belief-verifier`, `plugins/ducklake` are empty husks (just `src/`, no Cargo.toml). `plugins/models` and `plugins/repos` are dead (just `src/` + dangling wit symlink). `plugins/sdk` is a lone wit symlink. Meanwhile, `children/` has the canonical versions of all active children.
- `wit/mother-child/` defines a WIT world for a runtime path (`MotherChild` trait) that no longer exists.
- `scripts/` and `resources/scripts/` are two script locations with unclear split.

This creates confusion about what's live vs. dead and where new work should go.

## Goal

Make the workspace directory tree honest: every directory that exists should contain live, referenced code. Remove dead paths. Unify duplicates.

## Status

Draft. Truth audit complete for doctor execution path, orphan plugin directories, `wit/mother-child` tooling references, and script-location split. Ready to promote to active for execution.

## Non-Goals

- Do NOT restructure `src/` internals (splitting the CLI monolith is a separate effort).
- Do NOT move `grammars/` — they're legitimately separate crates.
- Do NOT restructure `layer/` — the knowledge layer has its own conventions.
- Do NOT extract children to separate repos yet — that's a future effort. `children/` stays in-tree.
- Do NOT rename identifiers (e.g., `plugin_name` → `child_name`) — that's deferred from PVR.
- Do NOT unify doctor into a single WASM-canonical form — that's a separate feature spec. Doctor currently runs via native `doctor_runtime`, not WASM.

## Current State

```
patina/
├── children/           # 7 active WASM children (canonical, in workspace)
├── plugins/            # 6 dirs: 1 workspace member, 3 husks, 2 dead
│   ├── doctor/         # workspace member — ORPHAN: real WASM command child (380 lines)
│   │                   #   but NOT in execution path. Users get native doctor_runtime.
│   │                   #   children/doctor is a separate knowledge-child stub.
│   ├── belief-verifier/ # empty husk (just src/)
│   ├── ducklake/       # empty husk (just src/)
│   ├── models/         # dead (src/ + dangling wit symlink)
│   ├── repos/          # dead (src/ + dangling wit symlink)
│   └── sdk/            # lone wit symlink
├── wit/
│   ├── mother-child/   # dead WIT world (MotherChild trait deleted)
│   └── ...             # 7 other live WIT worlds
├── scripts/            # some scripts here
├── resources/scripts/  # other scripts here
└── ...
```

## Target State

```
patina/
├── children/           # sole home for in-tree WASM children
├── wit/                # only live WIT worlds remain
├── resources/scripts/  # unified script location
└── ...                 # everything else unchanged
```

## Solution

Three independent deletions plus one merge:

1. **Delete `plugins/` entirely.** `plugins/doctor` is an orphan WASM command child — real code but not in the execution path (users get native `doctor_runtime`). `children/doctor` is a separate knowledge-child stub, NOT a duplicate. Neither depends on `plugins/doctor`. Remove `plugins/doctor` from Cargo.toml workspace members. Delete the entire `plugins/` tree. Clean up stale references (e.g., `src/child/internal/tests.rs:1342` mentions `-p patina-ai-extension-doctor`).

2. **Delete `wit/mother-child/`.** Verify no remaining references across runtime, Cargo/deps, imports, and tooling/scripts (including pre-push mirror checks), then delete.

3. **Unify scripts.** Determine which location (`scripts/` or `resources/scripts/`) should be canonical. Move any unique scripts from the other. Delete the empty one. Update any references (Cargo.toml, CI, docs).

4. **Verify.** Full workspace build, test, guard script.

## Implementation Order

Gate-free — these are independent deletions. Can be done in 1-3 commits. Suggested order:

1. `plugins/` removal (highest confusion surface)
2. `wit/mother-child/` removal (smallest change)
3. Script unification (needs reference audit)

## Resolved Decisions

- `children/` is the canonical home for in-tree WASM children (established by vocabulary-alignment and PVR specs).
- Children will eventually move to their own repos; `children/` makes that migration path obvious.
- `plugins/doctor` is orphan code: real WASM command child logic (~380 lines) but never wired into the execution path. Users run native `doctor_runtime`. Safe to delete — recoverable from git history. Doctor WASM unification is a separate future spec.
- `children/doctor` (knowledge-child stub) stays — it's a different child kind serving a different purpose.

## Verification

```bash
cargo check --workspace -q
cargo test -q
bash resources/scripts/check-plugin-vocab-guard.sh
ls plugins/ 2>&1  # should fail: No such file or directory
ls wit/mother-child/ 2>&1  # should fail: No such file or directory
```

## Build Readiness

Ready for execution. Doctor execution-path truth already verified (native `doctor_runtime` is what runs; `plugins/doctor` is orphan WASM). No blockers.
