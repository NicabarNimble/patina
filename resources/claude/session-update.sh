#!/bin/bash
# Update current Patina session with rich context capture

CURRENT_SESSION=$(ls -t .claude/context/sessions/*.md 2>/dev/null | head -1)
LAST_UPDATE_FILE=".claude/context/.last-update"
MARK="$*"  # Capture the arguments as the interest mark

if [ -z "$CURRENT_SESSION" ]; then
    echo "No active session found. Start one with: /session-start"
    exit 1
fi

# Get last update time
LAST_UPDATE=$(cat "$LAST_UPDATE_FILE" 2>/dev/null || echo "session start")
CURRENT_TIME=$(date +"%H:%M")

# Add timestamp header with mark (if provided) and time span
echo "" >> "$CURRENT_SESSION"
if [ -n "$MARK" ]; then
    echo "### $CURRENT_TIME - $MARK (covering since $LAST_UPDATE)" >> "$CURRENT_SESSION"
else
    echo "### $CURRENT_TIME - Update (covering since $LAST_UPDATE)" >> "$CURRENT_SESSION"
fi
echo "" >> "$CURRENT_SESSION"
echo "<!-- Claude: Fill in what happened during this time span -->" >> "$CURRENT_SESSION"
echo "" >> "$CURRENT_SESSION"

# Update last update time
echo "$CURRENT_TIME" > "$LAST_UPDATE_FILE"

echo "Update marker added for time span: $LAST_UPDATE → $CURRENT_TIME"
if [ -n "$MARK" ]; then
    echo "Mark captured: $MARK"
fi
echo "Claude will fill in the context for this period"