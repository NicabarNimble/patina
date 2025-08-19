---
id: session-git-end-flow
status: active
created: 2025-08-17
tags: [commands, git-integration, session-management, llm-workflow, testing]
references: [session-git-start-flow.md, session-git-update-flow.md, git-hooks-integration.md]
---

# /session-git-end Command Flow (Testing Version)

## Purpose
Testing version of deeply integrated session+git end command. Once proven, will replace standard `/session-end`.

## Important: Testing Strategy
- This is `/session-git-end` for testing in fresh Claude instance
- Preserves all original session functionality
- Adds mandatory git analysis and branch classification
- Once validated, will become the new `/session-end`

## Simple Flow (Session + Git Memory)

### 1. Single Command Execution
```bash
/session-git-end
```

First, ensure clean state:
```bash
# Check for uncommitted work
if git status --porcelain | grep -q .; then
    echo "⚠️ Uncommitted changes detected!"
    echo "Please commit or stash before ending session"
    exit 1
fi
```

### 2. Session Summary

```bash
# Simple metrics for awareness
COMMITS=$(git rev-list --count main..HEAD)
FILES_CHANGED=$(git diff --name-only main..HEAD | wc -l)

echo "Session Complete:"
echo "  Commits made: $COMMITS"
echo "  Files touched: $FILES_CHANGED"
echo "  Branch preserved: $(git branch --show-current)"
```

### 3. Branch Preservation

```bash
# The session branch is permanent memory - never delete it
echo "Branch preserved as permanent memory"
echo "To revisit this work: git checkout session/$SESSION_ID"

# Optional: Create a feature branch if ready for PR
echo ""
echo "If ready for PR, you can:"
echo "  git checkout -b feature/[name]"
echo "  git merge --squash session/$SESSION_ID"
```

### 4. Session Archive

```bash
# Archive session to layer/sessions/ with git context
cat > "layer/sessions/${SESSION_ID}.md" << EOF
# Session: $SESSION_TITLE
**ID**: $SESSION_ID
**Branch**: session/${SESSION_ID}
**Commits**: $(git rev-list --count main..HEAD)

## Work Summary
[Auto-generated from git commits]

## To Resume
Checkout: git checkout session/${SESSION_ID}
EOF
```

### 5. Memory Preservation

Create session summary with git context:

```markdown
# Session: exploring authentication
Branch: session/20250817-140000-auth
Duration: 2h 45m
Outcome: SUCCESS - Feature implemented

## Git Summary
- Commits: 12 (3 WIP, 9 semantic)
- Final state: All tests passing
- Code survival: 85% (15% refactored during session)

## What Worked
- JWT approach after trying sessions
- Bcrypt for password hashing
- Middleware pattern for auth checks

## What Didn't
- First tried cookie sessions (too complex)
- Attempted custom crypto (bad idea)

## Patterns Established
- auth-middleware pattern
- password-validation rules
- token-refresh strategy

## Next Session Can
- Build on feature/user-authentication branch
- Reference patterns in layer/sessions/20250817-140000.md
- Avoid cookie session approach (didn't work)
```

### 6. Simple Cleanup

```bash
# Session branches are always kept (memory preservation)
# No complex decision tree needed
# If you want to clean up later:
echo "All session branches preserved in .git/"
echo "To list: git branch | grep session/"
echo "To archive old ones: git tag archive/[session-name] [branch]"
```

## LLM Guidance

### For Successful Feature Sessions
```markdown
✅ Session Success! Detected: Feature Implementation

Next steps:
1. Creating feature/user-auth from your session
2. Squashing 12 commits into 3 semantic ones:
   - feat: add user model and validation
   - feat: implement auth endpoints
   - test: add auth integration tests
3. Session branch preserved at: session/20250817-140000-auth

Ready to PR? Run: git push origin feature/user-auth
```

### For Learning/Exploration Sessions
```markdown
📚 Exploration Session Complete!

Valuable learnings detected:
- WebSocket approach complexity discovered
- Performance bottleneck identified
- Alternative pattern found

Archiving to: layer/dust/explorations/websocket-investigation.md
Session branch tagged: archive/websocket-exploration

This knowledge is now searchable for future sessions.
```

### For Failed Experiments
```markdown
🔬 Experiment Didn't Work - But That's Valuable!

What we learned:
- Custom crypto is harder than expected
- Library X has unexpected limitations
- Approach Y doesn't scale

Preserving failure as memory:
- Branch: archived/failed-custom-crypto
- Notes: layer/dust/failed-experiments/crypto-attempt.md

Future sessions will warn when similar approach detected.
```

## Configuration Options

```toml
# .patina/config.toml
[session.end]
auto_classify = true                  # Automatically detect session type
require_clean_tree = true            # Block if uncommitted changes
create_feature_branches = true       # Auto-create feature/* from successful sessions
preserve_session_branches = true     # Keep session/* branches as memory
squash_on_feature = true            # Squash when creating feature branches
archive_failed = true                # Archive failed experiments
cleanup_simple_sessions = false      # Delete trivial session branches
```

## Success Metrics

- Clean git state at session end
- Correct classification (feature/fix/exploration)
- Meaningful commit messages in feature branches
- Failed experiments preserved as memory
- No lost work

## Example Full Flow

```bash
$ /session-end

Analyzing session/20250817-140000-auth...

📊 Session Summary:
- Duration: 2h 45m
- Commits: 12
- Changes: +450 -23 lines across 8 files
- Tests: ✅ All passing
- Patterns: 3 new, 1 modified

🎯 Classification: FEATURE (confidence: 92%)

This session successfully implemented user authentication.

Recommended actions:
1. ✅ Create feature branch (will squash 12 → 3 commits)
2. ✅ Preserve session branch for memory
3. ✅ Extract patterns to layer/surface/
4. ⚠️ Review and create PR

Proceed with recommendations? [Y/n]: Y

✨ Creating feature/user-authentication...
✨ Squashing commits with semantic messages...
✨ Patterns extracted to layer/surface/auth-patterns.md
✨ Session archived to layer/sessions/20250817-140000.md

Success! Your feature branch is ready:
  git checkout feature/user-authentication
  git push origin feature/user-authentication

Session branch preserved at: session/20250817-140000-auth
```