#!/bin/bash
# Update current Patina session with a mark of interest

CURRENT_SESSION=$(ls -t .claude/context/sessions/*.md 2>/dev/null | head -1)

if [ -z "$CURRENT_SESSION" ]; then
    echo "No active session found. Start one with: /session-start"
    exit 1
fi

# Just timestamp and capture what's interesting
echo "" >> "$CURRENT_SESSION"
echo "### $(date +"%H:%M") - Interest" >> "$CURRENT_SESSION"
echo "${*}" >> "$CURRENT_SESSION"

echo "Marked: ${*}"