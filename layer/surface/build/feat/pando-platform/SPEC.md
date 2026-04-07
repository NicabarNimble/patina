---
type: feat
id: pando-platform
status: active
created: 2026-04-06
sessions:
  origin: 20260405-133644-511306000
related:
- layer/surface/build/feat/pando-platform-phase-a/SPEC.md
- layer/surface/build/feat/slate-pando-migration/SPEC.md
- layer/surface/build/explore/mother-child-artifact-registry/SPEC.md
- resources/pandos/folder-text-to-parquet/pando.toml
- mother/src/pando.rs
- src/commands/pando.rs
- src/commands/mother/daemon.rs
beliefs:
- '[[pandos-are-products-children-are-compute]]'
- '[[pando-is-composed-children]]'
- '[[children-have-agency-toys-are-capabilities]]'
- '[[wasi-is-foundation-not-option]]'
- '[[children-are-portable-wasm-artifacts]]'
- '[[mother-manages-artifact-install-and-runtime]]'
- '[[pandos-are-shareable-compositions]]'
exit_criteria:
- id: pp5-folder-text-retrofit
  text: '`folder-text-to-parquet` has a `pando.toml` and is managed by Mother as a pando. No CLI commands — pipeline pando, proves basic pando lifecycle (register, list, health).'
  checked: true
- id: pp5c1-ready-live-lifecycle
  text: 'Pando lifecycle distinguishes readiness from runtime activity: `ready` means all required children are installed/resolvable; `live` means all required children are loaded in Mother. Missing children is not `error` for valid manifests.'
  checked: true
- id: pp5c2-first-party-child-artifact-install
  text: First-party child artifacts required by `folder-text-to-parquet` are installable into `~/.patina/children/` as compiled `.wasm + .toml` pairs, and Mother can transition the pando from `registered`/`ready` to `live` when artifacts are present and loaded.
  checked: true
- id: pp5c3-shared-artifact-composition-model
  text: 'Spec and design encode the portable artifact model: children are versioned reusable WASM artifacts, pandos are shareable compositions referencing those artifacts, and Mother separates artifact install/cache from runtime instances for future P2P sharing.'
  checked: true
---
# feat: Pando Platform

## Problem

Patina needed a real pando platform: manifest parsing, Mother-side registration,
collision checks, lifecycle projection, and a CLI verification surface for
commandless pipeline pandos.

## Goal

Deliver a production pando foundation and prove it with the canonical
`folder-text-to-parquet` pando using six reusable child artifacts.

## Status

Complete.

## Delivered

- Phase A foundations (in extracted spec): strict `pando.toml` parsing,
  Mother registry, collision policy, and pando list protocol surface.
- `folder-text-to-parquet` first-party pando seeding and registry visibility.
- Lifecycle semantics split into `registered`/`ready`/`live`/`degraded`/`error`.
- Canonical identity matching fix: installed children resolve via
  `child.toml` `[child].name` (not wasm filename stems).
- Release/runtime verification with six canonical children installed and healthy,
  resulting in `folder-text-to-parquet` lifecycle `live`.

## Verification

```bash
cargo check --workspace -q
cargo test -q --lib
patina mother status
patina pando list --json
```

Expected runtime outcome:

- Mother loads six canonical children as healthy.
- `patina pando list --json` shows `folder-text-to-parquet` with `status: "live"`.

## Follow-on Work

Slate migration and CLI command routing are now split into
`slate-pando-migration` so this spec remains platform-only and complete.
