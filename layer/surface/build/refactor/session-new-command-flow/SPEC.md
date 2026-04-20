---
type: refactor
id: session-new-command-flow
status: draft
created: 2026-04-20
sessions:
  origin: 20260419-160913-422415000
related:
- src/commands/ai/mod.rs
- src/commands/ai/internal.rs
- src/commands/session/internal.rs
- src/interface/runtime/templates.rs
- resources/claude/session-start.md
- resources/opencode/session-start.md
- resources/gemini/session-start.toml
- .pi/prompts/session-start.md
exit_criteria: []
---
# refactor: Rename /session-start to /session-new and align auto-session naming

> Replace session-start command surface with session-new across interfaces and runtime templates; remove start alias and CLI start command; add first-update naming hook to propose/refine session title for auto-created sessions; keep git tag semantics (start/end tag IDs stable) and surface linkability in session artifacts.

## Problem

## Goal

## Status

## Non-Goals

## Current State

## Target State

## Solution

## Implementation Order

## Resolved Decisions

## Verification

## Exit Criteria

## Build Readiness
