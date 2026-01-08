#!/bin/bash
# Update current Patina session with Git-aware context
# Testing version - will replace session-update once validated

ACTIVE_SESSION=".opencode/context/active-session.md"
LAST_UPDATE_FILE=".opencode/context/.last-update"

if [ ! -f "$ACTIVE_SESSION" ]; then
    echo "No active session found. Start one with: /session-start"
    exit 1
fi

# Get last update time
LAST_UPDATE=$(cat "$LAST_UPDATE_FILE" 2>/dev/null || echo "session start")
CURRENT_TIME=$(date +"%H:%M")

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Git status check
if command -v git &> /dev/null && [ -d .git ]; then
    echo -e "${GREEN}📊 Git Status Check${NC}"
    echo ""
    
    # Get current branch
    CURRENT_BRANCH=$(git branch --show-current)
    echo "Current branch: $CURRENT_BRANCH"
    
    # Get uncommitted changes
    MODIFIED_FILES=$(git status --porcelain | grep -c "^ M")
    STAGED_FILES=$(git status --porcelain | grep -c "^M")
    UNTRACKED_FILES=$(git status --porcelain | grep -c "^??")
    TOTAL_CHANGES=$((MODIFIED_FILES + STAGED_FILES + UNTRACKED_FILES))
    
    # Get line count of changes
    LINES_CHANGED=$(git diff --stat | tail -1 | grep -oE '[0-9]+ insertion' | grep -oE '[0-9]+' || echo "0")
    
    # Get last commit time
    LAST_COMMIT_TIME=$(git log -1 --format="%ar" 2>/dev/null || echo "never")
    LAST_COMMIT_MSG=$(git log -1 --format="%s" 2>/dev/null || echo "no commits yet")
    
    # Recent commits
    echo ""
    echo "Recent commits:"
    git log --oneline -5 --decorate 2>/dev/null || echo "  No commits yet"
    
    echo ""
    echo "Working tree status:"
    if [ $TOTAL_CHANGES -eq 0 ]; then
        echo -e "${GREEN}✅ Clean working tree - all changes committed${NC}"
        echo "Last commit: $LAST_COMMIT_TIME - $LAST_COMMIT_MSG"
    else
        echo "- Modified files: $MODIFIED_FILES"
        echo "- Staged files: $STAGED_FILES"
        echo "- Untracked files: $UNTRACKED_FILES"
        echo "- Lines changed: ~$LINES_CHANGED"
        echo "- Last commit: $LAST_COMMIT_TIME"
        
        # Smart reminders based on state
        echo ""
        if [[ "$LAST_COMMIT_TIME" == *"hour"* ]] || [[ "$LAST_COMMIT_TIME" == *"hours"* ]]; then
            echo -e "${YELLOW}⚠️  Last commit was $LAST_COMMIT_TIME${NC}"
            echo "Strong recommendation: Commit your work soon"
            echo "Suggested: git add -p && git commit -m \"checkpoint: progress on session goals\""
        elif [ $LINES_CHANGED -gt 100 ]; then
            echo -e "${YELLOW}💡 Large changes detected ($LINES_CHANGED+ lines)${NC}"
            echo "Consider: Breaking into smaller commits"
            echo "Use: git add -p to stage selectively"
        elif [ $TOTAL_CHANGES -gt 0 ] && [[ "$LAST_COMMIT_TIME" == *"minutes"* ]]; then
            MINUTES=$(echo $LAST_COMMIT_TIME | grep -oE '[0-9]+' || echo "30")
            if [ "$MINUTES" -gt 30 ]; then
                echo "📝 Checkpoint reminder: Consider committing progress"
            else
                echo "✓ Recent commit detected, continue working"
            fi
        fi
    fi
    
    # Show git diff summary
    echo ""
    echo "Changes summary:"
    git diff --stat 2>/dev/null || echo "  No unstaged changes"
fi

# Add timestamp header with time span
echo "" >> "$ACTIVE_SESSION"
echo "### $CURRENT_TIME - Update (covering since $LAST_UPDATE)" >> "$ACTIVE_SESSION"

# Add git activity summary to session
if [ -d .git ]; then
    echo "" >> "$ACTIVE_SESSION"
    echo "**Git Activity:**" >> "$ACTIVE_SESSION"
    echo "- Commits this session: $(git log --oneline --since="$LAST_UPDATE" | wc -l)" >> "$ACTIVE_SESSION"
    echo "- Files changed: $TOTAL_CHANGES" >> "$ACTIVE_SESSION"
    echo "- Last commit: $LAST_COMMIT_TIME" >> "$ACTIVE_SESSION"
    echo "" >> "$ACTIVE_SESSION"
fi

# Update last update time NOW so agent sees correct window
echo "$CURRENT_TIME" > "$LAST_UPDATE_FILE"

# Session health indicator
echo ""
if [ -d .git ]; then
    if [ $TOTAL_CHANGES -eq 0 ]; then
        echo -e "Session Health: ${GREEN}🟢 Excellent${NC} (clean working tree)"
    elif [[ "$LAST_COMMIT_TIME" == *"hour"* ]]; then
        echo -e "Session Health: ${YELLOW}🟡 Good${NC} (commit recommended)"
    else
        echo -e "Session Health: ${GREEN}🟢 Good${NC} (active development)"
    fi
fi

# Prompt for direct update
echo ""
echo "Please fill in the update section in active-session.md with:"
echo "- Work completed since $LAST_UPDATE"
echo "- Key decisions and reasoning"
echo "- Challenges faced and solutions"
echo "- Patterns observed"
echo ""
echo "✓ Update marker added: $LAST_UPDATE → $CURRENT_TIME"

# Git philosophy reminder (occasional)
if [ -d .git ] && [ $((RANDOM % 3)) -eq 0 ]; then
    echo ""
    echo "💡 Git Philosophy Reminder:"
    echo "   Instead of one big commit changing everything:"
    echo "   - auth/user.rs: \"add user model\""
    echo "   - auth/login.rs: \"implement login logic\""
    echo "   - tests/auth_test.rs: \"add auth tests\""
    echo "   Each commit should have ONE clear purpose."
fi