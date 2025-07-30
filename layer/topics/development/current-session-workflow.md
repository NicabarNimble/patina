---
id: current-session-workflow
version: 1
created_date: 2025-07-29
confidence: medium
oxidizer: nicabar
tags: []
---

# Current Session Workflow

## Overview
The Patina session system provides a simple, effective way to capture development context during AI-assisted coding sessions. It uses slash commands integrated with Claude Code.

## How It Works

### 1. Starting a Session
```bash
/session-start "your session goal"
```
- Creates `.claude/context/active-session.md` with session metadata
- Generates unique session ID based on timestamp (YYYYMMDD-HHMMSS)
- AI reads previous session context if available
- AI asks if you want to create todos for the session goal

### 2. During Development
```bash
/session-update
```
- Adds a timestamp marker showing time span (e.g., "### 14:30 - Update (covering since 14:15)")
- AI fills in what happened during that time span:
  - Work completed
  - Key decisions and reasoning
  - Challenges faced and solutions
  - Patterns observed

### 3. Ending a Session
```bash
/session-end
```
- Archives the active session to two locations:
  - `.claude/context/sessions/{SESSION_ID}.md` (local Claude context)
  - `layer/sessions/{SESSION_ID}.md` (permanent knowledge store)
- Updates `.claude/context/last-session.md` with pointer to archived session
- Cleans up active session files

## Session File Format

```markdown
# Session: your session goal
**ID**: 20250728-143052
**Started**: 2025-07-28T14:30:52Z
**LLM**: claude

## Previous Session Context
[AI-generated summary of last session]

## Goals
- [ ] your session goal

## Activity Log
### 14:30 - Session Start
Session initialized with goal: your session goal

### 14:45 - Update (covering since 14:30)
[AI fills in work context here]

### 15:30 - Update (covering since 14:45)
[AI fills in more work context]
```

## Key Design Decisions

### No Git Integration
Sessions focus purely on work intent and progress, not environmental context. Git branch names were causing incorrect assumptions about session goals.

### Script-Owned Timestamps
Shell scripts generate all timestamps in consistent format. AI never generates timestamps, only fills context between them.

### Simple Time Tracking
Uses `.claude/context/.last-update` file to track time spans between updates, enabling "covering since HH:MM" messages.

### Dual Archive
Sessions are stored in both:
- `.claude/context/sessions/` - For Claude Code context
- `layer/sessions/` - For permanent knowledge accumulation

## Additional Commands

### /session-note "your insight"
```bash
/session-note "found that the bug was due to race condition"
```
- Adds a timestamped note to the active session
- Captures human insights verbatim
- Useful for marking important discoveries or decisions
- Format: "### HH:MM - Note" followed by your text

## What's NOT Implemented

### Automated Features
No hooks, sub-agents, or enforcement mechanisms. The system relies on manual command usage.

### Smart Session Naming
User input is used directly as the session title without pre-processing or sanitization beyond basic filesystem safety.

## Best Practices

1. **Start with Clear Goals**: Use descriptive session titles that explain what you're trying to accomplish

2. **Update Regularly**: Run `/session-update` at natural breakpoints or when switching between different aspects of work

3. **Let AI Fill Context**: Don't manually edit the activity log - let the AI document based on the conversation and code changes

4. **End Sessions Cleanly**: Always run `/session-end` before starting a new session or switching projects

## Integration with Patina

Sessions feed into Patina's knowledge layer:
- Patterns discovered in sessions can be promoted to topics or core patterns
- Session archives provide historical context for future work
- The layer system allows knowledge to evolve from project-specific to universal

## File Locations

- **Active Session**: `.claude/context/active-session.md`
- **Last Session Pointer**: `.claude/context/last-session.md`
- **Session Archives**: `.claude/context/sessions/` and `layer/sessions/`
- **Shell Scripts**: `resources/claude/session-*.sh`
- **Command Definitions**: `.claude/commands/session-*.md`