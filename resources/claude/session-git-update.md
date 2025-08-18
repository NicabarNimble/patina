# /session-git-update

Update current session with Git-aware progress tracking and commit coaching.

**Testing Version**: This command will eventually replace `/session-update` once validated.

## Usage

```
/session-git-update
```

## Description

Captures session progress with integrated Git status awareness. This command:

1. **Shows Git status** - Current branch, changes, last commit
2. **Provides smart reminders** - Based on time and change volume
3. **Tracks session health** - Visual indicator of commit hygiene
4. **Coaches best practices** - Gentle reminders about small commits

## Git Integration Features

- Shows uncommitted changes count and line volume
- Displays recent commits (last 5)
- Time-based commit reminders (30+ minutes)
- Volume-based suggestions (100+ lines)
- Session health indicator (🟢 🟡 🔴)

## Smart Reminders

### Based on Time
- **< 30 minutes**: Continue working
- **30-60 minutes**: Gentle checkpoint reminder
- **> 1 hour**: Strong recommendation to commit

### Based on Changes
- **< 50 lines**: Small changes, commit when ready
- **50-100 lines**: Consider committing soon
- **> 100 lines**: Break into smaller commits

## Philosophy

**Checkpoint Culture**: Commits are free, lost work is expensive. Every checkpoint is a save point in your exploration game.

## Examples

Clean working tree:
```
✅ Git status: Clean
Last commit: 5 minutes ago
Session Health: 🟢 Excellent
```

Large uncommitted changes:
```
⚠️ Git status: 12 files modified (500+ lines)
Last commit: 45 minutes ago
Session Health: 🟡 Good (commit recommended)
```

## Related Commands

- `/session-git-start` - Begin session with Git branch
- `/session-git-note` - Capture insights with Git context
- `/session-git-end` - Conclude and classify work