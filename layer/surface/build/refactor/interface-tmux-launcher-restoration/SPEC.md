---
type: refactor
id: interface-tmux-launcher-restoration
status: ready
created: 2026-03-23
updated: 2026-03-23
sessions:
  origin: 20260323-055550-214548000
beliefs:
- agents-are-guests-mother-is-infrastructure
- tmux-lane-defines-active-session
- durability-lives-outside-interface-process
- git-tags-must-be-real-or-not-claimed
- universal-artifact-interface-specific-enrichment
exit_criteria:
- id: TL1
  text: Tmux launch support is restored for Claude/OpenCode/Gemini with explicit policy (`--tmux` and `--no-tmux`) and deterministic per-interface lane naming
  checked: false
- id: TL2
  text: AI launch request/runtime contract carries tmux decision + lane identity without regressing current session check-in, environment injection, and bundle bootstrap behavior
  checked: false
- id: TL3
  text: Launcher transport supports tmux attach/reuse (`tmux -L <socket> new-session -A -D`) with safe direct-launch fallback when tmux is unavailable or unsuitable
  checked: false
- id: TL4
  text: Session liveness authority remains Mother/session runtime (socket/daemon truth), not tmux lane existence
  checked: false
- id: TL5
  text: No reintroduction of historical tag-integrity bugs (`tmux-lost`/`superseded` frontmatter claims without matching real git tags)
  checked: false
- id: TL6
  text: Interface bundle model is extended for launch policy metadata without blocking future bundle/tarball evolution
  checked: false
- id: TL7
  text: Runtime policy/tooling/docs are aligned (no stale references to removed flags or contradictory launcher semantics)
  checked: false
- id: TL8
  text: '`cargo check -q`, targeted AI/session tests, and launcher smoke probes pass for tmux and non-tmux paths'
  checked: false
---
# refactor: restore tmux interface launch lanes safely

> Restore tmux launch support for Claude/OpenCode/Gemini as launcher transport while keeping Mother-owned session truth and preserving interface bundle evolution.

## Problem

Recent refactor work intentionally removed tmux launch lanes from the interface runtime path
(`f597c608`: retire interface runtime launchers and tmux lanes) while completing major Mother/
Child/Toy architecture cleanup. This protected boundary work, but it also removed proven operator
ergonomics for the three supported interfaces (Claude/OpenCode/Gemini): per-interface tmux lanes,
reattach flow, and explicit lane identity.

The risk now is a pendulum swing: blindly restoring old tmux code can reintroduce broken behavior
we already learned from (session truth coupled to tmux state, inconsistent crash tag semantics,
runtime coupling drift).

We need a constrained restoration: tmux as launch transport and UX lane, while preserving the
new architecture where Mother/session runtime owns liveness and durability.

## Goal

Restore tmux-backed launch lanes for the three AI interfaces without undoing architectural gains:

- tmux returns as an explicit launcher transport,
- Mother/session runtime remains source of truth for session liveness and archive semantics,
- interface bundles gain launch-policy metadata to support current behavior and future tarball-style
  bundle evolution,
- verification gates prevent recurrence of known session/tag regressions.

## Status

Draft. No implementation commits in this spec yet.

## Non-Goals

- No rollback to "tmux lane = session liveness" semantics.
- No reintroduction of monolithic/legacy interface runtime paths that were intentionally retired.
- No rewrite of Mother session lifecycle architecture in this spec.
- No changes to child capability schema (`[needs].toys` + optional `[needs.scopes]`).
- No expansion beyond the three supported interfaces.

## Current State

- Current launch contract is direct exec only (`src/interface/internal/launcher.rs`,
  `src/interface/mod.rs` `LaunchRequest`), no tmux decision/session lane fields.
- AI launch flow still handles workspace/project checks, bundle readiness, and session check-in in
  `src/commands/ai/surface.rs`; tmux-specific reconciliation was removed.
- Interface bundle model exists (`src/interface/internal/bundle.rs`,
  `src/interface/internal/bootstrap.rs`) but does not yet express launch transport policy.
- Belief layer was intentionally scoped: tmux is launcher infrastructure, not architectural liveness
  authority (`layer/surface/epistemic/beliefs/tmux-lane-defines-active-session.md`).

## Target State

- `patina ai claude|opencode|gemini` supports explicit tmux usage and explicit no-tmux behavior.
- Launch transport includes deterministic per-interface lane naming and socket isolation.
- Tmux path has safe fallback to direct exec when conditions fail (missing tmux, old tmux, non-TTY,
  explicit disable).
- Session truth remains Mother-owned: tmux lane state is diagnostic context, not archive authority.
- Bundle metadata can represent launch policy/versioned behavior so future interface tarball work
  can evolve without re-threading launch semantics through unrelated modules.

## Solution

1. Restore tmux transport primitives (decision, lane naming, tmux exec strategy) in interface
   launcher internals.
2. Re-extend launch contract (`LaunchRequest`) so AI surface can pass tmux policy + lane identity.
3. Add explicit user policy flags in AI launch surface (`--tmux`, `--no-tmux`) and define deterministic
   resolution order.
4. Integrate tmux policy defaults into interface bundle metadata (policy-capable now, future bundle
   packaging friendly).
5. Keep reconciliation constrained: no tmux-only liveness claims; if stale-session handling is
   present, it must rely on Mother/session authority and real tag behavior.
6. Add regression tests and smoke probes covering both tmux and direct launch paths.

## Implementation Order

1. Spec/design lock: policy contract, liveness boundary, and regression guardrails.
2. Contract restore: `LaunchRequest` + interface launcher API extension.
3. Tmux transport restore: decision engine + lane derivation + tmux exec/fallback.
4. AI CLI policy surface: add and wire `--tmux` / `--no-tmux`.
5. Bundle metadata policy extension and defaults for three interfaces.
6. Tests + command proofs + policy/tooling alignment pass.

## Resolved Decisions

1. Tmux is restored as launcher transport, not session truth authority.
2. Session archive/tag integrity remains governed by Mother/session runtime semantics.
3. Explicit user control beats implicit heuristics; auto behavior may exist but must be policy-visible
   and test-covered.
4. Bundle model is the right place for interface launch policy metadata.
5. If tmux is unavailable, launch continues via direct exec with clear messaging; no silent failure.

## Verification

- `cargo check -q`
- `cargo test -q`
- Targeted launcher/session tests covering:
  - tmux decision precedence,
  - lane naming stability by interface,
  - tmux fallback path,
  - no-tmux explicit behavior,
  - no false liveness claims from tmux absence.
- CLI smoke:
  - `patina ai --help`
  - `patina ai claude --help`
  - `patina ai opencode --help`
  - `patina ai gemini --help`

## Exit Criteria

See frontmatter `exit_criteria` TL1-TL8.

## Build Readiness

Ready for implementation after DESIGN commit plan is finalized with path-anchored code targets and
test additions.
