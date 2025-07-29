#!/bin/bash
# Add a human note to the current Patina session

ACTIVE_SESSION=".claude/context/active-session.md"
NOTE="$*"

if [ ! -f "$ACTIVE_SESSION" ]; then
    echo "No active session found. Start one with: /session-start"
    exit 1
fi

if [ -z "$NOTE" ]; then
    echo "Usage: /session-note <your note text>"
    exit 1
fi

# Add note with timestamp
echo "" >> "$ACTIVE_SESSION"
echo "### $(date +"%H:%M") - Note" >> "$ACTIVE_SESSION"
echo "$NOTE" >> "$ACTIVE_SESSION"

echo "✓ Note added to session"