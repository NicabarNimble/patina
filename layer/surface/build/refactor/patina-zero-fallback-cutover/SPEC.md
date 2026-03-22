---
type: refactor
id: patina-zero-fallback-cutover
status: draft
created: 2026-03-22
sessions:
  origin: 20260321-162736-004031000
blocked_by: []
related:
- layer/surface/build/refactor/patina-pre-v1/SPEC.md
- src/commands/context.rs
- src/commands/scry/mod.rs
- src/commands/assay/mod.rs
- src/commands/spec/mod.rs
- src/commands/measure/mod.rs
- src/commands/lake.rs
- src/mother/daemon_client.rs
- mother/src/daemon.rs
exit_criteria:
- id: cutover-g1
  text: No embedded Mother execution paths remain for migrated commands (context, scry, assay, spec, measure, lake); CLI uses daemon client only
  checked: false
- id: cutover-g2
  text: Daemon actions for migrated commands return real behavior or explicit operational errors, never scaffold placeholders (e.g. "not yet implemented")
  checked: false
- id: cutover-g3
  text: Retired MCP behavior is unreachable; no runtime/template/config path launches `--mcp` except explicit retirement errors
  checked: false
- id: cutover-g4
  text: Poison-pill CI checks fail on fallback/scaffold marker reintroduction and embedded-path regressions
  checked: false
- id: cutover-g5
  text: Daemon-only end-to-end verification passes for migrated commands without embedded fallback assertions
  checked: false
---
# refactor: refactor: Patina Zero-Fallback Cutover

> Finalize daemon-only architecture and remove embedded fallback execution paths.

## Problem

`patina-pre-v1` reached implementation boundary with daemon-first routing in place, but key command paths still preserve embedded fallback behavior and several daemon actions still return scaffolds. This keeps functional parity, but it does not complete architectural cutover.

## Goal

Ship a finite zero-fallback cutover that removes legacy execution semantics for already-migrated commands without expanding product scope.

## Status

Draft. `patina-pre-v1` is now blocked by this spec until cutover gates are met.

## Non-Goals

- No new user-facing features.
- No enterprise DuckLake pipeline work.
- No unrelated refactors beyond migration cleanup and verification harnessing.

## Current State

- Daemon transport/protocol exists and is exercised.
- CLI routes migrated commands daemon-first.
- Embedded fallbacks still preserve behavior when daemon path is scaffolded or unavailable.
- Tests currently pass, but completion semantics are hybrid (daemon + fallback).

## Target State

- Migrated commands execute daemon-only.
- Daemon handlers provide non-scaffold behavior for migrated actions.
- MCP retired behavior is fail-fast only.
- CI prevents reintroduction of fallback/scaffold behavior.

## Solution

Use a strict five-gate cutover policy:

1. Remove embedded command fallbacks for migrated surfaces.
2. Replace daemon scaffolds with real behavior (or explicit operational errors).
3. Enforce MCP retirement consistency across all setup/runtime surfaces.
4. Add poison-pill CI checks for banned markers and fallback entry points.
5. Prove daemon-only E2E behavior across migrated commands.

## Implementation Order

1. Inventory fallback/scaffold markers and embedded call paths.
2. Remove embedded fallback branches for migrated commands.
3. Replace scaffold daemon handlers for migrated actions.
4. Add/enable poison-pill CI checks.
5. Run daemon-only E2E matrix and record proof.

## Resolved Decisions

- Completion is defined by binary cutover gates, not by additional phase count.
- Scope is locked to migration completion for existing surfaces only.

## Verification

Minimum command matrix (daemon-only path):

- `patina context "<query>"`
- `patina scry "<query>"`
- `patina assay <subcommand>`
- `patina spec <subcommand>`
- `patina measure <subcommand-or-flags>`
- `patina lake list`
- `patina lake create <valid-name>`

Negative checks:

- Search reports show no scaffold placeholder strings for migrated actions.
- Search reports show no embedded fallback entry points for migrated commands.
- Search reports show no `--mcp` launch paths outside explicit retirement errors.

## Exit Criteria

See metadata `exit_criteria` (cutover-g1 through cutover-g5).

## Build Readiness

Ready when `patina-pre-v1` remains blocked and cutover work is authorized as the current active build stream.
