---
id: command-refactoring-decisions
version: 2
created_date: 2025-07-30
updated_date: 2025-07-30
confidence: high
oxidizer: nicabar
tags: [architecture, cli, commands, refactoring]
---

# Command Refactoring Decisions

## Context
During review of Patina's CLI commands, we identified confusion between the actual purpose of commands and their implementation. This document captures our decisions about command structure.

## Problem Analysis

### Naming Confusion
- "workspace" appeared as both a dev environment option and a command group
- The workspace service is actually for managing Dagger agent environments
- This created confusion about what "workspace" meant in different contexts

### Dead Commands
Several commands exist but serve no real purpose:
- `patina add` - Pattern management that's never used
- `patina commit` - Git-like pattern commits that don't fit workflow
- `patina push` - Should be automatic, not manual

## Completed Changes

### 1. Workspace → Agent Rename ✅
**Status: Completed in PR #4**

Following the container-use philosophy, we renamed `patina workspace` to `patina agent`:
- Better describes what it does: manages isolated environments for AI agents
- Aligns with container-use patterns (each agent gets its own container)
- Removes confusion with "workspace" as a dev environment
- Internal workspace terminology kept where accurate (directory name, WorkspaceClient)

### 2. Dead Command Removal
**Status: Pending implementation**

Commands to remove:
- `patina add` - Pattern management never used in practice
- `patina commit` - Git-like pattern commits  
- `patina push` - Will become automatic

### 3. Final Command Structure
**Status: Pending implementation**

Streamlined commands after cleanup:
```bash
patina init      # Create project + scaffold
patina build     # Build the project
patina test      # Test the project
patina update    # Update adapters + regenerate context
patina doctor    # Check project health
patina version   # Version info
patina agent     # Manage agent environments (formerly workspace)
  ├── start    # Start the agent service
  ├── stop     # Stop the service
  ├── status   # Check service status
  └── list     # List active environments
```

### 4. Context Generation
**Status: To be implemented**
- `push` becomes automatic in `init` and `update`
- Context files (CLAUDE.md, etc.) stay fresh without manual intervention

### 5. Future Pattern Management
**Status: Deferred**

The concept of `add`/`commit` for patterns is sound but needs rethinking:
- Could become `patina distill` - extract patterns from sessions
- Or stay internal - patterns extracted automatically
- Decision deferred until layer system matures

## Rationale

### Why "agent" not "dagger" or "container"?
- "agent" describes the purpose (AI agents working in isolation)
- "dagger" is an implementation detail
- "container" is too generic
- Follows container-use naming philosophy

### Why remove pattern commands?
- Never used in practice
- Git-like workflow doesn't match how patterns actually emerge
- Sessions → patterns happens organically, not through commands

### Why keep doctor?
- Actually implemented and useful
- Checks environment changes
- Provides health metrics
- Guides users when tools are missing

## Implementation Notes
- Only rename user-facing parts
- Keep internal "workspace" terminology where accurate
- Preserve backward compatibility where possible
- Update help text to remove confusion

## Lessons Learned
1. Commands should reflect user intent, not implementation
2. Dead code confuses both humans and AI assistants
3. Naming is critical - especially when AI is the primary user
4. Start minimal, add commands as patterns emerge

## Next Steps
1. Remove `add`, `commit`, and `push` commands from the codebase
2. Clean up associated code (session management, pattern storage)
3. Make `push` functionality automatic in `init` and `update`
4. Update help text and documentation
5. Ensure CI passes with all changes