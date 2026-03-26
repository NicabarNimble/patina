# Design: Consolidate workspace layout after architecture retirement

## Why This Design

The greenfield retirement arc left dead directories that make the workspace layout lie about what's active. This is a cleanup, not a restructure — we're removing what's already dead, not reorganizing what's alive.

## Build Target

Three deletions, one merge, one Cargo.toml edit. Low risk, high clarity gain.

## Resolved Decisions

- `children/` is canonical (not `plugins/`).
- `resources/scripts/` is the likely canonical script location (already has the guard scripts, grammar scripts, crate-name checks). `scripts/` needs audit to see what's there.
- `wit/mother-child/` has no consumers after MotherChild trait deletion.
- `plugins/doctor` is NOT a duplicate of `children/doctor` — they are completely different implementations:
  - `plugins/doctor`: WASM command child, 380 lines, full health check logic. But **orphan code** — not in execution path. Users get native `doctor_runtime`.
  - `children/doctor`: WASM knowledge-child stub, 37 lines. Different WIT world, different SDK features.
  - Safe to delete `plugins/doctor` — recoverable from git. Doctor WASM unification (making doctor run as WASM instead of native) is a separate future spec.

## Commits

1. `refactor(workspace): remove plugins/ directory` — Remove `plugins/doctor` from Cargo.toml workspace members. Delete entire `plugins/` tree. Clean stale references: `src/child/internal/tests.rs:1342` mentions `-p patina-ai-extension-doctor`. Check for any other refs to `patina-ai-extension-doctor` crate name or `plugins/` paths in docs.

2. `refactor(wit): remove dead mother-child WIT world` — Delete `wit/mother-child/`. Verify no Cargo.toml or build.rs references.

3. `refactor(scripts): unify script locations` — Audit `scripts/` vs `resources/scripts/`. Move unique scripts to canonical location. Delete empty dir. Update references in docs, CI, Cargo.toml.

## Direct Code Targets

- `Cargo.toml` — remove `plugins/doctor` from workspace members
- `plugins/` — entire directory deletion
- `wit/mother-child/` — directory deletion
- `scripts/` — contents audit, then merge or delete
- `resources/scripts/` — possible destination for merged scripts
- `src/child/internal/tests.rs:1342` — stale reference to `patina-ai-extension-doctor`
- `README.md`, `AGENTS.md` — update any directory tree references

## Pre-Flight Check

Doctor redundancy is RESOLVED — they are different implementations:
```bash
# Already verified:
# plugins/doctor = command child (WASM, orphan, not executed)
# children/doctor = knowledge-child stub (WASM, different world)
# Native doctor_runtime = what users actually run
# Conclusion: safe to delete plugins/doctor, no functionality lost
```

Before committing, grep for stale references:
```bash
rg "patina-ai-extension-doctor" src/ tests/ resources/
rg "plugins/doctor" . --glob '!plugins/**' --glob '!target/**' --glob '!layer/sessions/**'
```

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
