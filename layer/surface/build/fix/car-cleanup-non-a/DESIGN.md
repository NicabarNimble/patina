# Design: CAR Cleanup (non-A)

## Principle Alignment

- [[unix-philosophy]]: remove stale surfaces and deprecated flags.
- [[session-capture]] and [[spec-driven-design]]: archive stale specs using first-class lifecycle commands, not file deletes.

## Gate Details

### car-deprecated-serve

Delete `src/commands/serve/mod.rs` (empty since v0.12.0, now v0.45.1). Remove `pub mod serve` from `commands/mod.rs`. The `Serve` variant in `main.rs` already delegates to `commands::mother::daemon` with a deprecation warning — keep that delegation but remove the dead module.

### car-deprecated-flags

Remove `--legacy` and `--full` flags from `main.rs` Scry command definition. Delete backing code paths in `scry/mod.rs` (legacy search at lines 270-301) and `scry/internal/semantic.rs` (full content at lines 122,151). Both were declared deprecated at v0.12.0.

### car-upgrade-placeholder

Move `Commands::Upgrade` behind `#[cfg(feature = "dev")]` in `main.rs`. Move `commands/upgrade.rs` into `commands/dev/upgrade.rs`. Default builds will not expose a broken upgrade check pointing at `rust-lang/rust`.

### car-build-md-version

Update `layer/core/build.md` line 3 version to `0.45.1`. Add milestone entries for v0.24.0 through v0.45.1 to the milestone table. Ensure the architecture diagram and pipeline description still match current code (they do per the audit).

### car-agents-md-accuracy

Update AGENTS.md:
- Line 9: Remove `wit/command/`, `wit/task/` references. Change to: "Child world composition lives in `wit/knowledge-child/` and `wit/pipeline/` (per-kind)."
- Line 10: Remove SDK tier references. Change to: "SDK surface is `sdk/patina-sdk` (umbrella crate with inline toy types)."

### car-core-tools-decision

Delete `src/core_tools/mod.rs`. Remove `pub mod core_tools` from `src/lib.rs`. The 8-line re-export of `spec` is premature — `spec` is already accessible as `crate::spec`.

### car-stale-specs-archived

Archive 18 completed/abandoned specs via `patina spec complete` / `patina spec abandon` workflow (14 complete, 4 abandoned). Each gets a `spec/<id>` git tag and removal from the working tree.

## Strategy

- Do cleanup as final pass after behavior and architecture are settled.
- Keep each cleanup in a narrow commit.
- Use official spec lifecycle workflow for archival operations (not raw `git rm`).

## Verification

- `cargo check --workspace -q`
- `patina --help` — verify serve is gone, upgrade is gone from default build
- `patina scry --help` — verify --legacy and --full are gone
- `patina spec list` — verify stale specs are archived (count decreases)
- Read AGENTS.md and build.md — verify accuracy

## Out of Scope

- Any safety fixes, inversion fixes, or dead-code gate work.
