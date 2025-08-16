# /git-note

Capture Git-specific insights and patterns for memory building.

## Usage

```
/git-note <insight text>
```

## Description

Records Git-related insights with intelligent pattern detection:
- Captures notes with current Git context
- Detects mentioned files and shows their survival metrics
- Identifies pattern indicators (always/never/tends to)
- Recognizes failure indicators (valuable negative knowledge)
- Tracks success indicators (solutions worth remembering)
- Shows co-modification patterns for mentioned files
- Provides context-aware memory tips
- Maintains cumulative insights file

Notes are stored in `.claude/context/git-work/insights.md` and added to current work session if active.

## Philosophy

**Build Memory from Patterns**: Every insight contributes to understanding code survival and evolution patterns.

## Pattern Detection

The command recognizes:
- **Pattern indicators**: "always", "never", "every time", "pattern", "tends to", "usually"
- **Failure indicators**: "failed", "didn't work", "broke", "error", "issue", "problem"
- **Success indicators**: "fixed", "solved", "works", "success", "better"

## Examples

```
/git-note The auth module always needs updating when user.rs changes
/git-note Circular dependency issue between parser and lexer modules
/git-note Fixed memory leak by clearing cache on route change
/git-note This refactoring pattern tends to break tests in edge cases
```

## Memory Tips

- Failed experiments prevent repeated mistakes
- Patterns might belong in layer/topics/ or layer/core/
- Solutions that work should be committed with good messages

## Related Commands

- `/git-start` - Begin Git work tracking
- `/git-update` - Capture progress
- `/git-end` - Conclude and analyze patterns