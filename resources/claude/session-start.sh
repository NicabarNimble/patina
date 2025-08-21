#!/bin/bash
# Start a new Patina development session with Git integration
# Uses work branch + tags instead of creating session branches

# Check for active session first
ACTIVE_SESSION=".claude/context/active-session.md"
if [ -f "$ACTIVE_SESSION" ]; then
    echo "Found incomplete session, cleaning up..."
    
    # Check if active session has meaningful content
    # (more than just headers - roughly 10 lines)
    if [ $(wc -l < "$ACTIVE_SESSION") -gt 10 ]; then
        # Run session-end silently to archive it
        $(dirname "$0")/session-end.sh --silent
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

# Git integration: Use work branch + tags (not session branches!)
CURRENT_BRANCH=$(git branch --show-current 2>/dev/null || echo "none")
STARTING_COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "none")
SESSION_TAG="session-${SESSION_ID}-start"

# Check for uncommitted changes
if [[ -n $(git status --porcelain 2>/dev/null) ]]; then
    echo "⚠️  Warning: Uncommitted changes exist"
    echo "   Consider: git stash or git commit -am 'WIP: saving work'"
    echo ""
fi

# Smart branch handling: respect work and its sub-branches
if command -v git &> /dev/null && [ -d .git ]; then
    # Check if current branch is work or descended from work
    IS_WORK_RELATED=false
    if [[ "$CURRENT_BRANCH" == "work" ]]; then
        IS_WORK_RELATED=true
    elif git merge-base --is-ancestor work HEAD 2>/dev/null; then
        IS_WORK_RELATED=true
        echo "📌 Staying on work sub-branch: $CURRENT_BRANCH"
    fi
    
    # Only switch to work if we're on main/master or unrelated branch
    if [[ "$IS_WORK_RELATED" == "false" ]]; then
        if [[ "$CURRENT_BRANCH" == "main" ]] || [[ "$CURRENT_BRANCH" == "master" ]]; then
            # Create work branch if it doesn't exist, or switch to it
            git checkout -b work 2>/dev/null || git checkout work
            echo "✅ Switched to work branch from $CURRENT_BRANCH"
        else
            echo "⚠️  On unrelated branch: $CURRENT_BRANCH"
            echo "   Consider: git checkout work or git checkout -b work/$CURRENT_BRANCH"
        fi
    fi
    
    # Tag the session start point
    git tag -a "$SESSION_TAG" -m "Session start: ${SESSION_TITLE}" 2>/dev/null && \
        echo "✅ Session tagged: $SESSION_TAG" || \
        echo "⚠️  Could not create tag (may already exist)"
    
    CURRENT_BRANCH=$(git branch --show-current)
else
    echo "📝 Not a git repository - session tracking only"
fi

# Track in SQLite if database exists
DB_PATH=".patina/navigation.db"
if [ -f "$DB_PATH" ] && command -v sqlite3 &> /dev/null; then
    sqlite3 "$DB_PATH" "
        INSERT INTO state_transitions (
            workspace_id,
            to_state,
            transition_reason,
            metadata
        ) VALUES (
            '${SESSION_TAG}',
            'SessionStart',
            'Session: ${SESSION_TITLE}',
            json_object(
                'session_id', '${SESSION_ID}',
                'title', '${SESSION_TITLE}',
                'branch', '${CURRENT_BRANCH}',
                'parent_commit', '${STARTING_COMMIT}'
            )
        );
    " 2>/dev/null && echo "✅ Session tracked in database" || echo "⚠️  Could not update database"
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
**Git Branch**: ${CURRENT_BRANCH}
**Session Tag**: ${SESSION_TAG}
**Starting Commit**: ${STARTING_COMMIT}

## Previous Session Context
<!-- AI: Summarize the last session from last-session.md -->

## Goals
- [ ] ${SESSION_TITLE}

## Activity Log
### $(date +"%H:%M") - Session Start
Session initialized with goal: ${SESSION_TITLE}
Working on branch: ${CURRENT_BRANCH}
Tagged as: ${SESSION_TAG}

EOF

# Create/update last update marker
echo "$(date +"%H:%M")" > .claude/context/.last-update

echo "✓ Session started: ${SESSION_TITLE}"
echo "  ID: ${SESSION_ID}"
echo "  Branch: ${CURRENT_BRANCH}"
echo "  Tag: ${SESSION_TAG}"

# Git coaching for LLM (updated for work branch strategy)
if [ -d .git ]; then
    echo ""
    echo "📍 Session Strategy:"
    if [[ "$CURRENT_BRANCH" == "work" ]]; then
        echo "- You're on the 'work' branch - all sessions happen here"
    else
        echo "- You're on '$CURRENT_BRANCH' (work sub-branch) - perfect for isolated experiments"
    fi
    echo "- Session tagged as: ${SESSION_TAG}"
    echo "- Commit early and often - each commit is a checkpoint"
    echo "- Failed attempts are valuable memory"
    echo "- No need to create branches for sessions"
    echo ""
    echo "Remember to:"
    echo "- Make small, focused commits"
    echo "- Use descriptive commit messages"
    echo "- Tags mark session boundaries, commits mark progress"
fi

# Prompt AI to read last-session.md and provide context
echo ""
if [ -f ".claude/context/last-session.md" ]; then
    echo "Please read .claude/context/last-session.md and fill in the Previous Session Context section above."
else
    echo "No previous session found. Starting fresh."
fi
echo "Then ask: 'Would you like me to create todos for \"${SESSION_TITLE}\"?'"