---
type: fix
id: spec-complete-archives
status: ready
created: 2026-02-14
beliefs:
- unix-philosophy
- dependable-rust
related:
- layer/surface/build/fix/spec-tree-and-cycles/SPEC.md
---

# fix: Collapse Spec Complete + Archive into One Command

> Completing a spec and archiving it are the same conceptual action — "this
> work is done." Splitting them into two commands creates a manual hoop that
> gets forgotten. Evidence: 7 completed specs sat unarchived in the tree
> until manually cleaned up. The system should enforce the lifecycle it
> defines.

## Problem

`patina spec status <id> complete` does the release (version bump, commit,
tag) but leaves the spec file in the tree. `patina spec archive <id>` must
be run separately to tag `spec/<id>` and remove the file. In practice this
second step is forgotten, leading to:

- Completed specs cluttering `patina spec list` output
- `patina spec ready` showing stale context
- Manual cleanup sessions instead of automated lifecycle
- Drift between spec status and tree reality

The archive step has a **clean tree requirement** that creates a chicken-and-egg
problem when other work is in progress — you either commit unrelated WIP or
stash, archive, pop. This friction is why it gets skipped.

## Design

### Principle: One Action = One Concept

When `patina spec status <id> complete` runs, it should do everything:

1. Update frontmatter status to `complete` (existing)
2. Update database status (existing)
3. Run release preflight + execute (existing)
4. **NEW**: Create `spec/<id>` annotated git tag
5. **NEW**: `git rm -r` the spec directory
6. **NEW**: Fold the archive into the release commit (or create a
   follow-up commit if no release needed)

### Git Commit Strategy

Today's flow creates **two commits**:
```
release: v0.22.0 — feat: Some Feature     ← complete
docs: archive spec/some-feature            ← archive (separate, manual)
```

The new flow creates **one commit** when there's a release:
```
release: v0.22.0 — feat: Some Feature     ← complete + archive in one
```

The release commit already stages `Cargo.toml` + `spec_path`. We extend it
to also `git rm -r` the spec directory before committing. The `spec/<id>`
tag is created after the commit (same as the `v{version}` tag).

When there's **no release** (explore/design specs), the archive still
creates its own commit:
```
docs: archive spec/some-exploration        ← archive-only commit
```

### Escape Hatch: `--no-archive`

```
patina spec status <id> complete --no-archive
```

Preserves current behavior — complete + release without removing the spec
file. Useful when:
- You want to keep the spec in-tree for reference during a multi-spec push
- CI or tooling depends on the spec file existing post-complete
- You're completing without archiving intentionally

Default is **archive on complete**. The escape hatch is opt-out, not opt-in.

### Abandoned Specs

`patina spec status <id> abandoned` should also auto-archive. Abandoned
work is still "done" — it just didn't ship. Same lifecycle: tag, remove,
commit. No version bump for abandoned specs (already the case — abandoned
has no BumpType mapping).

### Clean Tree Handling

The clean tree requirement moves **inside** the complete flow. Today:
- `update_spec_status` writes the frontmatter (dirties tree)
- `execute_cargo` stages Cargo.toml + spec file, commits (cleans tree)
- Archive requires clean tree (already clean after release commit)

After this fix:
- `update_spec_status` writes frontmatter (dirties tree)
- `execute_cargo` stages Cargo.toml + `git rm -r spec_dir`, commits, tags
- Archive tag `spec/<id>` created on same commit — no second commit needed
- For no-release specs: `git rm -r spec_dir` + commit in one step

The clean tree check in `archive_spec()` becomes unnecessary when called
from the complete flow because the flow itself manages staging.

### Bulk Archive: `patina spec archive --stale`

For the current backlog (and future drift), add:
```
patina spec archive --stale
```

Finds all specs with status `complete` or `abandoned` that still have files
in the tree and archives them in sequence. Each gets its own tag and commit
(reusing existing `archive_spec` logic in a loop).

---

## Files to Change

```
# Modified — collapse complete + archive
src/commands/spec/mod.rs              # Add --no-archive flag to Status variant
src/commands/spec/internal.rs         # update_spec_status calls archive after release
                                      # archive_spec gets internal entry point (no clean-tree check)

# Modified — release includes spec removal
src/release/internal.rs               # execute_cargo stages git rm before commit
src/release/mod.rs                    # PreparedRelease carries spec_dir for removal

# Modified — CLI dispatch
src/main.rs                           # Pass --no-archive flag through
```

---

## Build Order

1. **Add `--no-archive` / `--stale` CLI flags** — wire through clap, no behavior change yet.
2. **Refactor `archive_spec` internals** — extract `archive_spec_inner()` that
   skips clean-tree check (for use from complete flow). Public `archive_spec()`
   keeps the check (for standalone `patina spec archive` use).
3. **Integrate archive into release flow** — `execute_cargo` receives spec dir,
   does `git rm -r` before staging, `spec/<id>` tag after version tag.
4. **Handle no-release archive** — for explore/design/abandoned specs, archive
   creates its own commit (reuse `archive_spec_inner`).
5. **Implement `--stale` bulk archive** — loop over completed+unarchived, call
   archive for each.
6. **Tests + pre-push** — unit tests for flag parsing, integration test for
   the full complete→archive flow.

Target: 4-5 commits. Each passes `cargo test`.

---

## Exit Criteria

### Critical
- [ ] `patina spec status <id> complete` auto-archives (tag + remove) by default
- [ ] Release commit includes spec file removal (one commit, not two)
- [ ] `spec/<id>` tag created on the release commit
- [ ] `--no-archive` flag preserves old behavior (complete without archive)

### Important
- [ ] `patina spec status <id> abandoned` also auto-archives
- [ ] `patina spec archive --stale` cleans up completed-but-unarchived specs
- [ ] Standalone `patina spec archive <id>` still works (for manual use)
- [ ] No-release specs (explore/design) get archive-only commit

### Pre-push
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace`
- [ ] `cargo test --workspace`
- [ ] Existing spec commands still work (no regressions)
