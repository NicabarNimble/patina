---
id: session-git-update-flow
status: active
created: 2025-08-17
tags: [commands, git-integration, session-management, llm-workflow, testing]
references: [session-git-start-flow.md, git-hooks-integration.md]
---

# /session-git-update Command Flow (Testing Version)

## Purpose
Testing version of deeply integrated session+git update command. Once proven, will replace standard `/session-update`.

## Important: Testing Strategy
- This is `/session-git-update` for testing in fresh Claude instance
- Preserves all original session functionality
- Adds mandatory git status checks and reminders
- Once validated, will become the new `/session-update`

## Simple Flow (Session + Git Memory)

### 1. Single Command Execution
```bash
/session-git-update
```

First, check git state:
```bash
# Automatically executed
git status --short
git diff --stat
git log --oneline -5 --decorate
```

### 2. Intelligent Git Reminder

Based on git state, provide contextual guidance:

#### Clean Working Tree
```markdown
✅ Git status: Clean
Last commit: 5 minutes ago
Great job keeping commits current!
```

#### Uncommitted Changes (Small)
```markdown
📝 Git status: 3 files modified (47 lines)
Last commit: 15 minutes ago

Suggested action:
- Review changes: git diff
- Commit if working: git commit -am "feat: implement login form"
```

#### Uncommitted Changes (Large)
```markdown
⚠️ Git status: 12 files modified (500+ lines)
Last commit: 45 minutes ago

Strong recommendation: Break this into smaller commits
- Stage related changes: git add -p
- Commit by feature: git commit -m "feat: add user model"
- Continue with next piece
```

#### Many WIP Commits
```markdown
🔄 Git status: 8 WIP commits since session start

Consider:
- Squashing related commits: git rebase -i HEAD~8
- Or keep as-is for full exploration history
- Decision can wait until session-end
```

### 3. Commit Reminder
```bash
# Simple reminder based on time and changes
if [[ $UNCOMMITTED_LINES -gt 100 ]] || [[ $TIME_SINCE_COMMIT -gt 30 ]]; then
    echo "💡 Consider committing your work"
    echo "   git commit -am 'checkpoint: progress on $CURRENT_WORK'"
fi
```

### 4. Context Enhancement
Add to session update output:
```markdown
## Git Activity Summary
- Current branch: session/20250817-140000-auth
- Commits this session: 5
- Files changed: 8
- Patterns detected: authentication-flow, user-validation
- Code survival rate: 85% (good stability)
```

### 5. Smart Reminders

Based on time and changes:

#### Every 30 minutes
```markdown
🕐 Checkpoint reminder:
Have you committed your recent work?
Remember: Commits are free, lost work is expensive.
```

#### After Error/Test Failure
```markdown
🔴 Test failure detected
Before fixing: Commit current state as "WIP: failing test for X"
This preserves the learning moment.
```

#### Before Major Refactor
```markdown
🔧 Large refactor detected (50+ lines changing)
Strongly recommend: Commit current working state first
git commit -am "checkpoint: before refactoring auth module"
```

## LLM Coaching Patterns

### The Scalpel Approach
```markdown
📍 Git Philosophy Reminder:
Instead of one big commit changing everything:
- auth/user.rs: "add user model"
- auth/login.rs: "implement login logic"  
- tests/auth_test.rs: "add auth tests"

Each commit should have ONE clear purpose.
```

### The Learning Preservation
```markdown
💡 Failed experiment? Still commit it!
git commit -am "attempt: tried JWT auth (didn't work due to X)"

These commits become searchable memory for future sessions.
```

## Configuration Options

```toml
# .patina/config.toml
[session.update]
auto_git_check = true                    # Check git status on update
reminder_threshold_minutes = 30          # Remind after X minutes without commit
reminder_threshold_lines = 100          # Remind after X lines changed
show_patterns = true                    # Show detected patterns
show_survival_rate = true              # Show code stability metrics
```

## Implementation Notes

### Core Functionality
- Show git status to maintain awareness
- Suggest commits at reasonable intervals
- Keep commits as memory checkpoints

### Future Enhancements
- Pattern detection from commit messages
- Code survival tracking
- Session health metrics

## Contextual Suggestions

### Based on Git State
```bash
# Simple reminder logic
if [[ $UNCOMMITTED_LINES -eq 0 ]]; then
    echo "✅ All work committed"
elif [[ $UNCOMMITTED_LINES -lt 50 ]]; then
    echo "Small changes pending - commit when ready"
else
    echo "⚠️ Large uncommitted changes - consider committing"
fi
```

### Based on Session Duration
```bash
# Simple bash logic
if [[ $SESSION_MINUTES -lt 30 ]]; then
    echo "Early exploration - commit when something works"
elif [[ $SESSION_MINUTES -lt 90 ]]; then
    echo "Deep work - remember regular checkpoints"
else
    echo "Long session - definitely commit your progress"
fi
```

## Example Full Flow

```bash
$ /session-update

📊 Session Progress: 2h 15m on session/20250817-140000-auth

Git Status:
- 5 files modified (127 lines)
- Last commit: 22 minutes ago "feat: add password validation"
- 3 patterns detected this session

⚠️ Recommendation: Commit current work
You have meaningful changes in auth/validator.rs

Suggested commit:
  git add auth/validator.rs tests/validator_test.rs
  git commit -m "feat: add password strength validation"

Session Health: 🟡 Good (would be 🟢 with more frequent commits)

Continue with your excellent work on authentication!
```

## Success Metrics
- LLM acknowledges git status
- Commits happen within reminder threshold
- No work lost to uncommitted changes
- Clean, semantic commit history emerging