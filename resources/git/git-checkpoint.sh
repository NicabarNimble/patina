#!/bin/bash
# Universal git checkpoint script
# Creates semantic commits with timing context
# Outputs JSON for LLM-agnostic consumption

set -euo pipefail

# Helper function to output JSON
output_json() {
    local status="$1"
    local data="$2"
    local message="$3"
    
    cat <<EOF
{
  "status": "$status",
  "data": $data,
  "message": "$message",
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
}
EOF
}

# Get current branch
CURRENT_BRANCH=$(git branch --show-current 2>/dev/null || echo "")
if [ -z "$CURRENT_BRANCH" ]; then
    output_json "error" "null" "Not in a git repository"
    exit 1
fi

# Check if on main/master
if [ "$CURRENT_BRANCH" = "main" ] || [ "$CURRENT_BRANCH" = "master" ]; then
    output_json "error" "{\"branch\": \"$CURRENT_BRANCH\"}" "Cannot checkpoint on main branch. Use /git-start first."
    exit 1
fi

# Get timing context from session if available
LAST_CHECKPOINT=""
CURRENT_TIME=$(date +"%H:%M")
SESSION_ID=""

# Try to get timing from Claude session
if [ -f ".claude/context/.last-update" ]; then
    LAST_CHECKPOINT=$(cat ".claude/context/.last-update" 2>/dev/null || echo "")
fi

# Try to get session ID
if [ -f ".claude/context/active-session.md" ]; then
    SESSION_ID=$(grep "\*\*ID\*\*:" ".claude/context/active-session.md" 2>/dev/null | cut -d' ' -f2 || echo "")
fi

# Get change statistics
CHANGED_FILES=$(git diff --name-only | wc -l | xargs)
STAGED_FILES=$(git diff --cached --name-only | wc -l | xargs)
UNTRACKED_FILES=$(git ls-files --others --exclude-standard | wc -l | xargs)
TOTAL_CHANGES=$((CHANGED_FILES + STAGED_FILES + UNTRACKED_FILES))

# Get list of changed files (for commit message context)
CHANGED_LIST=$(git status --porcelain | head -10)

# Build data object
DATA=$(cat <<EOF
{
  "branch": "$CURRENT_BRANCH",
  "changed_files": $CHANGED_FILES,
  "staged_files": $STAGED_FILES,
  "untracked_files": $UNTRACKED_FILES,
  "total_changes": $TOTAL_CHANGES,
  "time_context": {
    "last_checkpoint": "${LAST_CHECKPOINT:-unknown}",
    "current_time": "$CURRENT_TIME"
  },
  "session_id": "$SESSION_ID",
  "changes_preview": $(echo "$CHANGED_LIST" | jq -Rs .)
}
EOF
)

# Check if there are changes to commit
if [ "$TOTAL_CHANGES" -eq 0 ]; then
    output_json "info" "$DATA" "No changes to checkpoint"
    exit 0
fi

# Stage all changes (can be made configurable)
git add -A 2>/dev/null

# Get commit message suggestion based on changes
COMMIT_SUGGESTION=""
if [ "$CHANGED_FILES" -eq 1 ]; then
    # Single file changed - be specific
    FILE=$(git diff --cached --name-only | head -1)
    DIR=$(dirname "$FILE")
    BASE=$(basename "$FILE")
    
    # Guess commit type based on file
    if [[ "$FILE" == *"test"* ]]; then
        COMMIT_SUGGESTION="test: update $BASE"
    elif [[ "$FILE" == *".md" ]]; then
        COMMIT_SUGGESTION="docs: update $BASE"
    elif [[ "$DIR" == "src"* ]]; then
        COMMIT_SUGGESTION="feat: update $BASE"
    else
        COMMIT_SUGGESTION="chore: update $FILE"
    fi
else
    # Multiple files - be general
    if [ "$UNTRACKED_FILES" -gt "$CHANGED_FILES" ]; then
        COMMIT_SUGGESTION="feat: add new functionality"
    else
        COMMIT_SUGGESTION="feat: update implementation"
    fi
fi

# Add timing context to suggestion if available
if [ -n "$LAST_CHECKPOINT" ] && [ "$LAST_CHECKPOINT" != "$CURRENT_TIME" ]; then
    COMMIT_SUGGESTION="$COMMIT_SUGGESTION ($LAST_CHECKPOINT-$CURRENT_TIME)"
fi

# Update checkpoint data with suggestion
CHECKPOINT_DATA=$(cat <<EOF
{
  "branch": "$CURRENT_BRANCH",
  "staged_count": $(git diff --cached --name-only | wc -l | xargs),
  "commit_suggestion": "$COMMIT_SUGGESTION",
  "time_context": {
    "last_checkpoint": "${LAST_CHECKPOINT:-unknown}",
    "current_time": "$CURRENT_TIME"
  },
  "session_id": "$SESSION_ID",
  "ready_to_commit": true
}
EOF
)

# Update last checkpoint time if we have a session
if [ -f ".claude/context/.last-update" ]; then
    echo "$CURRENT_TIME" > ".claude/context/.last-update"
fi

output_json "success" "$CHECKPOINT_DATA" "Changes staged. Ready for commit with suggested message: $COMMIT_SUGGESTION"