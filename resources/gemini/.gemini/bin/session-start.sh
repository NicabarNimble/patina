#!/bin/bash
# Start a new Patina development session (Gemini adapter)

# Check for active session first
ACTIVE_SESSION=".gemini/context/active-session.md"
if [ -f "$ACTIVE_SESSION" ]; then
    echo "Found incomplete session, cleaning up..."

    # Check if active session has meaningful content
    # (more than just headers - roughly 10 lines)
    if [ $(wc -l < "$ACTIVE_SESSION") -gt 10 ]; then
        # Run session-end silently to archive it
        $(dirname "$0")/session-end.sh --silent
    else
        # Just delete if it's empty/trivial
        rm "$ACTIVE_SESSION"
        echo "Removed empty session file"
    fi
fi

# Create session ID and title
SESSION_ID="$(date +%Y%m%d-%H%M%S)"
SESSION_TITLE="${1:-untitled}"

# Create active session file
mkdir -p .gemini/context/sessions

# Get LLM info
LLM_NAME="gemini"

# Create active session with metadata
cat > "$ACTIVE_SESSION" << EOF
# Session: ${SESSION_TITLE}
**ID**: ${SESSION_ID}
**Started**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**LLM**: ${LLM_NAME}

## Previous Session Context
<!-- AI: Summarize the last session from last-session.md -->

## Goals
- [ ] ${SESSION_TITLE}

## Activity Log
### $(date +"%H:%M") - Session Start
Session initialized with goal: ${SESSION_TITLE}

EOF

# Create/update last update marker
echo "$(date +"%H:%M")" > .gemini/context/.last-update

echo "✓ Session started: ${SESSION_TITLE}"
echo "  ID: ${SESSION_ID}"

# Prompt AI to read last-session.md and provide context
echo ""
if [ -f ".gemini/context/last-session.md" ]; then
    echo "Please read .gemini/context/last-session.md and fill in the Previous Session Context section above."
else
    echo "No previous session found. Starting fresh."
fi
echo "Then ask: 'Would you like me to create todos for \"${SESSION_TITLE}\"?'"
