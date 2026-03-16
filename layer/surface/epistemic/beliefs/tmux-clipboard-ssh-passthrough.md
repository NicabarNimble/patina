---
type: belief
id: tmux-clipboard-ssh-passthrough
persona: architect
facets: [operations, session-capture, tooling]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-15
revised: 2026-03-15
---

# tmux-clipboard-ssh-passthrough

For SSH tmux sessions, clipboard reliability depends on tmux clipboard passthrough settings and reloading each active tmux socket after config changes.

## Statement

For SSH tmux sessions, clipboard reliability depends on tmux clipboard passthrough settings and reloading each active tmux socket after config changes.

## Evidence

- Observed in session 20260315-081718-XVA6: OpenCode reported copied text without clipboard updates until tmux servers were reloaded with set-clipboard on, copy-command pbcopy, and allow-passthrough on.

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-03-15: Created — metrics computed by `patina scrape`
