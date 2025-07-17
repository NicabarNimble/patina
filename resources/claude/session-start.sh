#!/bin/bash
# Start a new Patina development session with git awareness

SESSION_NAME="${1:-$(date +%Y%m%d-%H%M%S)}"
SESSION_FILE=".claude/context/sessions/${SESSION_NAME}.md"
LAST_SESSION_FILE=".claude/context/last-session.md"
LAST_UPDATE_FILE=".claude/context/.last-update"

mkdir -p .claude/context/sessions

# Check for previous sessions and git state
PREVIOUS_SESSION=$(ls -t .claude/context/sessions/*.md 2>/dev/null | grep -v "$SESSION_FILE" | head -1)
CURRENT_BRANCH=$(git branch --show-current 2>/dev/null || echo "not in git repo")
CURRENT_COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "no commits")
UNCOMMITTED=$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')

# Create session file with minimal rails
cat > "$SESSION_FILE" << EOF
# Session: ${SESSION_NAME}
**Started**: $(date)
**Branch**: ${CURRENT_BRANCH}
**Starting Commit**: ${CURRENT_COMMIT}
**Uncommitted Changes**: ${UNCOMMITTED} files

## Goals
- [ ] ${2:-[Session goals]}

## Context
EOF

# Add previous session summary if exists
if [ -f "$LAST_SESSION_FILE" ]; then
    echo "### Previous Session Summary" >> "$SESSION_FILE"
    cat "$LAST_SESSION_FILE" >> "$SESSION_FILE"
    echo "" >> "$SESSION_FILE"
elif [ -n "$PREVIOUS_SESSION" ]; then
    echo "### Previous Session" >> "$SESSION_FILE"
    echo "Found at: $PREVIOUS_SESSION" >> "$SESSION_FILE"
    echo "" >> "$SESSION_FILE"
fi

# Add current git status summary if uncommitted changes
if [ "$UNCOMMITTED" -gt 0 ]; then
    echo "### Current Working State" >> "$SESSION_FILE"
    echo '```' >> "$SESSION_FILE"
    git status --short 2>/dev/null >> "$SESSION_FILE"
    echo '```' >> "$SESSION_FILE"
    echo "" >> "$SESSION_FILE"
fi

echo "## Activity Log" >> "$SESSION_FILE"
echo "<!-- Claude fills this naturally during work -->" >> "$SESSION_FILE"
echo "" >> "$SESSION_FILE"

# Initialize last update time
date +"%H:%M" > "$LAST_UPDATE_FILE"

echo "Session started: $SESSION_FILE"
echo "Branch: $CURRENT_BRANCH | Uncommitted: $UNCOMMITTED files"