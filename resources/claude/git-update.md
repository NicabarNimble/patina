# /git-update

Update Git work tracking with current state and survival metrics.

## Usage

```
/git-update
```

## Description

Captures the current Git state during active work:
- Time elapsed in work session
- Commits made during this session
- Current uncommitted changes with statistics
- Files being modified together (co-modification patterns)
- Survival patterns in modified files
- Rotating Git memory tips

Updates the tracking file with progress markers.

## Philosophy

**Track Survival**: Shows which files have survived over time and how many commits they've been through. Code that survives = good patterns.

## Examples

```
/git-update
```

Call periodically during work to build memory of the development process.

## Related Commands

- `/git-start` - Begin Git work tracking
- `/git-note` - Capture specific insights
- `/git-end` - Conclude and archive work