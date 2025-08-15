#!/bin/bash
# Start Git-aware work tracking
# Philosophy: Provide Git context and memory, don't force workflow

set -e

# Get work description
WORK_DESC="${1:-git-work}"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}═══ Git Work: ${WORK_DESC} ═══${NC}"
echo ""

# Show current Git state
echo -e "${GREEN}Current Branch:${NC}"
git branch --show-current
echo ""

# Show recent related work (if any)
echo -e "${GREEN}Previous work on '${WORK_DESC}':${NC}"
# Search commit history for related work
RELATED_COMMITS=$(git log --grep="${WORK_DESC}" --oneline -5 2>/dev/null || echo "")
if [ -n "$RELATED_COMMITS" ]; then
    echo "$RELATED_COMMITS"
else
    echo "  No previous commits found"
fi
echo ""

# Show if there are failed experiments
echo -e "${GREEN}Experimental branches:${NC}"
EXP_BRANCHES=$(git branch -a | grep -E "exp/|experiment/" | head -5 || echo "")
if [ -n "$EXP_BRANCHES" ]; then
    echo "$EXP_BRANCHES"
else
    echo "  No experimental branches found"
fi
echo ""

# Show current working state
echo -e "${GREEN}Working Directory Status:${NC}"
STATUS=$(git status --short)
if [ -n "$STATUS" ]; then
    echo "$STATUS"
else
    echo "  Clean working directory"
fi
echo ""

# Show survival metrics for related files
echo -e "${GREEN}Code Survival Insights:${NC}"
# Find files modified in last 30 days that survived 90+ days
OLD_SURVIVORS=$(find src -type f -name "*.rs" -mtime +90 -exec sh -c 'git log -1 --format="%ar" -- "$1" | grep -q "months\|year" && echo "  ✓ $(basename "$1"): survived $(git log -1 --format="%ar" -- "$1")"' _ {} \; 2>/dev/null | head -3)
if [ -n "$OLD_SURVIVORS" ]; then
    echo "$OLD_SURVIVORS"
else
    echo "  No long-term survivors to learn from yet"
fi
echo ""

# Git best practices reminder
echo -e "${YELLOW}Git Reminders:${NC}"
echo "  • Commit frequently with descriptive messages"
echo "  • Use 'git like a scalpel, not a shotgun'"
echo "  • Prefix commits: feat:, fix:, refactor:, test:, docs:"
echo "  • Failed experiments are valuable - keep them in exp/ branches"
echo ""

# Create a tracking file for this Git work
GIT_WORK_DIR=".claude/context/git-work"
mkdir -p "$GIT_WORK_DIR"
GIT_WORK_FILE="$GIT_WORK_DIR/current.md"

cat > "$GIT_WORK_FILE" << EOF
# Git Work: ${WORK_DESC}
**Started**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**Branch**: $(git branch --show-current)
**Initial Status**: $(git status --short | wc -l) uncommitted changes

## Work Intent
${WORK_DESC}

## Git Activity Log
### $(date +"%H:%M") - Work Started
- Branch: $(git branch --show-current)
- Uncommitted: $(git status --short | wc -l) files
- Last commit: $(git log -1 --format="%h %s" 2>/dev/null || echo "none")

EOF

echo -e "${GREEN}✓ Git work tracking started${NC}"
echo "  Tracking file: $GIT_WORK_FILE"
echo ""
echo "Use /git-update to capture progress"
echo "Use /git-note for important Git insights"
echo "Use /git-end to conclude and analyze survival patterns"