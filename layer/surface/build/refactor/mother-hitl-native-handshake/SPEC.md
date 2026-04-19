---
type: refactor
id: mother-hitl-native-handshake
status: ready
created: 2026-04-19
sessions:
  origin: 20260417-221509-436744000
related:
- src/commands/launch/internal.rs
- src/commands/ai/surface.rs
- src/interface/internal/launcher.rs
- src/interface/internal/checkin.rs
- src/commands/mother/daemon/startup.rs
- src/commands/mother/daemon/dispatch.rs
- mother/src/http_api.rs
- mother/src/http_api/health.rs
- mother/src/http_api/child.rs
- wit/child/child.wit
- wit/pando/pando.wit
exit_criteria:
- id: mhnh1-mother-required-hitl
  text: HITL launch path fails closed when Mother is unavailable; no best-effort local fallback in default policy.
  checked: false
- id: mhnh2-fast-ready-probe
  text: Launch preflight uses a bounded fast readiness probe (status-line based) and avoids multi-second blocking loops.
  checked: false
- id: mhnh3-native-hitl-api-surface
  text: Mother exposes native HITL control operations (`handshake`, `resolve-envelope`, `heartbeat`, `end-envelope`) through a dedicated interface control route.
  checked: false
- id: mhnh4-wit-contract-source-of-truth
  text: HITL control request/response types are defined in WIT and used as the canonical contract source.
  checked: false
- id: mhnh5-rivet-shape-native-core
  text: Native HITL route adopts Rivet-style operation envelope semantics (`operation_id`, `args`, `correlation`) without requiring Rivet integration mode.
  checked: false
- id: mhnh6-rivet-optional-adapter
  text: Existing `/api/rivet/dispatch` remains an optional ingress adapter and can invoke the same internal typed operations when enabled.
  checked: false
- id: mhnh7-envelope-authority-in-mother
  text: Mother is authoritative for `(project, interface)` envelope resolution (`attach|create|choose|reject`) and returns deterministic session/lane metadata.
  checked: false
- id: mhnh8-identity-handshake-fields
  text: Handshake contract includes protocol version, CLI version, project uid, project root, interface identity, and launch intent.
  checked: false
- id: mhnh9-tmux-lane-contract
  text: Launcher uses Mother-returned envelope/session lane identity for tmux launch/reattach behavior.
  checked: false
- id: mhnh10-observability
  text: Typed launch operations emit stable events with correlation and decision fields for audit/debugging.
  checked: false
- id: mhnh11-no-regression-existing-hitl
  text: Existing HITL UX goals remain intact (picker behavior, selected/default setup behavior, direct `patina ai <interface>` path).
  checked: false
- id: mhnh12-proof
  text: '`cargo check --workspace -q` passes and targeted tests cover readiness fast-path, Mother-required failure mode, handshake/resolve decision outcomes, and Rivet-adapter parity.'
  checked: false
validated_against_commit: 65a8423d
last_freshness_check: 2026-04-19
freshness_scope:
- src/commands/launch/internal.rs
---
# refactor: Mother-native HITL handshake + envelope control

> Define a WIT-shaped, Rivet-style typed contract for HITL launcher↔Mother interactions (ready, handshake, envelope resolve, heartbeat, end), enforce Mother-required semantics for HITL, and keep Rivet as optional ingress adapter rather than core control path.

## Problem

HITL launch currently performs a fragile/slow Mother liveness check and then falls back to best-effort local behavior when Mother is unavailable. This conflicts with the desired operator model: Mother should behave like a required daemon for HITL control-plane decisions.

Current preflight also mixes concerns:
- liveness probing,
- daemon startup attempts,
- envelope/session resolution,
- launcher-side fallback behavior.

This causes noisy UX, false negatives, and ambiguous ownership boundaries.

## Goal

1. Make Mother a strict default prerequisite for HITL launch.
2. Make readiness probing fast and bounded.
3. Move HITL envelope decisions to Mother as typed operations.
4. Use WIT as contract source of truth.
5. Reuse Rivet envelope shape semantics without coupling HITL core to Rivet profile flags.

## Status

Draft and implementation-ready.

## Non-Goals

- Full agentic runtime redesign in this slice.
- Replacing tmux with another process host in this slice.
- Requiring Rivet profile to be enabled for core HITL behavior.
- Rewriting session artifact model.

## Current State

- `patina`/`patina ai <interface>` launches can continue after Mother check failure via warning/fallback.
- Mother health preflight in launcher is vulnerable to partial-response parse issues.
- Rivet integration exists as `/api/rivet/dispatch` and is feature-gated by daemon startup profile.
- HITL envelope attach/create logic lives primarily in interface check-in path with launcher-side orchestration.

## Target State

- HITL launch does: `ready -> handshake -> resolve-envelope -> launch`.
- Mother determines authoritative envelope decision (`attach|create|choose|reject`).
- Launcher executes runtime with Mother-issued identity metadata and fails closed when required preconditions fail.
- Native HITL route is always available in Mother control plane (independent of Rivet profile).
- Rivet ingress can call same typed operations when enabled.

## Solution

### 1) Fast ready gate
Add a dedicated readiness endpoint (`/ready`) and/or fast status-line probe semantics:
- minimal payload,
- tiny bounded timeout,
- no long retry loops in interactive launch path.

### 2) Native HITL typed call surface
Add Mother interface-control route (native):
- `patina:interface/handshake.v1`
- `patina:interface/envelope.resolve.v1`
- `patina:interface/envelope.heartbeat.v1`
- `patina:interface/envelope.end.v1`

Use a typed operation envelope:
- `operation_id`
- `args`
- `correlation`

### 3) WIT-first contracts
Define WIT package for interface control types and operations. Generate/align Rust types to this contract. Keep transport local and fast (UDS HTTP) while contract remains component-model aligned.

### 4) Mother-required HITL policy
Default HITL launcher behavior:
- Mother unavailable => fail closed with actionable operator message.
- No silent best-effort fallback in default mode.

(Any override policy must be explicit and out-of-band from default operator UX.)

### 5) Rivet as optional adapter
Retain `/api/rivet/dispatch` for actor/workflow ingress. Map Rivet dispatch to same internal typed operation handlers when applicable. Rivet remains optional adapter, not core dependency.

## Implementation Order

1. **Contract scaffold**: add WIT package/types for HITL control operations.
2. **Mother endpoints**: add native interface-control route and handlers.
3. **Launcher adoption**: switch HITL launch path to ready+handshake+resolve flow and strict failure policy.
4. **Parity adapter**: wire Rivet ingress mapping into same internal operation handlers.
5. **Tests + observability**: add targeted tests and stable events/correlation fields.

## Resolved Decisions

- Mother is required by default for HITL control path.
- Fast readiness gate precedes all launch orchestration.
- WIT defines contract types; native Mother route is runtime control-plane.
- Rivet contract shape is adopted; Rivet route is optional adapter.
- Envelope authority remains in Mother.

## Verification

- `cargo check --workspace -q`
- targeted tests:
  - readiness probe bounded behavior,
  - Mother-required failure mode,
  - handshake validation cases,
  - envelope resolve attach/create/ambiguous/reject,
  - Rivet adapter parity to native internal operations.
- smoke:
  - existing project `patina` in TTY,
  - direct `patina ai <interface>`,
  - non-interactive failure behavior.

## Exit Criteria

Tracked in frontmatter `exit_criteria` (`mhnh1`..`mhnh12`).

## Build Readiness

Ready for phased implementation.
