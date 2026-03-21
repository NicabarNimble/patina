---
type: belief
id: tmux-lane-defines-active-session
persona: architect
facets: [sessions, tmux, interfaces, runtime-hygiene]
entrenchment: medium
status: scoped
endorsed: true
extracted: 2026-03-16
revised: 2026-03-21
---

# tmux-lane-defines-active-session

An AI session is active only while its interface-specific tmux lane is alive; when the lane dies, Patina must archive the session with handoff context and start a new lane/session pair on next launch.

## Statement

An AI session is active only while its interface-specific tmux lane is alive; when the lane dies, Patina must archive the session with handoff context and start a new lane/session pair on next launch.

## Evidence

- User requirement: the tmux bubble is runtime truth; each interface owns an independent lane and artifact; dead lanes must produce a handoff summary and avoid orphan active sessions.
- [[src/commands/ai/surface.rs]]: Launch now reconciles tmux-bound sessions and archives stale interface sessions with `## Handoff` context before check-in.
- [[src/interface/internal/tmux.rs]]: Tmux launch keeps lane reuse enabled from inside tmux and provides lane liveness checks.

## Supports

- `[[stale-context-is-hostile-context]]`
- `[[context-loss-audit-required]]`

## Attacks

- `[[silent-default-hides-missing-data]]`

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[src/commands/ai/surface.rs]]: `reconcile_tmux_bound_sessions` auto-archives stale active sessions when the interface lane is not alive.
- [[src/interface/internal/tmux.rs]]: `tmux_session_alive` makes lane liveness explicit and queryable.

## Scope Rationale

Scoped by [[session-20260320-212325-011658000]] (2026-03-21). Tmux lanes are agent-launcher infrastructure for Claude Code, OpenCode, and Gemini CLI — they're how `patina ai <interface>` sets up the workspace. But tmux is NOT the session liveness mechanism. Session liveness is defined by the agent's socket connection to Mother. When the socket closes (EOF), Mother archives the session. Agents that don't use tmux (pi, custom scripts, CI pipelines, future apps) still have sessions — they just connect to Mother directly without a tmux wrapper. The surviving principle: dead agent connections must produce handoff context. The mechanism: socket EOF, not tmux lane death.

## Revision Log

- 2026-03-16: Created — metrics computed by `patina scrape`
- 2026-03-16: Linked implementation evidence and belief relationships for tmux-bound session validity.
- 2026-03-21: Scoped — tmux is agent-launcher infrastructure, not session liveness mechanism. Session liveness is socket connection to Mother. Agents without tmux still have sessions. Surviving principle: dead connections produce handoff.
