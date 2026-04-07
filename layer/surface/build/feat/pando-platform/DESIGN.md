# Design: pando-platform

## Why This Design

- Keep this spec focused on pando platform foundations and canonical pipeline proof.
- Separate slate migration/routing into its own spec so platform completion stays crisp.

## Build Target

- Deliver and verify commandless pando lifecycle foundations with
  `folder-text-to-parquet` as the canonical proof.

## Delivered Scope

- Phase A parser/registry foundations (tracked in extracted Phase A spec).
- First-party pando seeding for `folder-text-to-parquet`.
- Lifecycle semantics (`registered`/`ready`/`live`/`degraded`/`error`).
- Installed-child identity correction to canonical `[child].name`.
- Runtime verification to `live` with six installed canonical children.

## Key Code Targets (Delivered)

- `mother/src/pando.rs`
- `src/commands/mother/daemon.rs`
- `src/commands/pando.rs`
- `resources/pandos/folder-text-to-parquet/pando.toml`
- `crates/patina-protocol/src/lib.rs`

## Verification Snapshot

- `cargo check --workspace -q`
- `cargo test -q --lib`
- `patina mother status` shows six healthy canonical children
- `patina pando list --json` shows `folder-text-to-parquet` as `live`

## Follow-on

Routing and slate migration continue under
`layer/surface/build/feat/slate-pando-migration/SPEC.md`.
