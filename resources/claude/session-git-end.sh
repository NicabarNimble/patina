#!/bin/bash
# Archive current Patina session with Git work classification
# Testing version - will replace session-end once validated

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
GIT_BRANCH=$(grep "\*\*Git Branch\*\*:" "$ACTIVE_SESSION" | cut -d' ' -f2- || echo "none")

# Debug: Check if SESSION_ID is empty
if [ -z "$SESSION_ID" ]; then
    echo "Warning: SESSION_ID is empty, using timestamp"
    SESSION_ID=$(date +"%Y%m%d-%H%M%S")
fi

# Git integration: Check and classify work
if command -v git &> /dev/null && [ -d .git ] && [ "$GIT_BRANCH" != "none" ]; then
    if [ "$SILENT_MODE" = false ]; then
        echo -e "${BLUE}═══ Git Work Classification ═══${NC}"
        echo ""
        
        # Check current branch
        CURRENT_BRANCH=$(git branch --show-current)
        echo "Session branch: $GIT_BRANCH"
        echo "Current branch: $CURRENT_BRANCH"
        
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
            echo "   3. Discard: git reset --hard (DESTRUCTIVE)"
            echo ""
            read -p "Press Enter to continue anyway, or Ctrl+C to go back and commit... "
        fi
        
        # Analyze session commits
        echo ""
        echo "Session Analysis:"
        COMMIT_COUNT=$(git log --oneline $GIT_BRANCH --not main 2>/dev/null | wc -l)
        echo "- Commits on branch: $COMMIT_COUNT"
        
        if [ $COMMIT_COUNT -eq 0 ]; then
            echo "- Classification: 🧪 EXPLORATION (no commits)"
            CLASSIFICATION="exploration"
        elif [ $COMMIT_COUNT -lt 3 ]; then
            echo "- Classification: 🔬 EXPERIMENT (few commits)"
            CLASSIFICATION="experiment"
        else
            echo "- Classification: 🚀 FEATURE (substantial work)"
            CLASSIFICATION="feature"
        fi
        
        # Branch preservation options
        echo ""
        echo -e "${GREEN}Branch Preservation:${NC}"
        echo "Your session branch '$GIT_BRANCH' will be preserved as memory."
        echo ""
        echo "Future options:"
        echo "1. Merge to main: git checkout main && git merge $GIT_BRANCH"
        echo "2. Create PR: gh pr create --base main --head $GIT_BRANCH"
        echo "3. Keep as experiment: git branch -m $GIT_BRANCH exp/$SESSION_TITLE"
        echo "4. Archive: Leave as-is (becomes searchable memory)"
        echo ""
        echo "The branch will remain for future reference - failed experiments are valuable!"
    fi
    
    # Add classification to session file
    echo "" >> "$ACTIVE_SESSION"
    echo "## Session Classification" >> "$ACTIVE_SESSION"
    echo "- Work Type: ${CLASSIFICATION:-unknown}" >> "$ACTIVE_SESSION"
    echo "- Commits: $COMMIT_COUNT" >> "$ACTIVE_SESSION"
    echo "- Branch Preserved: $GIT_BRANCH" >> "$ACTIVE_SESSION"
    echo "- Final Status: $([ $UNCOMMITTED -gt 0 ] && echo 'uncommitted changes' || echo 'clean')" >> "$ACTIVE_SESSION"
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
Branch: ${GIT_BRANCH}
Classification: ${CLASSIFICATION:-unclassified}

Quick start: /session-git-start "continue from ${SESSION_TITLE}"
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
    
    if [ "$GIT_BRANCH" != "none" ]; then
        echo ""
        echo "✓ Git branch preserved: $GIT_BRANCH"
        echo "  This branch is now permanent memory - it will never be deleted"
        echo "  Failed experiments and successful features are equally valuable!"
    fi
    
    echo ""
    echo "💭 Session Memory:"
    echo "  Your work is preserved in Git history and can be found by:"
    echo "  - git log --grep=\"$SESSION_TITLE\""
    echo "  - git branch | grep session"
    echo "  - patina navigate \"$SESSION_TITLE\""
fi