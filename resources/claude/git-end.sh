#!/bin/bash
# End Git work tracking and analyze patterns
# Philosophy: Learn from survival, preserve failures, build memory

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m' # No Color

GIT_WORK_FILE=".claude/context/git-work/current.md"
GIT_ARCHIVE_DIR=".claude/context/git-work/archive"

if [ ! -f "$GIT_WORK_FILE" ]; then
    echo "No active Git work tracking. Use /git-start first."
    exit 1
fi

echo -e "${BLUE}═══ Git Work Conclusion ═══${NC}"
echo ""

# Extract work description
WORK_DESC=$(grep "# Git Work:" "$GIT_WORK_FILE" | cut -d':' -f2 | xargs)
BRANCH=$(git branch --show-current)

# Show final statistics
echo -e "${GREEN}Work Summary:${NC}"
echo "  Description: $WORK_DESC"
echo "  Branch: $BRANCH"
echo "  Commits made: $(git log --since="$(date -d 'today' +%Y-%m-%d)" --oneline 2>/dev/null | wc -l)"
echo "  Files touched: $(git diff --name-only HEAD@{1}..HEAD 2>/dev/null | wc -l || echo "0")"
echo ""

# Analyze survival patterns
echo -e "${GREEN}Survival Analysis:${NC}"
echo ""

# Check if any old patterns were modified (potential refactoring)
echo "  ${CYAN}Old code modified (refactoring survivors):${NC}"
for file in $(git diff --name-only HEAD@{1}..HEAD 2>/dev/null || git diff --name-only); do
    if [ -f "$file" ]; then
        AGE=$(git log --follow --format="%ar" -- "$file" 2>/dev/null | tail -1)
        if echo "$AGE" | grep -qE "month|year"; then
            echo "    ✓ $file: survived $AGE"
        fi
    fi
done | head -5 || echo "    No old survivors modified"
echo ""

# Check for co-modification patterns
echo "  ${CYAN}Co-modification patterns:${NC}"
MODIFIED=$(git diff --name-only HEAD@{1}..HEAD 2>/dev/null || git diff --name-only)
if [ -n "$MODIFIED" ]; then
    # Group by directory
    echo "$MODIFIED" | xargs -I {} dirname {} | sort | uniq -c | sort -rn | head -3 | 
    while read count dir; do
        echo "    $dir/: $count files modified together"
    done
else
    echo "    No patterns detected"
fi
echo ""

# Check for potential failed experiments
echo "  ${CYAN}Experimental indicators:${NC}"
UNCOMMITTED=$(git status --short | wc -l)
if [ "$UNCOMMITTED" -gt 0 ]; then
    echo "    ⚠ $UNCOMMITTED uncommitted changes - might be incomplete experiment"
fi

# Check if work touched test files
if echo "$MODIFIED" | grep -qE "test|spec"; then
    echo "    ✓ Tests were modified - good practice"
fi

# Check commit message quality
COMMIT_MSGS=$(git log --since="$(date -d 'today' +%Y-%m-%d)" --format="%s" 2>/dev/null)
if echo "$COMMIT_MSGS" | grep -qE "^(feat|fix|refactor|test|docs):"; then
    echo "    ✓ Conventional commit messages used"
else
    echo "    ⚠ Consider using conventional commits (feat:, fix:, etc.)"
fi
echo ""

# Ask for classification
echo -e "${YELLOW}Classify this work for future memory:${NC}"
echo "  1) feature   - New functionality added"
echo "  2) bugfix    - Problem solved"
echo "  3) refactor  - Code improved without changing behavior"
echo "  4) experiment - Trying something (might fail)"
echo "  5) research  - Learning/exploring code"
echo ""
read -p "Classification [1-5]: " -n 1 -r CLASS_NUM
echo ""

case "$CLASS_NUM" in
    1) CLASSIFICATION="feature" ;;
    2) CLASSIFICATION="bugfix" ;;
    3) CLASSIFICATION="refactor" ;;
    4) CLASSIFICATION="experiment" ;;
    5) CLASSIFICATION="research" ;;
    *) CLASSIFICATION="unclassified" ;;
