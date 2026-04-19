# Design: Mother-native HITL handshake + envelope control

## Why This Design

HITL launch now depends on a soft, launcher-side Mother check that can stall and can falsely report Mother unavailable. That creates UX noise and split authority.

This design makes Mother the strict control-plane authority for HITL envelopes while preserving fast local launch latency and keeping Rivet optional.

Principles:
- Mother-required by default for HITL
- fast bounded readiness probe
- typed contract-first (WIT)
- single internal operation handlers
- multiple ingress adapters (native + Rivet)

## Build Target

1. Fast ready gate (`/ready`) with bounded probe budget.
2. Native HITL typed operation route (`/api/interface/call`).
3. WIT contract for HITL control operations and types.
4. Mother-authoritative envelope decision (`attach|create|choose|reject`).
5. Launcher flow: `ready -> handshake -> resolve -> launch`.
6. Rivet dispatch can invoke same internal operation handlers (optional).

## Resolved Decisions

- Mother is required in default HITL policy.
- No best-effort fallback in default HITL launch.
- WIT is source-of-truth for operation contract/types.
- Native route is primary runtime control plane.
- Rivet route is adapter, not dependency.
- tmux/session identity comes from Mother resolution result.

## Cutover Checklist (No-Frankenstein Gates)

This checklist is mandatory before declaring implementation complete.

1. **Single authority (mhnh13)**
   - Envelope decisions (`attach|create|choose|reject`) happen in Mother handlers only.
   - Launcher consumes decision output and does not duplicate decision logic.

2. **Legacy fallback removal (mhnh14)**
   - Default HITL path no longer warns-and-continues when Mother checks fail.
   - Any bypass mode is explicit, opt-in, and outside normal operator defaults.

3. **Typed decision model (mhnh15)**
   - Core control outcomes and errors use typed enums/variants.
   - No stringly-typed branching for primary launch-state transitions.

4. **State-machine proof (mhnh16)**
   - Tests cover transitions: `ready -> handshake -> resolve -> launch -> heartbeat -> end`.
   - Failure edges include timeout, identity rejection, ambiguous selection, stale handshake, and unknown envelope.

5. **Audit readiness (mhnh17)**
   - Invariants documented and enforced in code.
   - IO remains bounded in launch preflight.
   - Fail-closed defaults validated.
   - Superseded default path code removed after parity confirmation.

## Rust Systems Rigor Audit Lens

If a Rust systems auditor reviewed this end state, it should satisfy:
- **Explicit invariants** over implicit runtime assumptions.
- **Small, bounded IO** in critical launch path.
- **Typed state transitions** rather than ad-hoc string matching.
- **Single source of truth** for envelope resolution.
- **Delete-after-cutover discipline** to prevent long-lived dual-path drift.

## Contract Shape

### WIT package

Add new package (path proposal):
- `wit/interface-control/interface-control.wit`

Operations:
- `patina:interface/handshake.v1`
- `patina:interface/envelope.resolve.v1`
- `patina:interface/envelope.heartbeat.v1`
- `patina:interface/envelope.end.v1`

### Native HTTP operation envelope

`POST /api/interface/call`

```json
{
  "operation_id": "patina:interface/handshake.v1",
  "args": { ... },
  "correlation": {
    "project_uid": "ebdc3b02",
    "interface": "pi",
    "launch_id": "uuid"
  }
}
```

Use Rivet-shaped fields (`operation_id`, `args`, `correlation`) for parity and future adapter reuse.

### Ready gate

`GET /ready`
- `204 No Content` when control plane is ready
- non-2xx otherwise
- no large JSON payload required

## Operation Semantics

### 1) handshake.v1

Purpose: validate caller/project/interface identity and issue short-lived handshake token.

Required args:
- `protocol_version`
- `cli_version`
- `project_uid`
- `project_root`
- `interface_name`
- `interface_kind` (`hitl`)
- `launch_intent` (`attach-or-create|attach-only|create-only`)
- optional `requested_session`
- optional `tmux_mode`
- `tty` bool

Result:
- `handshake_id`
- `mother_version`
- normalized `project_uid`
- `control_plane_ready`
- expiry timestamp

### 2) envelope.resolve.v1

