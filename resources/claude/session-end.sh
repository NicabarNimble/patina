#!/bin/bash
# End current Patina session with distillation and pattern extraction

CURRENT_SESSION=$(ls -t .claude/context/sessions/*.md 2>/dev/null | head -1)
LAST_SESSION_FILE=".claude/context/last-session.md"
LAST_UPDATE_FILE=".claude/context/.last-update"

if [ -z "$CURRENT_SESSION" ]; then
    echo "No active session found."
    exit 1
fi

# Run final update first
echo "Running final session update..."
.claude/bin/session-update.sh "final context before ending"

# Get session metadata
SESSION_NAME=$(basename "$CURRENT_SESSION" .md)
SESSION_START=$(grep "Started:" "$CURRENT_SESSION" | head -1 | cut -d: -f2- | xargs)
SESSION_START_COMMIT=$(grep "Starting Commit:" "$CURRENT_SESSION" | head -1 | awk '{print $3}')
CURRENT_COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "no commits")

# Calculate git statistics
if [ -n "$SESSION_START_COMMIT" ] && [ "$SESSION_START_COMMIT" != "$CURRENT_COMMIT" ] && [ "$SESSION_START_COMMIT" != "no" ]; then
    COMMITS_COUNT=$(git rev-list --count "$SESSION_START_COMMIT..$CURRENT_COMMIT" 2>/dev/null || echo "0")
    FILES_CHANGED=$(git diff --name-only "$SESSION_START_COMMIT..$CURRENT_COMMIT" 2>/dev/null | wc -l | tr -d ' ')
else
    COMMITS_COUNT="0"
    FILES_CHANGED="0"
fi

# Add structured end section with rails
echo "" >> "$CURRENT_SESSION"
echo "---" >> "$CURRENT_SESSION"
echo "## Session End: $(date)" >> "$CURRENT_SESSION"
echo "**Duration**: $SESSION_START → $(date)" >> "$CURRENT_SESSION"
echo "**Commits**: $COMMITS_COUNT (from $SESSION_START_COMMIT to $CURRENT_COMMIT)" >> "$CURRENT_SESSION"
echo "" >> "$CURRENT_SESSION"

# Add git activity if relevant
if [ "$COMMITS_COUNT" -gt "0" ]; then
    echo "### Git Activity" >> "$CURRENT_SESSION"
    echo '```' >> "$CURRENT_SESSION"
    git log --oneline "$SESSION_START_COMMIT..$CURRENT_COMMIT" 2>/dev/null | head -10 >> "$CURRENT_SESSION"
    echo '```' >> "$CURRENT_SESSION"
    echo "" >> "$CURRENT_SESSION"
fi

# Add required sections for Claude to fill
cat >> "$CURRENT_SESSION" << 'EOF'
### Required Sections (Claude: Please fill ALL sections)

#### What We Did
<!-- List key activities from the activity log above -->

#### Key Insights
<!-- Extract important discoveries, especially from Notes -->

#### Patterns Identified
<!-- Any reusable patterns worth adding to brain? -->

#### Next Session Should
<!-- Based on progress and open questions -->

### Verification Checklist
- [ ] All Notes addressed in insights
- [ ] Activity log summarized
- [ ] Patterns extracted if any
- [ ] Clear next steps provided
EOF

# Create verification status file
echo "PENDING_DISTILLATION" > "$CURRENT_SESSION.status"

# Archive the raw session
mkdir -p .claude/context/sessions/archive
cp "$CURRENT_SESSION" ".claude/context/sessions/archive/"

# Create summary pointer
SUMMARY_NAME="$(date +%Y-%m-%d)-${SESSION_NAME}-summary"
cat > "$LAST_SESSION_FILE" << EOF
# Last Session: $SESSION_NAME

See: .claude/context/sessions/${SUMMARY_NAME}.md

Quick start: /session-start "continue-from-$SESSION_NAME"
EOF

# Clean up temporary files
rm -f "$LAST_UPDATE_FILE"

echo "Session end structure added to: $CURRENT_SESSION"
echo ""
echo "Next steps:"
echo "1. Fill in ALL required sections in the session file"
echo "2. Verify the checklist is complete"
echo "3. Say 'Session ended and distilled' when done"
echo "4. If patterns found: patina add <type> \"pattern-name\""