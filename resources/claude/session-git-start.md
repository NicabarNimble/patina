# /session-git-start

Start a new Patina development session with integrated Git memory tracking.

**Testing Version**: This command will eventually replace `/session-start` once validated.

## Usage

```
/session-git-start [session name]
```

## Description

Begins a new development session with automatic Git branch creation for memory persistence. This command:

1. **Creates a session branch** - Every session gets its own Git branch for exploration
2. **Preserves context** - Links to previous session and parent branch
3. **Coaches Git workflow** - Reminds about commit best practices
4. **Tracks everything** - Failed experiments are valuable memory

## Git Integration Features

- Automatic branch creation: `session/[timestamp]-[name]`
- Warns about uncommitted changes (but doesn't block)
- Records parent branch and starting commit
- Provides Git workflow coaching

## Philosophy

**Memory through Git**: Every session is a branch, every commit is a memory checkpoint. Failed experiments are as valuable as successes - they prevent repeating mistakes.

## Examples

```
/session-git-start "implement authentication"
/session-git-start "debug memory leak"
/session-git-start exploration
```

## Session Strategy

When you start a session:
- You're in YOUR exploration space - experiment freely
- Commit early and often - git remembers everything
- Failed attempts are valuable - they become memory
- Think of commits as checkpoints in a game
- The messier the exploration, the more we learn

## Related Commands

- `/session-git-update` - Track progress with Git awareness
- `/session-git-note` - Capture insights with Git context
- `/session-git-end` - Conclude session and classify work