Purpose: choose/create envelope and return launch identity.

Input:
- `handshake_id`
- optional `title`

Result variant:
- `attach` => existing envelope/session/lane
- `create` => new envelope/session/lane
- `choose` => ambiguous active sessions (TTY can prompt)
- `reject` => policy/identity failure

Return identity fields:
- `envelope_id`
- `session_runtime_id`
- `session_file_id`
- `tmux_lane`
- `interface_name`
- `project_uid`

### 3) heartbeat.v1

Purpose: keep envelope liveness current.

Input:
- `envelope_id`
- `pid`
- `tmux_lane`
- timestamp

Result:
- ack or reject (unknown envelope/expired)

### 4) end.v1

Purpose: close envelope lifecycle on explicit end.

Input:
- `envelope_id`
- `reason`

Result:
- ack + final state summary

## Mother Internal Architecture

### Ingress adapter layer

- Native route parser (`/api/interface/call`) validates envelope and maps to typed internal request enum.
- Rivet route (`/api/rivet/dispatch`) maps compatible operations to the same internal enum when enabled.

### Internal operation handlers

Single handler module executes typed operations and owns validation + policy decisions.

Proposed module seam:
- `src/commands/mother/daemon/interface_control.rs`

### Session/envelope authority

Reuse existing session truth in Mother runtime store and interface check-in semantics, but move attach/create decision authority into Mother handler path.

## Launcher Integration

Update HITL launcher flow (`launch` / `ai::surface::launch`):

1. `ready` probe (fast bounded)
2. call `handshake.v1`
3. call `envelope.resolve.v1`
4. launch runtime using Mother-returned identity

Failure behavior:
- if ready/handshake/resolve fails in default policy => fail closed with actionable error.

## Timeout & UX Budgets

- UDS connect timeout: very small (single-digit to low tens of ms)
- ready probe budget: bounded (no multi-second loops)
- no daemon-start polling loop in interactive launch path

## Observability

Emit stable events for:
- `interface.handshake`
- `interface.envelope.resolve`
- `interface.envelope.heartbeat`
- `interface.envelope.end`

Fields:
- `project_uid`, `interface`, `operation_id`, `decision`, `session_runtime_id`, `session_file_id`, `envelope_id`, correlation fields.

## Rivet Integration Strategy

- Keep existing `--rivet enabled|disabled` gate.
- When enabled, `/api/rivet/dispatch` may invoke HITL operations via same internal handler.
- Native HITL route remains available regardless of Rivet mode.

## Commits

1. `feat(wit): add interface-control WIT contract`
2. `feat(mother): add /ready and native interface control route`
3. `feat(mother): implement handshake + envelope resolve handlers`
4. `refactor(launch): require Mother handshake flow for HITL`
5. `feat(mother): map rivet dispatch to interface-control ops (adapter parity)`
6. `test(launch): add fast-ready and fail-closed behavior coverage`

## Direct Code Targets

- `wit/interface-control/interface-control.wit`
- `mother/src/http_routes.rs`
- `mother/src/http_api.rs`
- `mother/src/http_api/health.rs`
- `mother/src/http_api/child.rs` (shared envelope parsing pattern reference)
- `src/commands/mother/daemon/dispatch.rs`
- `src/commands/mother/daemon/startup.rs`
- `src/commands/mother/daemon/interface_control.rs` (new)
- `src/commands/launch/internal.rs`
- `src/commands/ai/surface.rs`
- `src/interface/internal/checkin.rs`

## Verification Plan

- `cargo check --workspace -q`
- targeted tests for:
  - ready probe fast path
  - fail-closed Mother-required behavior
  - handshake identity validation
  - resolve attach/create/choose/reject outcomes
  - native vs Rivet adapter parity to internal handler
- manual smoke:
  - existing project interactive `patina`
  - `patina ai <interface>` direct path
  - Mother stopped case produces immediate actionable failure

## Build Readiness

Ready for phased implementation.

## Open Questions

1. Should `choose` ever be returned to non-interactive callers, or always hard reject with explicit choices?
2. Should heartbeat be launcher-driven only, or also refreshed by session update/note flows?
3. Do we require handshake token TTL + nonce revocation in v1, or rely on short process-local scope first?
