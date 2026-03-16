---
type: belief
id: tmux-lane-defines-active-session
persona: architect
facets: [sessions, tmux, interfaces, runtime-hygiene]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-16
revised: 2026-03-16
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

## Revision Log

- 2026-03-16: Created — metrics computed by `patina scrape`
- 2026-03-16: Linked implementation evidence and belief relationships for tmux-bound session validity.
