#!/bin/bash
# Update Git work tracking with current state
# Philosophy: Show what changed, track survival, don't judge

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

GIT_WORK_FILE=".claude/context/git-work/current.md"

if [ ! -f "$GIT_WORK_FILE" ]; then
    echo "No active Git work tracking. Use /git-start first."
    exit 1
fi

echo -e "${BLUE}═══ Git Work Update ═══${NC}"
echo ""

# Calculate time elapsed
START_TIME=$(grep "Started" "$GIT_WORK_FILE" | cut -d' ' -f2 | tr -d 'T' | tr -d 'Z' | tr ':' ' ')
CURRENT_TIME=$(date -u +"%H %M %S")
# Simple elapsed time display (not perfect but good enough)
echo -e "${GREEN}Time in work:${NC} ~$(date +"%H:%M") since start"
echo ""

# Show commits made during this work
echo -e "${GREEN}Commits made this session:${NC}"
BRANCH=$(git branch --show-current)
# Get commits since work started (approximation based on time)
RECENT_COMMITS=$(git log --since="$(date -d 'today' +%Y-%m-%d)" --oneline --author="$(git config user.name)" 2>/dev/null | head -5)
if [ -n "$RECENT_COMMITS" ]; then
    echo "$RECENT_COMMITS"
else
    echo "  No commits yet in this work session"
fi
echo ""

# Show current changes
echo -e "${GREEN}Current uncommitted changes:${NC}"
git status --short
echo ""

# Show diff stats
echo -e "${GREEN}Change statistics:${NC}"
git diff --stat 2>/dev/null || echo "  No uncommitted changes"
echo ""

# Track what files are being modified together (co-modification patterns)
echo -e "${GREEN}Files modified together:${NC}"
MODIFIED_FILES=$(git diff --name-only 2>/dev/null)
if [ -n "$MODIFIED_FILES" ]; then
    echo "$MODIFIED_FILES" | head -5
    COUNT=$(echo "$MODIFIED_FILES" | wc -l)
    if [ $COUNT -gt 5 ]; then
        echo "  ... and $((COUNT - 5)) more files"
    fi
else
    echo "  No files currently modified"
fi
echo ""

# Show survival insights - files that were modified before and survived
echo -e "${GREEN}Survival patterns in modified files:${NC}"
for file in $(git diff --name-only 2>/dev/null | head -3); do
    if [ -f "$file" ]; then
        AGE=$(git log -1 --format="%ar" -- "$file" 2>/dev/null || echo "new file")
        COMMITS=$(git log --oneline -- "$file" 2>/dev/null | wc -l)
        echo "  $file: $AGE, $COMMITS commits"
    fi
done
echo ""

# Append to tracking file
cat >> "$GIT_WORK_FILE" << EOF

### $(date +"%H:%M") - Update
- Commits made: $(git log --since="$(date -d 'today' +%Y-%m-%d)" --oneline 2>/dev/null | wc -l)
- Files modified: $(git diff --name-only 2>/dev/null | wc -l)
- Lines changed: +$(git diff --stat 2>/dev/null | tail -1 | grep -oE '[0-9]+ insertion' | grep -oE '[0-9]+' || echo "0") -$(git diff --stat 2>/dev/null | tail -1 | grep -oE '[0-9]+ deletion' | grep -oE '[0-9]+' || echo "0")
- Status: $(git status --short | wc -l) uncommitted changes
EOF

echo -e "${CYAN}Git Memory Tip:${NC}"
# Rotating tips about Git patterns
TIPS=(
    "Files modified together often have hidden dependencies"
    "Code that survives 90+ days has proven its value"
    "Failed experiments (exp/ branches) teach what doesn't work"
    "Frequent small commits create better memory than large dumps"
    "Co-committed files reveal architectural relationships"
)
TIP_INDEX=$((RANDOM % ${#TIPS[@]}))
echo "  ${TIPS[$TIP_INDEX]}"
echo ""

echo -e "${GREEN}✓ Git work updated${NC}"