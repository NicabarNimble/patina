---
type: feat
id: slate-pando-migration
status: active
created: 2026-04-07
updated: 2026-04-21
beliefs:
  - "[[spec-driven-design]]"
  - "[[safety-boundaries]]"
  - "[[dependable-rust]]"
  - "[[wasi-is-foundation-not-option]]"
references:
  - layer/core/values/spec-driven-design.md
  - layer/core/values/safety-boundaries.md
  - layer/core/values/dependable-rust.md
  - layer/core/values/unix-philosophy.md
related:
  - children/spec-manager/
  - sdk/patina-sdk/
  - src/commands/spec/
  - src/commands/mother/daemon/dispatch.rs
  - mother/src/runtime.rs
  - mother/src/builtin_children.rs
  - src/release/internal.rs
  - layer/surface/build/feat/spec-release-pr-automation/SPEC.md
exit_criteria:
  - id: sp1-parity-command-surface
    text: "Slate exposes a 1:1 parity command surface for current `patina spec` flows (`list`, `next`, `show`, `check`, `prompt`, `handoff`, `packet`, `complete`, `archive`) with equivalent outputs for existing fixtures."
    checked: false

  - id: sp2-full-wit-child
    text: "Slate runs as a proper WASM child with typed WIT exports/imports (no legacy `handle(action,payload)` fallback in execute path)."
    checked: true

  - id: sp3-toy-scoped-manifest
    text: "Slate child manifest uses `[needs].toys` (+ optional scopes) only; granted toys are minimal and explicit for git/fs/process/release interactions."
    checked: false

  - id: sp4-spec-compat-kept
    text: "`patina spec` remains operational as compatibility surface and can route through Slate when enabled, preserving script compatibility."
    checked: true

  - id: sp5-project-opt-in-policy
    text: "Project-level opt-in exists (`off|observe|execute`) with policy config for PR/release command customization so projects can map to different CI conventions without forking core logic."
    checked: false

  - id: sp6-sdk-fit-defined
    text: "SDK role is explicit: `sdk/patina-sdk` remains the child authoring surface; Slate uses it directly plus generated WIT bindings, and missing toy helpers are tracked/implemented only when they reduce repeated boilerplate."
    checked: false

  - id: sp7-mother-distribution-contract
    text: "Mother can register/provision Slate as a child capability for a project and enforce runtime policy/grants centrally."
    checked: false

  - id: sp8-fail-closed-rollback
    text: "Failure modes are fail-closed with deterministic rollback path (`spec` builtin path remains callable until Slate parity is proven)."
    checked: true

  - id: sp9-proof-tests
    text: "Parity and routing tests pass (`cargo check -q --workspace`, targeted command snapshots, and child-runtime integration tests for observe/execute modes)."
    checked: false
---
# feat: Slate migration to full-WIT spec child (1:1 parity first)

> Build Slate as a proper child/tool that mirrors today’s `spec` behavior exactly first. Keep `spec` alive as compatibility surface until Slate proves parity.

## Problem

Current slate concept is tied to older pando framing and does not yet lock the immediate need:

1. A **full-WIT child** implementation for spec operations.
2. **1:1 behavior parity** with existing `patina spec` commands.
3. A **project-selectable runtime policy** for PR/release command differences across CI setups.

Without this parity-first contract, migration introduces risk and operational drift.

## Goal

Deliver Slate as an opt-in full-WIT child that can execute the current spec lifecycle end-to-end with compatibility guarantees.

Primary principle: **parity first, innovation second**.

## Non-Goals

- Replacing/removing `patina spec` in this spec.
- Multi-channel release orchestration (alpha/beta/nightly).
- New product surface unrelated to current spec lifecycle.

## Normative architecture

### 1) Command surface (user-facing)

- Keep `patina spec ...` as stable command surface.
- Add Slate execution backend and routing toggle.
- Initial routing modes:
  - `off`: current behavior only.
  - `observe`: run Slate plan/render side-by-side, no side effects.
  - `execute`: Slate performs side effects.

### 2) Child runtime (execution)

- Slate is a **proper child** using typed WIT interfaces.
- No legacy untyped action envelope in execute path.
- Child manifest follows current canon:
  - `[needs].toys = [...]`
  - optional `[needs.scopes]` for least privilege.

### 3) Mother role (control plane)

