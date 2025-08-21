#!/bin/bash
# Add a note to the current Patina session with Git context
# Testing version - will replace session-note once validated

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

# Get git context if available
GIT_CONTEXT=""
if command -v git &> /dev/null && [ -d .git ]; then
    CURRENT_BRANCH=$(git branch --show-current 2>/dev/null)
    CURRENT_SHA=$(git rev-parse --short HEAD 2>/dev/null || echo "no-commits")
    GIT_CONTEXT=" [${CURRENT_BRANCH}@${CURRENT_SHA}]"
    
    # Check if this is an important insight that should be committed
    if [[ "$NOTE" == *"breakthrough"* ]] || 
       [[ "$NOTE" == *"discovered"* ]] || 
       [[ "$NOTE" == *"solved"* ]] || 
       [[ "$NOTE" == *"fixed"* ]] || 
       [[ "$NOTE" == *"important"* ]]; then
        echo ""
        echo "💡 Important insight detected!"
        echo "   Consider committing current work to preserve this context:"
        echo "   git commit -am \"checkpoint: $NOTE\""
    fi
fi

# Add note with timestamp and git context
echo "" >> "$ACTIVE_SESSION"
echo "### $(date +"%H:%M") - Note${GIT_CONTEXT}" >> "$ACTIVE_SESSION"
echo "$NOTE" >> "$ACTIVE_SESSION"

echo "✓ Note added to session${GIT_CONTEXT}"

# Occasional reminder about notes as memory
if [ $((RANDOM % 4)) -eq 0 ]; then
    echo ""
    echo "💭 Remember: Notes with Git context become searchable memory"
    echo "   Future sessions can find: 'when did we solve X?'"
fi