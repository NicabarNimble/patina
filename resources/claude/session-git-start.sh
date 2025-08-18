#!/bin/bash
# Start a new Patina development session with Git integration
# Testing version - will replace session-start once validated

# Check for active session first
ACTIVE_SESSION=".claude/context/active-session.md"
if [ -f "$ACTIVE_SESSION" ]; then
    echo "Found incomplete session, cleaning up..."
    
    # Check if active session has meaningful content
    # (more than just headers - roughly 10 lines)
    if [ $(wc -l < "$ACTIVE_SESSION") -gt 10 ]; then
        # Run session-end silently to archive it
        $(dirname "$0")/session-git-end.sh --silent
    else
        # Just delete if it's empty/trivial
        rm "$ACTIVE_SESSION"
        echo "Removed empty session file"
    fi
fi

# Create session ID and title
SESSION_ID="$(date +%Y%m%d-%H%M%S)"
SESSION_TITLE="${1:-untitled}"
SAFE_TITLE=$(echo "$SESSION_TITLE" | tr ' ' '-' | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9-]//g')

# Git integration: Create session branch
BRANCH_NAME="session/${SESSION_ID}-${SAFE_TITLE}"
PARENT_BRANCH=$(git branch --show-current 2>/dev/null || echo "main")
STARTING_COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "none")

# Check for uncommitted changes
if [[ -n $(git status --porcelain 2>/dev/null) ]]; then
    echo "⚠️  Warning: Uncommitted changes exist"
    echo "   Consider: git stash or git commit -am 'WIP: saving work'"
    echo ""
fi

# Create branch (don't fail if git isn't available)
if command -v git &> /dev/null && [ -d .git ]; then
    git checkout -b "$BRANCH_NAME" 2>/dev/null && \
    echo "✅ Memory branch created: $BRANCH_NAME" || \
    echo "⚠️  Could not create branch (may already exist or not in git repo)"
else
    echo "📝 Not a git repository - session tracking only"
fi

# Create active session file
mkdir -p .claude/context/sessions

# Get LLM info (claude for now, extensible later)
LLM_NAME="claude"

# Create active session with metadata including git info
cat > "$ACTIVE_SESSION" << EOF
# Session: ${SESSION_TITLE}
**ID**: ${SESSION_ID}
**Started**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**LLM**: ${LLM_NAME}
**Git Branch**: ${BRANCH_NAME}
**Parent Branch**: ${PARENT_BRANCH}
**Starting Commit**: ${STARTING_COMMIT}

## Previous Session Context
<!-- AI: Summarize the last session from last-session.md -->

## Goals
- [ ] ${SESSION_TITLE}

## Activity Log
### $(date +"%H:%M") - Session Start
Session initialized with goal: ${SESSION_TITLE}
Git branch created: ${BRANCH_NAME}

EOF

# Create/update last update marker
echo "$(date +"%H:%M")" > .claude/context/.last-update

echo "✓ Session started: ${SESSION_TITLE}"
echo "  ID: ${SESSION_ID}"

# Git coaching for LLM
if [ -d .git ]; then
    echo ""
    echo "📍 Session Strategy:"
    echo "- You're now on branch: ${BRANCH_NAME}"
    echo "- This is YOUR exploration space - experiment freely"
    echo "- Commit early and often - git remembers everything"
    echo "- Failed attempts are valuable - they become memory"
    echo "- Think of commits as checkpoints in a game"
    echo "- The messier the exploration, the more we learn"
    echo ""
    echo "Remember to:"
    echo "- Make small, focused commits (scalpel not shotgun)"
    echo "- Commit working code before major changes"
    echo "- Use descriptive commit messages"
    echo "- This is an exploration branch - failed experiments are valuable"
fi

# Prompt AI to read last-session.md and provide context
echo ""
if [ -f ".claude/context/last-session.md" ]; then
    echo "Please read .claude/context/last-session.md and fill in the Previous Session Context section above."
else
    echo "No previous session found. Starting fresh."
fi
echo "Then ask: 'Would you like me to create todos for \"${SESSION_TITLE}\"?'"