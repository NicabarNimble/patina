---
type: fix
id: sdk-readme-and-template-consolidation
status: complete
created: 2026-04-08
sessions:
  origin: 20260408-064526-677971000
related:
- sdk/patina-sdk/
- children/template/
exit_criteria:
  - id: srtc1
    text: "`children/template/` moved to `sdk/template/` and workspace Cargo.toml updated"
    checked: true
  - id: srtc2
    text: "SDK README references updated to point to `sdk/template/` path"
    checked: true
  - id: srtc3
    text: "SDK README stale content fixed: remove dead tier crate references (patina-sdk-core, patina-sdk-data, patina-sdk-agent), update toybox index to match current toys, remove stale task/command feature flags if they don't exist"
    checked: true
  - id: srtc4
    text: "Template Cargo.toml dependency path updated for new location"
    checked: true
  - id: srtc5
    text: "`cargo check --workspace -q` and `cargo test -q --lib` pass"
    checked: true
---
# fix: Consolidate SDK template and fix stale README

> Move children/template/ into sdk/, fix stale SDK README content

## Problem

The child template lives in `children/template/` alongside real children,
which is confusing. It's the SDK's onramp (`cargo generate`) and should
live under `sdk/`. The SDK README also has stale content from before
the single-crate consolidation: references to tier crates that don't
exist, a toybox index with retired toys, and feature flags that may
not exist.

## Fix

1. `git mv children/template/ sdk/template/`
2. Update template Cargo.toml paths (patina-sdk dep, WIT target)
3. Update SDK README: fix `cargo generate` path, remove dead tier refs,
   update toybox index, verify feature flags match Cargo.toml
4. Update workspace Cargo.toml member path

## Non-Goals

- No template content changes beyond path fixes.
- No SDK API changes.
