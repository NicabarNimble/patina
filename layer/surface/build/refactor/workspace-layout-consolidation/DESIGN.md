# Design: Consolidate workspace layout after architecture retirement

## Why This Design

The greenfield retirement arc left dead directories that make the workspace layout lie about what's active. This is a cleanup, not a restructure — we're removing what's already dead, not reorganizing what's alive.

## Build Target

Three deletions, one merge, one Cargo.toml edit. Low risk, high clarity gain.

## Resolved Decisions

- `children/` is canonical (not `plugins/`).
- `resources/scripts/` is the likely canonical script location (already has the guard scripts, grammar scripts, crate-name checks). `scripts/` needs audit to see what's there.
- `wit/mother-child/` has no consumers after MotherChild trait deletion.

## Commits

1. `refactor(workspace): remove dead plugins/ directory` — Verify plugins/doctor redundancy with children/doctor. Remove `plugins/doctor` from Cargo.toml workspace members. Delete `plugins/` tree. Update `plugins/README.md` references if any remain elsewhere.

2. `refactor(wit): remove dead mother-child WIT world` — Delete `wit/mother-child/`. Verify no Cargo.toml or build.rs references.

3. `refactor(scripts): unify script locations` — Audit `scripts/` vs `resources/scripts/`. Move unique scripts to canonical location. Delete empty dir. Update references in docs, CI, Cargo.toml.

## Direct Code Targets

- `Cargo.toml` — remove `plugins/doctor` from workspace members
- `plugins/` — entire directory deletion
- `wit/mother-child/` — directory deletion
- `scripts/` — contents audit, then merge or delete
- `resources/scripts/` — possible destination for merged scripts
- `README.md`, `AGENTS.md` — update any directory tree references

## Pre-Flight Check

Before committing, verify `plugins/doctor` vs `children/doctor`:
```bash
diff -rq plugins/doctor/src children/doctor/src
diff plugins/doctor/Cargo.toml children/doctor/Cargo.toml
diff plugins/doctor/child.toml children/doctor/child.toml
```

If they differ, reconcile before deleting. If `children/doctor` is the superset, safe to delete `plugins/doctor`.

## Verification Plan

```bash
cargo check --workspace -q
cargo test -q
bash resources/scripts/check-plugin-vocab-guard.sh
# Confirm deleted dirs are gone:
test ! -d plugins && echo "plugins: gone" || echo "FAIL: plugins still exists"
test ! -d wit/mother-child && echo "wit/mother-child: gone" || echo "FAIL: wit/mother-child still exists"
```

## Build Readiness

Ready to execute. No dependencies. No blockers. Can be done in a single session.

## Open Questions

- Which script location wins: `scripts/` or `resources/scripts/`? Needs audit of `scripts/` contents.
- Are there any external tools or CI that reference `plugins/` paths? (Likely not — the workspace already builds without them except `plugins/doctor`.)
