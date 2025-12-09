#!/bin/bash
# Update current Patina session with rich context capture (Gemini adapter)

ACTIVE_SESSION=".gemini/context/active-session.md"
LAST_UPDATE_FILE=".gemini/context/.last-update"

if [ ! -f "$ACTIVE_SESSION" ]; then
    echo "No active session found. Start one with: /session-start"
    exit 1
fi

# Get last update time
LAST_UPDATE=$(cat "$LAST_UPDATE_FILE" 2>/dev/null || echo "session start")
CURRENT_TIME=$(date +"%H:%M")

# Add timestamp header with time span
echo "" >> "$ACTIVE_SESSION"
echo "### $CURRENT_TIME - Update (covering since $LAST_UPDATE)" >> "$ACTIVE_SESSION"

# Update last update time NOW so agent sees correct window
echo "$CURRENT_TIME" > "$LAST_UPDATE_FILE"

# Prompt for direct update
echo ""
echo "Please fill in the update section in active-session.md with:"
echo "- Work completed since $LAST_UPDATE"
echo "- Key decisions and reasoning"
echo "- Challenges faced and solutions"
echo "- Patterns observed"
echo ""
echo "✓ Update marker added: $LAST_UPDATE → $CURRENT_TIME"
