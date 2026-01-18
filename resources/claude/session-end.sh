#!/bin/bash
# Archive current Patina session with Git work classification
# Works with work branch + tags strategy

ACTIVE_SESSION=".claude/context/active-session.md"
LAST_SESSION_FILE=".claude/context/last-session.md"
LAST_UPDATE_FILE=".claude/context/.last-update"
SILENT_MODE=false

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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
SESSION_TAG=$(grep "\*\*Session Tag\*\*:" "$ACTIVE_SESSION" | sed 's/.*\*\*Session Tag\*\*: *//' || echo "none")
GIT_BRANCH=$(grep "\*\*Git Branch\*\*:" "$ACTIVE_SESSION" | sed 's/.*\*\*Git Branch\*\*: *//' || echo "none")

# Debug: Check if SESSION_ID is empty
if [ -z "$SESSION_ID" ]; then
    echo "Warning: SESSION_ID is empty, using timestamp"
    SESSION_ID=$(date +"%Y%m%d-%H%M%S")
fi

# Create end session tag
# Extract frontend name from directory (.claude → claude)
FRONTEND=$(basename $(dirname $(dirname "$0")) | sed 's/^\.//')
SESSION_END_TAG="session-${SESSION_ID}-${FRONTEND}-end"

# Git integration: Check and classify work
if command -v git &> /dev/null && [ -d .git ] && [ "$SESSION_TAG" != "none" ]; then
    # Tag the session end point
    git tag -a "$SESSION_END_TAG" -m "Session end: ${SESSION_TITLE}" 2>/dev/null && \
        [ "$SILENT_MODE" = false ] && echo "✅ Session end tagged: $SESSION_END_TAG"
    
    # Calculate session metrics
    FILES_CHANGED=$(git diff --name-only ${SESSION_TAG}..HEAD 2>/dev/null | wc -l)
    COMMITS_MADE=$(git log --oneline ${SESSION_TAG}..HEAD 2>/dev/null | wc -l)
    PATTERNS_TOUCHED=$(git diff --name-only ${SESSION_TAG}..HEAD 2>/dev/null | grep -E "layer/|\.md" | wc -l)
    
    if [ "$SILENT_MODE" = false ]; then
        echo -e "${BLUE}═══ Session Summary ═══${NC}"
        echo ""
        
        # Check current branch
        CURRENT_BRANCH=$(git branch --show-current)
        echo "Working branch: $CURRENT_BRANCH"
        echo "Session range: ${SESSION_TAG}..${SESSION_END_TAG}"
        
        # Check for uncommitted changes
        UNCOMMITTED=$(git status --porcelain | wc -l)
        if [ $UNCOMMITTED -gt 0 ]; then
            echo ""
            echo -e "${YELLOW}⚠️  Uncommitted changes detected!${NC}"
            echo "   You have $UNCOMMITTED uncommitted files"
            echo "   Strongly recommend: Commit or stash before ending session"
            echo ""
            echo "   Options:"
            echo "   1. Commit: git commit -am \"session-end: final checkpoint\""
            echo "   2. Stash: git stash -m \"session $SESSION_ID work\""
            echo ""
            read -p "Press Enter to continue anyway, or Ctrl+C to go back and commit... "
        fi
        
        # Analyze session commits
        echo ""
        echo "Session Metrics:"
        echo "- Files changed: $FILES_CHANGED"
        echo "- Commits made: $COMMITS_MADE"
        echo "- Patterns touched: $PATTERNS_TOUCHED"
        
        # Classify based on actual work
        if [ $COMMITS_MADE -eq 0 ]; then
            echo "- Classification: 🧪 EXPLORATION (no commits)"
            CLASSIFICATION="exploration"
        elif [ $PATTERNS_TOUCHED -gt 0 ]; then
            echo "- Classification: 📚 PATTERN-WORK (modified patterns)"
            CLASSIFICATION="pattern-work"
        elif [ $FILES_CHANGED -gt 10 ]; then
            echo "- Classification: 🚀 MAJOR-FEATURE (many files)"
            CLASSIFICATION="major-feature"
        elif [ $COMMITS_MADE -lt 3 ]; then
            echo "- Classification: 🔬 EXPERIMENT (few commits)"
            CLASSIFICATION="experiment"
        else
            echo "- Classification: ✨ FEATURE (normal work)"
            CLASSIFICATION="feature"
        fi
        
        # Session history preserved through tags
        echo ""
        echo -e "${GREEN}Session Preserved:${NC}"
        echo "View session work: git log ${SESSION_TAG}..${SESSION_END_TAG}"
        echo "Diff session: git diff ${SESSION_TAG}..${SESSION_END_TAG}"
        echo "Cherry-pick to main: git cherry-pick ${SESSION_TAG}..${SESSION_END_TAG}"
    fi

    # Count beliefs captured during this session
    BELIEFS_DIR="layer/surface/epistemic/beliefs"
    BELIEFS_CAPTURED=0
    BELIEFS_SUMMARY=""

    if [ -d "$BELIEFS_DIR" ]; then
        # Find beliefs modified since session start tag
        for belief_file in "$BELIEFS_DIR"/*.md; do
            [ -f "$belief_file" ] || continue
            # Skip index file
            [[ "$(basename "$belief_file")" == "_index.md" ]] && continue

            # Check if file was modified since session start
            if git diff --name-only "${SESSION_TAG}..HEAD" 2>/dev/null | grep -q "$(basename "$belief_file")"; then
                BELIEFS_CAPTURED=$((BELIEFS_CAPTURED + 1))
                # Extract belief ID and statement
                BELIEF_ID=$(basename "$belief_file" .md)
                STATEMENT=$(grep "^statement:" "$belief_file" 2>/dev/null | sed 's/^statement: *//' | head -1)
                if [ -n "$STATEMENT" ]; then
                    BELIEFS_SUMMARY="${BELIEFS_SUMMARY}\n  - **${BELIEF_ID}**: ${STATEMENT}"
                fi
            fi
        done
    fi

    if [ "$SILENT_MODE" = false ]; then
        echo ""
        echo "Beliefs Captured: $BELIEFS_CAPTURED"
        if [ $BELIEFS_CAPTURED -gt 0 ]; then
            echo -e "$BELIEFS_SUMMARY"
        fi
    fi

    # Add beliefs section to session file
    echo "" >> "$ACTIVE_SESSION"
    echo "## Beliefs Captured: ${BELIEFS_CAPTURED}" >> "$ACTIVE_SESSION"
    if [ $BELIEFS_CAPTURED -gt 0 ]; then
        echo -e "$BELIEFS_SUMMARY" >> "$ACTIVE_SESSION"
    else
        echo "_No beliefs captured this session_" >> "$ACTIVE_SESSION"
    fi

    # Add classification to session file
    echo "" >> "$ACTIVE_SESSION"
    echo "## Session Classification" >> "$ACTIVE_SESSION"
    echo "- Work Type: ${CLASSIFICATION:-unknown}" >> "$ACTIVE_SESSION"
    echo "- Files Changed: $FILES_CHANGED" >> "$ACTIVE_SESSION"
    echo "- Commits: $COMMITS_MADE" >> "$ACTIVE_SESSION"
    echo "- Patterns Modified: $PATTERNS_TOUCHED" >> "$ACTIVE_SESSION"
    echo "- Beliefs Captured: $BELIEFS_CAPTURED" >> "$ACTIVE_SESSION"
    echo "- Session Tags: ${SESSION_TAG}..${SESSION_END_TAG}" >> "$ACTIVE_SESSION"
