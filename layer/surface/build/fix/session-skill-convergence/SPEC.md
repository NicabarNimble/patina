---
type: fix
id: session-skill-convergence
status: draft
created: 2026-03-12
sessions:
  origin: 20260311-223303-3EP2
related:
- native-session-runtime-binding
- cli-mcp-skill-unification
exit_criteria:
- id: tag-format-cleanup
  description: "Git tags use session-YYYYMMDD-HHMMSS-interface-{start|end} format — no random suffix"
  checked: false
- id: opencode-wrappers-fixed
  description: "OpenCode shell wrappers use `patina ai session` with --json, --adapter, and PATINA_AI_INTERFACE env var"
  checked: false
- id: opencode-commands-use-wrappers
  description: "OpenCode command files reference .opencode/bin/ wrappers instead of inlining CLI calls"
  checked: false
- id: session-note-parity
  description: "/session-note referenced in all adapter workflow reminders"
  checked: false
- id: shared-guidance-converged
  description: "Wikilink conventions, belief capture, and checkpoint commit guidance present in all adapters"
  checked: false
- id: multi-session-handling
  description: "Multiple active session handling documented in all adapter commands"
  checked: false
---
# fix: Session Skill Convergence & Tag Format Cleanup

> Session skills drifted across adapters (Claude/OpenCode) and git tag format accumulated unnecessary complexity (random 4-char suffix). Converge command files, fix shell wrappers, and restore clean tag format.

## Problem

Three issues compounded:

1. **Tag format bloat**: Original clean `session-YYYYMMDD-HHMMSS-{start|end}` grew to `session-YYYYMMDD-HHMMSS-XXXX-adapter-{start|end}`. The random 4-char suffix prevents collisions that effectively can't happen in a single-user project.

2. **OpenCode wrapper/command drift**: OpenCode command files inline raw CLI calls (`patina ai session start --json --adapter opencode`) while OpenCode shell wrappers use a completely different CLI surface (`patina session start` — no `ai`, no `--json`, no `--adapter`, no env var). The wrappers and commands disagree.

3. **Guidance drift**: Claude commands include wikilink conventions, belief capture prompts, `/session-note` workflow, and checkpoint commit suggestions. OpenCode commands lack all of these. OpenCode does handle multiple active sessions — Claude doesn't.

## Root Cause

Multi-interface support was added incrementally across several specs (`native-session-runtime-binding`, MCP lifecycle removal). Each change updated Claude's adapter files but left OpenCode behind. The random suffix was added to the session ID format (`ids.rs`) for collision safety but was overkill.

## Fix

### 1. Tag format: drop random suffix

In `src/session/internal/ids.rs`: revert file_id generation to `YYYYMMDD-HHMMSS` (no 4-char suffix). Tags become `session-YYYYMMDD-HHMMSS-interface-{start|end}`.

Keep the interface name in tags — it enables `git tag -l '*-claude-*'` filtering and prevents cross-interface same-second collision.

### 2. OpenCode wrappers: match Claude pattern

`.opencode/bin/session-*.sh` should mirror Claude's wrappers:
```bash
#!/bin/bash
exec env PATINA_AI_INTERFACE=opencode patina ai session start --json --adapter opencode "$@"
```

### 3. OpenCode commands: reference wrappers

`.opencode/commands/session-*.md` should reference `.opencode/bin/session-*.sh` wrappers instead of inlining CLI calls. Remove the "read AGENTS.md first" preamble — the wrappers handle adapter identity.

### 4. Converge shared guidance

All adapter command files should include:
- Wikilink conventions (`[[belief-id]]`, `[[session-id]]`, etc.)
- Belief capture suggestions in `/session-update`
- `/session-note` in workflow reminders
- Checkpoint commit suggestions
- Multiple active session handling

### 5. Back-port OpenCode innovations to Claude

- Multiple session list/selector handling
- `--note` parameter on session-end

## Exit Criteria
