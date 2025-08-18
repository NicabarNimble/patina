# /session-git-note

Capture insights and discoveries with Git context for searchable memory.

**Testing Version**: This command will eventually replace `/session-note` once validated.

## Usage

```
/session-git-note <insight text>
```

## Description

Records human insights with automatic Git context (branch and commit SHA). This creates searchable memory that future sessions can reference.

## Git Integration Features

- Adds Git context: `[branch@sha]` to every note
- Detects important insights (breakthrough, discovered, solved, fixed)
- Suggests commits for significant discoveries
- Creates searchable memory through Git history

## Smart Detection

The command detects important insights containing keywords:
- "breakthrough"
- "discovered"
- "solved"
- "fixed"
- "important"

When detected, it suggests creating a checkpoint commit to preserve the context.

## Philosophy

**Notes as Memory**: Every insight tied to a specific Git state becomes searchable knowledge. Future sessions can ask "when did we solve the auth problem?" and find the exact commit.

## Examples

```
/session-git-note "discovered the memory leak is in the connection pool"
→ 💡 Important insight detected!
→ Consider committing: git commit -am "checkpoint: discovered the memory leak..."

/session-git-note "TODO: refactor this later"
→ ✓ Note added [session/20250818-auth@abc123f]

/session-git-note "the async approach isn't working, trying sync instead"
→ ✓ Note added [session/20250818-auth@def456a]
```

## Memory Building

Notes with Git context enable powerful queries:
- "What branch did we fix the race condition on?"
- "Show all insights from authentication work"
- "What experiments failed last month?"

## Related Commands

- `/session-git-start` - Begin session with Git branch
- `/session-git-update` - Track progress with Git awareness  
- `/session-git-end` - Conclude and classify work