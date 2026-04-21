---
type: feat
id: unified-mother-service-lifecycle
status: active
created: 2026-04-20
sessions:
  origin: 20260416-133521-394965000
related:
- src/commands/mother/mod.rs
- src/commands/mother/daemon
- src/commands/mother/daemon/interface_control.rs
- src/commands/mother/daemon/dispatch.rs
- src/paths.rs
- README.md
- packaging/homebrew/Formula/patina.rb
exit_criteria:
- id: umsl1-unified-command-surface
  text: '`patina mother install|uninstall|start|stop|restart|status` provide one operator surface across supported platforms.'
  checked: false
- id: umsl2-macos-launchd-backed
  text: On macOS, install/uninstall is launchd-backed and remains compatible with Homebrew service usage.
  checked: false
- id: umsl3-linux-systemd-user-backed
  text: On Linux, install/uninstall uses systemd user units (`systemctl --user`) with equivalent daemon command contract.
  checked: false
- id: umsl4-backend-aware-status
  text: '`patina mother status` reports effective supervisor backend (manual/launchd/systemd-user) and health.'
  checked: true
- id: umsl5-logs-and-runbook
  text: Operator docs provide unified lifecycle runbook plus backend-specific log locations/commands.
  checked: false
- id: umsl6-conflict-guards
  text: CLI warns/fails safely on mixed supervisor control (e.g., manual/nohup plus managed service) and gives remediation steps.
  checked: false
validated_against_commit: b1254cb0
last_freshness_check: 2026-04-20
freshness_scope:
- src/commands/mother/mod.rs
- README.md
---
# feat: Unified Mother service lifecycle across macOS and Linux

> Make Mother feel like an integrated system daemon with one Patina command contract and native backend adapters per OS.

## Problem

Mother lifecycle management is currently macOS-forward and split between multiple control paths (`patina mother`, `brew services`, manual process management). Linux lacks a first-class equivalent supervisor flow in the same command surface.

Result: operator drift, inconsistent startup behavior, and weak cross-platform muscle memory.

## Goal

1. Keep one command contract for Mother lifecycle.
2. Use native OS supervisors behind that contract.
3. Keep status and logs discoverable in one place.
4. Reduce accidental mixed-control states.

## Status

Active — implementation underway (backend-aware status slice landed).

## Non-Goals

- Replacing launchd/systemd with a custom Patina supervisor.
- Supporting non-systemd Linux init systems in this slice.
- Changing Mother runtime protocol behavior.

## Target Shape

- macOS backend: launchd (current behavior, hardened).
- Linux backend: systemd user service.
- Shared CLI contract:
  - `patina mother install`
  - `patina mother uninstall`
  - `patina mother start`
  - `patina mother stop`
  - `patina mother restart`
  - `patina mother status`
- Backend-aware status and actionable diagnostics.

## Solution

1. Introduce backend abstraction for supervisor operations.
2. Preserve launchd implementation under macOS adapter.
3. Add systemd user unit install/start/stop/status adapter for Linux.
4. Add mixed-control guards and remediation messaging.
5. Document unified runbook and backend log paths.

## Implementation Order

1. Add backend detection + status surfacing (no behavior change).
2. Add Linux systemd user install/uninstall implementation.
3. Add `restart` command and unify lifecycle help text/output.
4. Add mixed-control guardrails and remediation messages.
5. Update README runbook and package notes.

## Resolved Decisions

- Use one operator surface with native backend adapters (launchd/systemd-user), matching proven patterns used by systems like Homebrew services and cloudflared.
- Keep service command contract stable (`patina mother start`) and move backend differences behind adapter seams.
- Prefer fail-closed diagnostics over implicit fallback behavior when backend commands are unavailable.

## Verification

```bash
cargo check --workspace -q
cargo test -q commands::mother

# macOS
patina mother install
patina mother status
patina mother uninstall

# Linux
patina mother install
patina mother status
patina mother uninstall
```

## Exit Criteria

See frontmatter `exit_criteria` checklist.

## Build Readiness

- [ ] Backend abstraction reviewed for fail-closed behavior.
- [ ] launchd parity preserved.
- [ ] Linux systemd user flow validated.
- [ ] Docs updated with one operator runbook.