else
    CLASSIFICATION="unclassified"
    FILES_CHANGED=0
    COMMITS_MADE=0
    PATTERNS_TOUCHED=0
fi

# Track in SQLite if database exists
DB_PATH=".patina/navigation.db"
if [ -f "$DB_PATH" ] && command -v sqlite3 &> /dev/null; then
    # Calculate session duration in minutes
    START_TIME=$(grep "\*\*Started\*\*:" "$ACTIVE_SESSION" | cut -d' ' -f2-)
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS date command
        DURATION=$(( ($(date +%s) - $(date -j -f "%Y-%m-%dT%H:%M:%SZ" "$START_TIME" +%s)) / 60 ))
    else
        # Linux date command
        DURATION=$(( ($(date +%s) - $(date -d "$START_TIME" +%s)) / 60 ))
    fi
    
    sqlite3 "$DB_PATH" "
        INSERT INTO state_transitions (
            workspace_id,
            to_state,
            transition_reason,
            metadata
        ) VALUES (
            '${SESSION_TAG}',
            'SessionEnd',
            'Session completed: ${SESSION_TITLE}',
            json_object(
                'session_id', '${SESSION_ID}',
                'end_tag', '${SESSION_END_TAG}',
                'classification', '${CLASSIFICATION}',
                'files_changed', ${FILES_CHANGED},
                'commits_made', ${COMMITS_MADE},
                'patterns_touched', ${PATTERNS_TOUCHED},
                'duration_minutes', ${DURATION:-0}
            )
        );
    " 2>/dev/null && [ "$SILENT_MODE" = false ] && echo "✅ Session end tracked in database" || \
        [ "$SILENT_MODE" = false ] && echo "⚠️  Could not update database"
fi

# Archive the session
mkdir -p .claude/context/sessions
mkdir -p layer/sessions

# Copy to both locations with ID as filename
cp "$ACTIVE_SESSION" ".claude/context/sessions/${SESSION_ID}.md"
cp "$ACTIVE_SESSION" "layer/sessions/${SESSION_ID}.md"

# Update last-session.md pointer with Git info
cat > "$LAST_SESSION_FILE" << EOF
# Last Session: ${SESSION_TITLE}

See: layer/sessions/${SESSION_ID}.md
Tags: ${SESSION_TAG}..${SESSION_END_TAG}
Classification: ${CLASSIFICATION:-unclassified}

Quick start: /session-start "continue from ${SESSION_TITLE}"
EOF

# Clean up
rm -f "$ACTIVE_SESSION"
rm -f "$LAST_UPDATE_FILE"

if [ "$SILENT_MODE" = false ]; then
    echo ""
    echo "✓ Session archived:"
    echo "  - .claude/context/sessions/${SESSION_ID}.md"
    echo "  - layer/sessions/${SESSION_ID}.md"
    echo "  - Updated last-session.md"
    
    if [ "$SESSION_TAG" != "none" ]; then
        echo ""
        echo "✓ Session preserved via tags: ${SESSION_TAG}..${SESSION_END_TAG}"
        echo "  View work: git log ${SESSION_TAG}..${SESSION_END_TAG}"
    fi
    
    echo ""
    echo "💭 Session Memory:"
    echo "  Your work is preserved in Git history and can be found by:"
    echo "  - git log --grep=\"$SESSION_TITLE\""
    echo "  - git tag | grep session"
    echo "  - patina navigate \"$SESSION_TITLE\""
fi