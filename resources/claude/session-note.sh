#!/bin/bash
# Add a human note to the current Patina session

CURRENT_SESSION=$(ls -t .claude/context/sessions/*.md 2>/dev/null | head -1)

if [ -z "$CURRENT_SESSION" ]; then
    echo "No active session found. Start one with: /session-start"
    exit 1
fi

# Add note with timestamp
echo "" >> "$CURRENT_SESSION"
echo "### $(date +"%H:%M") - Note" >> "$CURRENT_SESSION"
echo "$*" >> "$CURRENT_SESSION"
echo "" >> "$CURRENT_SESSION"

echo "Note added: $*"