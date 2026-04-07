# Design: pando-platform

## Why This Design
- Phase A shipped in `pando-platform-phase-a`; this design revision targets Phase C only (`pp5`) and leaves Phase B routing untouched.
- First-party pando retrofit is implemented as manifest seeding plus existing registry evaluation, keeping changes local to the pando command bootstrap path.
- Verification remains `patina pando list` so lifecycle visibility is proven without adding command dispatch behavior.

## Build Target
- Implement PP5 by introducing a first-party `folder-text-to-parquet` `pando.toml` and ensuring Mother lists it as a commandless pipeline pando with lifecycle status and child count.

## Resolved Decisions
- Canonical pando namespace is `folder-text-to-parquet`.
- Manifest source of truth is a repo resource file that gets seeded to `~/.patina/pandos/folder-text-to-parquet/pando.toml` at startup.
- Seeding is manifest-only and must not modify `state/` or `data/` directories.
- For commandless pandos, list output omits the commands column when no registered pando has commands.

## Commits
1. `spec(pando-platform): lock Phase C retrofit design — PP5` — lock concrete targets and verification for `folder-text-to-parquet` retrofit.
2. `feat(pando-platform): seed folder-text-to-parquet first-party pando — PP5` — add manifest resource, seeding path, and commandless list behavior.
3. `spec(pando-platform): check pp5 folder-text retrofit — PP5` — mark criterion complete after verification.

## Direct Code Targets
- `resources/pandos/folder-text-to-parquet/pando.toml` — first-party commandless pipeline manifest.
- `src/commands/pando.rs` — first-party pando seeding helper/tests and commandless list formatting logic.

## Verification Plan
- After each commit: `cargo check --workspace -q` and `cargo test -q --lib`.
- Unit checks: seeded manifest exists; seeding overwrites manifest while preserving `state/` and `data/` directories.
- Runtime checks: restart Mother, run `patina pando list` and `patina pando list --json`, verify `folder-text-to-parquet` appears with `child_count = 6` and lifecycle status.

## Build Readiness
- Ready. Existing PP1/PP2 parser/registry surfaces already support commandless manifest registration and lifecycle projection.

## Open Questions
- None for PP5 scope.
