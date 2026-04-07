---
type: feat
id: slate-pando-migration
status: draft
created: 2026-04-07
parent: pando-platform
beliefs:
  - "[[pandos-are-products-children-are-compute]]"
  - "[[children-have-agency-toys-are-capabilities]]"
related:
  - layer/surface/build/feat/pando-platform/SPEC.md
  - children/spec-manager/
  - mother/src/builtin_children.rs
  - src/commands/spec/
  - src/main.rs
exit_criteria:
  - id: sp1-cli-command-discovery
    text: "The `patina` binary asks Mother for registered pando commands. Unknown commands route to Mother for pando dispatch. `patina --help` shows native commands; `patina <pando> --help` shows pando commands served from the manifest."
    checked: false

  - id: sp2-pando-to-child-dispatch
    text: "Mother receives a pando command, resolves which child handles the action, calls `handle(action, payload)` on that child, and returns the result to the CLI."
    checked: false

  - id: sp3-slate-child-built
    text: "Slate-manager child exists as a proper WASM child using the SDK and handles spec lifecycle actions through toy boundaries."
    checked: false

  - id: sp4-slate-pando-commands
    text: "Slate pando has `pando.toml` command declarations and works end-to-end via pando dispatch (`patina slate list`, `next`, `complete`, `archive`)."
    checked: false

  - id: sp5-git-toy-additions
    text: "`patina:git` interface includes required additions (`rm`, `for-each-ref`) with host implementations for slate migration paths."
    checked: false

  - id: sp6-builtin-dispatch-removed
    text: "Builtin spec-manager dispatch paths are removed and replaced by slate pando flows."
    checked: false

  - id: sp7-spec-compat-alias
    text: "`patina spec` remains a compat alias that forwards to slate workflows so existing scripts still work (including `patina spec --help`)."
    checked: false

  - id: sp8-compile-proof
    text: "`cargo check --workspace -q` and `cargo test -q --lib` pass, and slate command flows work with real spec files."
    checked: false
---
# feat: Slate Pando Migration

## Problem

Spec/slate behavior still relies on builtin dispatch surfaces. To complete the
pando architecture, slate must move to a real pando + child workflow and retire
the builtin pathways.

## Goal

Deliver CLI discovery/routing plus full slate pando migration while preserving
`patina spec` compatibility semantics.

## Scope

- CLI discovery/help flow for pando commands.
- Pando command dispatch through Mother to child actions.
- Slate-manager child and slate pando implementation.
- Compat alias behavior for `patina spec` including `patina spec --help`.

## Non-Goals

- Reworking already-complete pando platform lifecycle and artifact identity work.
- Third-party pando registry transport (tracked in explore/future specs).
