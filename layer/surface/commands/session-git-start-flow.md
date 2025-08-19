---
id: session-git-start-flow
status: active
created: 2025-08-17
updated: 2025-08-17
tags: [commands, git-integration, session-management, llm-workflow, testing]
references: [git-hooks-integration.md, pattern-selection-framework.md]
---

# /session-git-start Command Flow (Testing Version)

## Purpose
Testing version of deeply integrated session+git commands. Once proven, will replace standard `/session-start`.

## Important: Testing Strategy
- This is `/session-git-start` for testing in fresh Claude instance
- Preserves all original session functionality
- Adds mandatory git integration
- Once validated, will become the new `/session-start`

## Simple Flow (Session + Git Memory)

### 1. Single Command Execution
```bash
/session-git-start "exploring authentication options"
```

### 2. Git Branch Creation
```bash
# Automatically executed
BRANCH="session/$(date +%Y%m%d-%H%M%S)-${SESSION_NAME// /-}"

# Check but don't block
if [[ -n $(git status --porcelain) ]]; then
    echo "⚠️ Warning: Uncommitted changes exist"
    echo "   Consider: git stash or git commit -am 'WIP'"
fi

git checkout -b "$BRANCH"
echo "✅ Memory branch created: $BRANCH"
```

### 3. Context Preparation
```markdown
# Added to active-session.md header
Git Branch: session/20250817-140000-exploring-authentication-options
Parent Branch: main (or current branch)
Starting Commit: abc123f
```

### 4. LLM Instructions Injection
After session starts, automatically remind LLM:
```markdown
📍 Session started on branch: session/[timestamp]-[topic]

Remember to:
- Make small, focused commits (scalpel not shotgun)
- Commit working code before major changes
- Use descriptive commit messages
- This is an exploration branch - failed experiments are valuable
```

### 5. Memory Setup
- Branch created = memory container
- All commits will be preserved
- Failed experiments are valuable

## Error Handling

### Dirty Working Tree
```bash
if git status --porcelain | grep -q .; then
    echo "⚠️ Uncommitted changes detected"
    echo "Options:"
    echo "  1. Commit current work: git commit -am 'WIP: saving work'"
    echo "  2. Stash changes: git stash"
    echo "  3. Force proceed (not recommended): /session-start --force"
fi
```

### Already in Session Branch
```bash
if git branch --show-current | grep -q "^session/"; then
    echo "📍 Already in session branch"
    echo "Options:"
    echo "  1. Continue current session: /session-update"
    echo "  2. End current and start new: /session-end && /session-start"
fi
```

## Configuration Options

```toml
# .patina/config.toml
[session.git]
auto_branch = true                    # Create git branches automatically
branch_prefix = "session"             # Prefix for session branches
require_clean = false                 # Don't block on dirty tree (just warn)
```

## LLM Coaching Text

Include in session start output:
```
🚀 Session Strategy:
- This is YOUR exploration space - experiment freely
- Commit early and often - git remembers everything
- Failed attempts are valuable - they become memory
- Think of commits as checkpoints in a game
- The messier the exploration, the more we learn
```

## Implementation Notes

### Core Functionality
- Create a git branch for the session
- Add branch name to active-session.md
- Preserve all branches as permanent memory

### Future Enhancements
- Pattern extraction from commit history
- Success/failure analysis from git data
- Automated PR creation from successful sessions

## Success Metrics
- Branch created successfully
- No uncommitted work lost
- LLM acknowledges git workflow
- First commit within 10 minutes

## Example Full Flow
```bash
$ /session-start "add user authentication"

Creating session: 20250817-140000-add-user-authentication
✓ Git branch created: session/20250817-140000-add-user-authentication
✓ Active session file initialized
✓ Previous session context preserved
✓ Pattern tracking activated

📍 You're now on session branch: session/20250817-140000-add-user-authentication

Remember: Commit early and often. Every experiment has value.
First checkpoint suggested after initial exploration (~10 mins).

Ready to explore user authentication options...
```