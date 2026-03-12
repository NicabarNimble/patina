---
type: fix
id: claude-session-skill-subagent
status: draft
created: 2026-03-12
sessions:
  origin: 20260311-232857
related:
- session-skill-convergence
exit_criteria: []
---
# fix: Wrap Claude session skill Bash calls in subagents to hide JSON from verbose output

> Session skills (start, update, end) output verbose JSON via Bash tool, which floods the screen when Claude Code verbose mode is on. Wrapping the Bash calls in Agent subagents keeps JSON in agent context and only surfaces clean summaries to the user.

## Problem

## Root Cause

## Fix

## Exit Criteria
