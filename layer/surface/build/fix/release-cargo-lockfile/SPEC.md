---
type: fix
id: release-cargo-lockfile
status: active
created: 2026-02-25
sessions:
  origin: 20260225-071955
---
# fix: Cargo release pipeline doesn't manage Cargo.lock

> execute_cargo bumps Cargo.toml but never regenerates or stages Cargo.lock, leaving it dirty after every spec complete

## Problem

Every `spec complete` on a Cargo project leaves `Cargo.lock` dirty. The
release pipeline bumps `Cargo.toml` version but never updates the lockfile,
so it still references the old version. This causes:

1. **Dirty tree after every release** — next `spec complete` fails the
   safeguard clean-tree check on `Cargo.lock` unless manually committed
2. **spec-complete-atomicity was a symptom** — both failures in session
   20260224-202650 were caused by dirty `Cargo.lock`, not by a missing
   DB rollback (though the rollback fix was independently correct)
3. **Manual workaround tax** — "always commit Cargo.lock before spec
   complete" is documented tribal knowledge, not enforced

## Root Cause

`execute_cargo` in `src/release/internal.rs` treats `Cargo.toml` as the
only artifact of a version change. The pipeline:

1. `update_cargo_version()` — string-replaces version in Cargo.toml
2. `git add Cargo.toml [spec_path]` — stages only Cargo.toml
3. `git commit` + `git tag`

Step 1 changes `Cargo.toml` but `Cargo.lock` still pins the old version.
Step 2 doesn't know `Cargo.lock` exists. The lockfile becomes a stale
derived artifact that drifts on every release.

The safeguard `run_safeguard_checks` excludes only `spec_path` from
the dirty-tree check — it has no concept of "files the release pipeline
will touch."

## Fix

Add lockfile regeneration to `execute_cargo` between version bump and
staging:

1. After `update_cargo_version(new_version)`, run `cargo update
   --workspace` to regenerate `Cargo.lock` with the new version.
   This is the minimal cargo command that updates only workspace
   member versions — no dependency resolution changes.
2. Add `Cargo.lock` to both `git add` calls (archive mode and normal mode).
3. Add `Cargo.lock` to the safeguard's dirty-tree exclusion list alongside
   `spec_path` — a dirty lockfile is expected when Cargo.toml has been
   modified by a prior incomplete release, and the pipeline will overwrite
   it anyway.

All changes in `src/release/internal.rs`.

## Key Files

```
src/release/internal.rs  — execute_cargo, run_safeguard_checks
src/release/mod.rs       — public interface (no changes needed)
```

## Exit Criteria

- [ ] `spec complete` on a fix spec produces a release commit containing
      both `Cargo.toml` and `Cargo.lock` with matching versions
- [ ] `Cargo.lock` is clean after the release commit (no drift)
- [ ] Running `spec complete` twice in a row (two specs) works without
      manual Cargo.lock cleanup between them
- [ ] Safeguard check does not reject a dirty `Cargo.lock` when
      `Cargo.toml` is also dirty (both are release-managed files)