esac

# Ask for outcome
echo -e "${YELLOW}Work outcome:${NC}"
echo "  1) completed - Work is done"
echo "  2) partial   - Some progress made"
echo "  3) failed    - Didn't work out (valuable for memory!)"
echo "  4) ongoing   - Will continue later"
echo ""
read -p "Outcome [1-4]: " -n 1 -r OUTCOME_NUM
echo ""

case "$OUTCOME_NUM" in
    1) OUTCOME="completed" ;;
    2) OUTCOME="partial" ;;
    3) OUTCOME="failed" ;;
    4) OUTCOME="ongoing" ;;
    *) OUTCOME="unknown" ;;
esac

# Add conclusion to work file
cat >> "$GIT_WORK_FILE" << EOF

## Work Conclusion
**Ended**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**Classification**: $CLASSIFICATION
**Outcome**: $OUTCOME
**Final Branch**: $BRANCH
**Commits Made**: $(git log --since="$(date -d 'today' +%Y-%m-%d)" --oneline 2>/dev/null | wc -l)

### Survival Insights
$(for file in $(git diff --name-only HEAD@{1}..HEAD 2>/dev/null | head -3); do
    if [ -f "$file" ]; then
        echo "- $file: $(git log --oneline -- "$file" 2>/dev/null | wc -l) total commits, age $(git log --follow --format="%ar" -- "$file" 2>/dev/null | tail -1)"
    fi
done)

### Co-modification Patterns
$(echo "$MODIFIED" | xargs -I {} dirname {} | sort | uniq -c | sort -rn | head -3)
EOF

# Archive the work file
mkdir -p "$GIT_ARCHIVE_DIR"
ARCHIVE_FILE="$GIT_ARCHIVE_DIR/${WORK_DESC//\//-}-$(date +%Y%m%d-%H%M%S).md"
mv "$GIT_WORK_FILE" "$ARCHIVE_FILE"

# Special handling for failed experiments
if [ "$OUTCOME" = "failed" ]; then
    echo ""
    echo -e "${RED}Failed Experiment Detected!${NC}"
    echo -e "${YELLOW}This is valuable memory. Consider:${NC}"
    echo "  • Creating exp/$WORK_DESC branch to preserve attempt"
    echo "  • Documenting why it didn't work in layer/surface/"
    echo "  • This prevents trying the same failed approach again"
    echo ""
    read -p "Create experimental branch? [y/N]: " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        EXP_BRANCH="exp/${WORK_DESC//[^a-zA-Z0-9-]/-}-$(date +%Y%m%d)"
        git checkout -b "$EXP_BRANCH" 2>/dev/null || echo "Branch creation failed"
        echo "Created $EXP_BRANCH to preserve failed experiment"
        git checkout "$BRANCH"
    fi
fi

echo ""
echo -e "${GREEN}✓ Git work concluded and archived${NC}"
echo "  Archive: $ARCHIVE_FILE"
echo ""

# Provide memory hints
echo -e "${CYAN}Memory Building:${NC}"
if [ "$OUTCOME" = "completed" ] && [ "$CLASSIFICATION" = "feature" ]; then
    echo "  → Consider committing if not already done"
    echo "  → This code will be tracked for survival metrics"
elif [ "$OUTCOME" = "failed" ]; then
    echo "  → Failed experiment archived for future reference"
    echo "  → Next time similar work appears, check: $ARCHIVE_FILE"
elif [ "$OUTCOME" = "partial" ]; then
    echo "  → Partial work saved, can continue with /git-start '$WORK_DESC'"
fi
echo ""

# Show related archived work
echo -e "${GREEN}Related past work:${NC}"
find "$GIT_ARCHIVE_DIR" -name "*${WORK_DESC}*" -o -name "*${CLASSIFICATION}*" 2>/dev/null | head -3 | while read archive; do
    if [ -f "$archive" ]; then
        PAST_OUTCOME=$(grep "Outcome" "$archive" | head -1 | cut -d':' -f2 | xargs)
        echo "  • $(basename "$archive"): $PAST_OUTCOME"
    fi
done || echo "  No related work found"