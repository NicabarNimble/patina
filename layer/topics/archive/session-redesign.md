---
id: session-redesign
version: 1
created_date: 2025-07-29
confidence: medium
oxidizer: nicabar
tags: []
promoted_from: projects/patina
---

# Session System Redesign

**⚠️ DEPRECATED**: This design was explored but not implemented. The current session system uses a simpler approach without git integration and with `### HH:MM - Event Type` format instead of bracketed timestamps. See `layer/topics/development/session-implementation.md` for the current implementation.

## Overview
Redesign sessions to create a clean pipeline from LLM-specific capture to permanent knowledge layer.

## Core Principles

### Timestamps are Script-Generated
- ID is the start timestamp: `20250727-183045`
- All timestamps in format: `[YYYYMMDD-HHMMSS]`
- Scripts inject timestamps, LLM fills context between them
- No LLM-generated timestamps (inconsistent and error-prone)

### Minimal Metadata
- **ID**: The start timestamp
- **Title**: User's exact input  
- **LLM**: Which adapter (claude/gemini)
- No git info in main session (separate concern)

## New Architecture

### File Structure
```
# During session
.claude/context/
├── active-session.md      # THE current session
├── last-session.md        # Points to layer/sessions/[ID].md
└── sessions/              # Archive of all Claude sessions
    └── 20250727-183045.md

# After session-end
layer/sessions/
├── 20250727-183045.md     # Processed session (flat structure)
├── 20250727-135823.md
└── 20250726-094512.md
```

## Session Format

```markdown
# Session: fix auth system
**ID**: 20250727-183045
**LLM**: claude

## Activity Log

[20250727-183045] Session started

[20250727-184532] Implemented JWT refresh mechanism
<!-- Claude fills in work context here -->

[20250727-185001] Note: Found race condition in token refresh

[20250727-185234] Fixed race condition with mutex
<!-- Claude fills in solution details -->

[20250727-192005] Session ended
```

### How It Works

1. **Scripts generate timestamps**:
   ```bash
   echo "[$(date +%Y%m%d-%H%M%S)] Note: $*" >> active-session.md
   ```

2. **Claude fills context** between timestamps:
   - Describes what happened
   - Explains decisions
   - Documents solutions

3. **Consistent timeline** without LLM timestamp errors

## Session Lifecycle

1. **Start**: `/session-start "fix auth system"`
   ```markdown
   # Session: fix auth system
   **ID**: 20250727-183045
   **LLM**: claude
   
   ## Activity Log
   
   [20250727-183045] Session started
   ```

2. **During**: 
   - `/session-update` adds: `[20250727-184532] Update`
   - `/session-note "found bug"` adds: `[20250727-185001] Note: found bug`
   - Claude fills in context between timestamps

3. **End**: `/session-end`
   - Adds: `[20250727-192005] Session ended`
   - Archives to `sessions/20250727-183045.md`
   - Calls: `patina layer session active-session.md`

## Implementation Notes

### Challenge: Script/LLM Coordination
- Scripts own timestamps and structure
- LLM owns context and descriptions
- Clear separation prevents timestamp chaos

### Future: Git Integration
- Could add `/session-git` command
- Adds git markers: `[20250727-185500] Git: committed abc123`
- Keeps git separate from main flow

## Benefits
1. Consistent timestamps throughout
2. No timestamp duplication or conflicts
3. Clean script/LLM separation
4. Searchable by time markers
5. Living document with clear timeline