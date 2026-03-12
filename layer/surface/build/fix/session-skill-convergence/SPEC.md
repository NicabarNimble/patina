---
type: fix
id: session-skill-convergence
status: ready
created: 2026-03-12
sessions:
  origin: 20260311-223303-3EP2
related:
- native-session-runtime-binding
- cli-mcp-skill-unification
exit_criteria:
- id: tag-format-cleanup
  text: Git tags use session-YYYYMMDD-HHMMSS-interface-{start|end} format — no random suffix
  checked: false
- id: opencode-wrappers-fixed
  text: OpenCode shell wrappers use `patina ai session` with --json, --adapter, and PATINA_AI_INTERFACE env var
  checked: false
- id: opencode-commands-use-wrappers
  text: OpenCode command files reference .opencode/bin/ wrappers instead of inlining CLI calls
  checked: false
- id: session-note-parity
  text: /session-note referenced in all adapter workflow reminders
  checked: false
- id: shared-guidance-converged
  text: Wikilink conventions, belief capture, and checkpoint commit guidance present in all adapters
  checked: false
- id: multi-session-handling
  text: Multiple active session handling documented in all adapter commands
  checked: false
- id: skill-tone-restored
  text: Session skills restore the human tone and lost features from golden era — work classification, commit thresholds, natural language guidance — adapted for Mother-backed architecture
  checked: false
- id: mother-architecture-accurate
  text: Skills correctly reference Mother-backed flow (artifact_path from JSON, durable layer/sessions/ artifacts, interface pointers) — no references to deleted active-session.md or .claude/context/ paths
  checked: false
- id: gemini-wrappers-fixed
  text: Gemini shell wrappers use `patina ai session` with --json, --adapter, and PATINA_AI_INTERFACE env var
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

### 6. Restore skill tone and lost features

The golden-era commands (pre-`b1b49ef9`) had personality and practical guidance that got stripped during the architecture migration. Restore:

- **Work classification**: Exploration / Experiment / Feature based on commit patterns
- **Commit thresholds**: "30+ minutes or 100+ lines changed" as checkpoint triggers
- **Natural language**: "Don't write generic fluff — include specific accomplishments"
- **Inline git context**: "The script will show what time period to document"
- **Discussion context prompt**: Key questions asked, reasoning frameworks, why we chose this approach

These must be adapted to the new Mother-backed architecture — the old versions referenced `.claude/context/active-session.md` and shell scripts that did git work directly. The new versions should:
- Reference `artifact_path` from wrapper JSON output (not active-session.md)
- Store durable artifacts in `layer/sessions/` (not `.claude/context/sessions/`)
- Use `.patina/local/last-session.md` for previous session pointer
- Acknowledge Mother owns the session lifecycle (start/sync/archive)
- Keep the wrappers as the invocation path (not raw CLI or MCP)

### 7. Fix Gemini wrappers

`.gemini/bin/session-*.sh` need the same treatment as OpenCode — `patina ai session` with `--json`, `--adapter gemini`, and `PATINA_AI_INTERFACE=gemini`.

## Exit Criteria
