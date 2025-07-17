#!/bin/bash
# End current Patina session with distillation and pattern extraction

CURRENT_SESSION=$(ls -t .claude/context/sessions/*.md 2>/dev/null | head -1)
LAST_SESSION_FILE=".claude/context/last-session.md"

if [ -z "$CURRENT_SESSION" ]; then
    echo "No active session found."
    exit 1
fi

# Get session metadata
SESSION_NAME=$(basename "$CURRENT_SESSION" .md)
SESSION_START=$(grep "Started:" "$CURRENT_SESSION" | head -1 | cut -d: -f2- | xargs)
SESSION_START_COMMIT=$(grep "Starting Commit:" "$CURRENT_SESSION" | head -1 | awk '{print $3}')
CURRENT_COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "no commits")

# Begin session summary
echo "" >> "$CURRENT_SESSION"
echo "---" >> "$CURRENT_SESSION"
echo "" >> "$CURRENT_SESSION"
echo "## Session Summary" >> "$CURRENT_SESSION"
echo "**Duration**: $SESSION_START → $(date)" >> "$CURRENT_SESSION"

# Analyze git changes during session
if [ -n "$SESSION_START_COMMIT" ] && [ "$SESSION_START_COMMIT" != "$CURRENT_COMMIT" ]; then
    COMMITS_COUNT=$(git rev-list --count "$SESSION_START_COMMIT..$CURRENT_COMMIT" 2>/dev/null || echo "0")
    FILES_CHANGED=$(git diff --name-only "$SESSION_START_COMMIT..$CURRENT_COMMIT" 2>/dev/null | wc -l | tr -d ' ')
    
    echo "**Git Activity**: $COMMITS_COUNT commits, $FILES_CHANGED files changed" >> "$CURRENT_SESSION"
    echo "" >> "$CURRENT_SESSION"
    
    # Show key changes
    echo "### Key Changes" >> "$CURRENT_SESSION"
    echo '```' >> "$CURRENT_SESSION"
    git diff --stat "$SESSION_START_COMMIT..$CURRENT_COMMIT" 2>/dev/null | head -20 >> "$CURRENT_SESSION"
    echo '```' >> "$CURRENT_SESSION"
    echo "" >> "$CURRENT_SESSION"
    
    # Extract commit messages as decisions/learnings
    if [ "$COMMITS_COUNT" -gt 0 ]; then
        echo "### Commits Made" >> "$CURRENT_SESSION"
        echo '```' >> "$CURRENT_SESSION"
        git log --oneline "$SESSION_START_COMMIT..$CURRENT_COMMIT" 2>/dev/null >> "$CURRENT_SESSION"
        echo '```' >> "$CURRENT_SESSION"
        echo "" >> "$CURRENT_SESSION"
    fi
fi

# Archive the raw session
mkdir -p .claude/context/sessions/archive
cp "$CURRENT_SESSION" ".claude/context/sessions/archive/"

# Create distilled version
DISTILLED_SESSION=".claude/context/sessions/$(date +%Y-%m-%d)-patina-${SESSION_NAME}.md"

cat > "$DISTILLED_SESSION" << EOF
# Session: $SESSION_NAME (Distilled)
**Duration**: $SESSION_START → $(date)
**Branch**: $(git branch --show-current 2>/dev/null || echo "unknown")

## Marks of Interest
[Claude: List all the interest marks from the session]

## Patterns Noticed  
[Claude: What patterns emerge from these marks?]

## Worth Remembering
[Claude: 2-3 key insights from this session]
EOF

# Create last-session.md pointing to distilled
cat > "$LAST_SESSION_FILE" << EOF
# Last Session: $SESSION_NAME

See: $DISTILLED_SESSION

Quick start: /session-start "continue-from-$SESSION_NAME"
EOF

# Move original to archive  
mv "$CURRENT_SESSION" ".claude/context/sessions/archive/"

echo "Session ended: $CURRENT_SESSION"
echo "Quick restart context saved to: $LAST_SESSION_FILE"
echo ""
echo "Next steps:"
echo "1. Edit the session file to fill in learnings and patterns"
echo "2. If valuable patterns found: patina add topic \"pattern-name\""
echo "3. Resume next time with: /session-start"