- Mother provisions Slate availability per project.
- Mother enforces grants/policy/scopes.
- Project data remains project-scoped; Mother owns enablement and policy authority.

## SDK fit in this design

`sdk/patina-sdk` is the **child authoring surface**, not release policy authority.

- Slate child should use `patina-sdk` + generated WIT bindings for toy calls and typed contracts.
- If Slate needs toy helpers not present in SDK (for example richer git/process helpers), add them only when they remove repeated cross-child boilerplate.
- CLI command semantics, policy, and dispatch remain in Patina core/Mother surfaces; SDK stays focused on child ergonomics.

## Parity matrix (must hold before default cutover)

- `spec list` -> `slate list` parity
- `spec next` -> `slate next` parity
- `spec show` -> `slate show` parity
- `spec check` -> `slate check` parity
- `spec prompt/handoff/packet` parity
- `spec complete/archive` parity including version bump/release triggers defined by current rules

## Implementation updates (2026-04-21)

- Slate child scaffold exists as full-WIT package at `children/slate-manager/` and compiles.
- `patina spec` now resolves backend mode from env or project manifest (`[spec] mode = off|observe|execute`).
- Observe mode preserves builtin output and appends `backend.slate_probe` metadata.
- Execute mode routes through typed `slate-manager` call path and fails closed if child is missing/unavailable.
- Execute mode is strict fail-closed: when Slate reports scaffold/not-implemented (or any execute-path error), Mother returns an error instead of silently falling back.
- Added routing smoke coverage in `src/commands/spec/mod.rs` and daemon dispatch tests in `src/commands/mother/daemon/tests/mod.rs`.
- Started Option A git-toy expansion for Slate mutate/release parity (`create-tag-at`, `status-porcelain`, `add-paths`, `is-clean-tracked`, `commits-behind-upstream`, `is-diverged`) in WIT + host bindings.
- Slate child dispatch is command-aware (parses envelope command + backend mode).
- Slate now implements initial read-only command handlers in-child (`list`, `next`, `show`, `check`, `prompt`, `handoff`, `packet`) from filesystem/frontmatter parsing; outputs are still early parity and need fixture-level equivalence checks.
- Execute dispatch now binds Slate reads to envelope project root (and fails closed on invalid project roots) for per-project isolation.
- Added execute handlers for `complete` and `archive` with git-backed archiving path inside Slate child.
  - `complete` now supports release-bump flows (`feat`/`fix`/`refactor` and `--major`) by bumping Cargo version, tagging release, and archiving the spec tag in execute mode.
  - Explore/unknown spec types remain archive-only completion (no version bump).
- Added `patina:git` toy operation `remove-paths` for tracked deletion workflows used by Slate archive path.
- Added observe-mode fixture diff harness test covering read-only command set and builtin/probe payload capture.

## Toy contract packaging direction

Git/release parity will use **Option A** (expand `patina:git`) and move toward
multi-file WIT package layout (foldered contracts, analogous to WASI HTTP/io).

- Keep WASI foundations for generic FS/clock behavior.
- Keep Patina toys for host-boundary deltas (git/release/policy).
- Implement packaging/tooling updates needed for non-flat Patina toy contracts as part of `sp3`.

## Project policy customization

Provide project-local config (read by routed Slate backend) for command differences such as:

- PR title/body template policy
- required check names / success interpretation
- release branch naming conventions
- optional command wrappers

Defaults must replicate current core behavior so projects can opt in without extra config.

## Execution order (implementation slices)

1. **Parity contract freeze**
   - Lock command/JSON/output fixtures for current `spec` flows.
2. **Full-WIT Slate child scaffold**
   - Add child contract + runtime wiring + minimal toy grants.
3. **Observe mode routing**
   - `patina spec` can run Slate shadow path and diff outputs.
4. **Execute mode routing**
   - Enable side effects behind opt-in.
5. **Per-project policy mapping**
   - Add CI/PR customization knobs.
6. **Default-switch readiness review**
   - Keep `spec` compat path until parity criteria stay green across fixtures.

## Verification

```bash
patina spec check slate-pando-migration --json
cargo check -q --workspace
cargo test -q --lib
```

Behavior checks:
- existing `patina spec` scripts still run with routing `off`.
- routing `observe` produces parity reports without mutating git/GitHub state.
- routing `execute` performs PR/release actions under explicit policy and toy grants.

## Exit Criteria

Frontmatter `sp1..sp9` are the source of truth.
