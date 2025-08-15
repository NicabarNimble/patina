# /git-start

Start Git-aware work tracking with memory and survival insights.

## Usage

```
/git-start [work description]
```

## Description

Begins tracking Git-related work with focus on code survival patterns and memory building. Shows:
- Previous attempts at similar work
- Failed experiments (exp/ branches) 
- Current Git state and uncommitted changes
- Code survival insights from long-lived files
- Git best practices reminders

Creates a tracking file in `.claude/context/git-work/current.md`.

## Philosophy

**Information over Automation**: Provides Git context and memory without forcing workflows. The LLM does the Git work, this command provides the memory.

## Examples

```
/git-start retry logic
/git-start "fix memory leak"
/git-start refactor-auth
```

## Related Commands

- `/git-update` - Capture progress during work
- `/git-note` - Record Git-specific insights
- `/git-end` - Conclude work and analyze patterns