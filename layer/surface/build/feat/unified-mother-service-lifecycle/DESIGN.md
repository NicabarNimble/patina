# Design: Unified Mother service lifecycle across macOS and Linux

## Why This Design

Patina should expose one operator command surface while delegating service control to native host supervisors. This preserves platform correctness and gives operators a single mental model.

## Build Target

- One CLI lifecycle contract for Mother.
- launchd adapter (macOS) + systemd user adapter (Linux).
- Backend-aware status and diagnostics.
- Conflict detection for mixed management modes.

## Resolved Decisions

1. Keep native supervisors; do not invent a custom cross-platform daemon manager.
2. Linux target is systemd user units for this slice.
3. Homebrew remains a valid backend owner on macOS; CLI should detect/report ownership clearly.
4. Status output should include both health and management backend.

## Commits

1. `feat(mother): surface supervisor backend in status output`
2. `feat(mother): add Linux systemd-user install/uninstall backend`
3. `feat(mother): add restart command + mixed-control guardrails`
4. `docs(mother): unify macOS/Linux service runbook`

## Direct Code Targets

- `src/commands/mother/mod.rs` — command surface and backend dispatch
- `src/commands/mother/daemon/` — lifecycle handlers and status probes
- `src/paths.rs` — cross-platform service file/unit paths
- `README.md` — unified operator runbook

## Verification Plan

1. Unit tests for backend detection and command dispatch.
2. Integration tests for launchd and systemd command generation/parsing (mocked).
3. Manual smoke on macOS and Linux user sessions.
4. Documentation validation against real commands.

## Build Readiness

Ready to start with status/backend detection slice first (low-risk, no lifecycle mutation).

## Open Questions

1. Should `patina mother install` on macOS no-op with guidance when Homebrew service already controls Mother?
2. Should restart semantics always route through supervisor when installed (vs direct process signal path)?
3. Do we need explicit `patina mother logs` in this slice or document native log commands only?
