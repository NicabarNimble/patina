---
type: fix
id: claude-session-skill-subagent
status: active
created: 2026-03-12
sessions:
  origin: 20260311-232857
related:
- session-skill-convergence
exit_criteria:
- id: session-start-md-wraps-bash-last-session-reads-in-agent-call
  text: session-start.md wraps Bash + last-session reads in Agent call
  checked: true
- id: session-update-md-wraps-bash-artifact-read-in-agent-call
  text: session-update.md wraps Bash + artifact read in Agent call
  checked: true
- id: session-end-md-wraps-end-bash-call-in-agent-call
  text: session-end.md wraps end Bash call in Agent call
  checked: true
- id: session-note-md-unchanged
  text: session-note.md unchanged
  checked: true
- id: no-changes-to-opencode-gemini-or-shell-wrappers
  text: No changes to OpenCode, Gemini, or shell wrappers
  checked: true
- id: deployed-via-build-install-refresh
  text: Deployed via build, install, refresh
  checked: true
---
# fix: Wrap Claude session skill Bash calls in subagents to hide JSON from verbose output

> Session skills (start, update, end) output verbose JSON via Bash tool, which floods the screen when Claude Code verbose mode is on. Wrapping the Bash calls in Agent subagents keeps JSON in agent context and only surfaces clean summaries to the user.

## Problem

When Claude Code verbose mode is on, session skills (`/session-start`, `/session-update`, `/session-end`) dump full JSON output from `patina ai session` to the user's screen. This is noisy and obscures the actual session workflow. Users want verbose mode for general tool visibility but not for session lifecycle JSON.

## Root Cause

The session command files (`resources/claude/session-*.md`) instruct Claude to run `.claude/bin/session-*.sh` directly via the Bash tool. In verbose mode, all Bash tool output is displayed on screen — there's no per-command verbosity control.

## Scope

Claude Code only. OpenCode and Gemini adapters are out of scope — they have different verbosity models and may not have an equivalent Agent tool.

Files to modify:
- `resources/claude/session-start.md` — wrap Bash + context gathering in Agent
- `resources/claude/session-update.md` — wrap Bash call in Agent
- `resources/claude/session-end.md` — wrap end Bash call in Agent
- `resources/claude/session-note.md` — **no change** (no `--json` flag, lightweight)

Shell wrappers (`.claude/bin/session-*.sh`) and Rust source are unchanged.

## Fix

Restructure the three JSON-producing command files to instruct Claude to launch an Agent (subagent) for the Bash execution step. The agent runs the shell wrapper, parses JSON, gathers any needed file content (e.g., last-session reference), and returns structured results to the main context. The main context then handles artifact writing, user interaction, and presentation.

Pattern per skill:
1. Agent runs `.claude/bin/session-*.sh`, parses JSON, reads referenced files
2. Agent returns key fields and file content to main context
3. Main context writes to session artifact, presents clean summary to user

`session-note` stays as-is — it has no `--json` flag and minimal output.

## Exit Criteria

- [x] `resources/claude/session-start.md` wraps Bash + last-session reads in an Agent call
- [x] `resources/claude/session-update.md` wraps Bash + artifact read in an Agent call
- [x] `resources/claude/session-end.md` wraps end Bash call in an Agent call
- [x] `resources/claude/session-note.md` is unchanged
- [x] No changes to OpenCode, Gemini, or shell wrappers
- [x] Deploy via `cargo build --release && cargo install --path .` then `patina ai refresh`
