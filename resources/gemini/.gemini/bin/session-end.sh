#!/bin/bash
# Archive current Patina session to permanent storage (Gemini adapter)

ACTIVE_SESSION=".gemini/context/active-session.md"
LAST_SESSION_FILE=".gemini/context/last-session.md"
LAST_UPDATE_FILE=".gemini/context/.last-update"
SILENT_MODE=false

# Check for silent mode
if [ "$1" = "--silent" ]; then
    SILENT_MODE=true
fi

if [ ! -f "$ACTIVE_SESSION" ]; then
    [ "$SILENT_MODE" = false ] && echo "No active session found."
    exit 1
fi

# Extract session metadata (escape ** for grep)
SESSION_ID=$(grep "\*\*ID\*\*:" "$ACTIVE_SESSION" | cut -d' ' -f2)
SESSION_TITLE=$(grep "# Session:" "$ACTIVE_SESSION" | cut -d: -f2- | xargs)
SESSION_START=$(grep "\*\*Started\*\*:" "$ACTIVE_SESSION" | cut -d' ' -f2-)

# Debug: Check if SESSION_ID is empty
if [ -z "$SESSION_ID" ]; then
    echo "Warning: SESSION_ID is empty, using timestamp"
    SESSION_ID=$(date +"%Y%m%d-%H%M%S")
fi

# Archive the session
mkdir -p .gemini/context/sessions
mkdir -p layer/sessions

# Copy to both locations with ID as filename
cp "$ACTIVE_SESSION" ".gemini/context/sessions/${SESSION_ID}.md"
cp "$ACTIVE_SESSION" "layer/sessions/${SESSION_ID}.md"

# Update last-session.md pointer
cat > "$LAST_SESSION_FILE" << EOF
# Last Session: ${SESSION_TITLE}

See: layer/sessions/${SESSION_ID}.md

Quick start: /session-start "continue from ${SESSION_TITLE}"
EOF

# Clean up
rm -f "$ACTIVE_SESSION"
rm -f "$LAST_UPDATE_FILE"

if [ "$SILENT_MODE" = false ]; then
    echo ""
    echo "✓ Session archived:"
    echo "  - .gemini/context/sessions/${SESSION_ID}.md"
    echo "  - layer/sessions/${SESSION_ID}.md"
    echo "  - Updated last-session.md"
fi
