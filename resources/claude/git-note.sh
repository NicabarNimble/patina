#!/bin/bash
# Capture Git-specific insights and patterns
# Philosophy: Build memory from Git patterns and survival

set -e

# Get note text (everything after the command)
NOTE="${*:-No note provided}"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

GIT_WORK_FILE=".claude/context/git-work/current.md"
GIT_INSIGHTS_FILE=".claude/context/git-work/insights.md"

# Create insights file if it doesn't exist
if [ ! -f "$GIT_INSIGHTS_FILE" ]; then
    mkdir -p "$(dirname "$GIT_INSIGHTS_FILE")"
    cat > "$GIT_INSIGHTS_FILE" << EOF
# Git Insights & Patterns

Accumulated knowledge from Git work sessions.

## Survival Patterns
<!-- Patterns that have survived refactoring -->

## Failed Experiments
<!-- What didn't work and why -->

## Co-modification Patterns
<!-- Files that change together -->

## Insights Log
EOF
fi

# Analyze current Git context for the note
BRANCH=$(git branch --show-current)
RECENT_COMMIT=$(git log -1 --format="%h %s" 2>/dev/null || echo "no commits")
MODIFIED_COUNT=$(git status --short | wc -l)

echo -e "${BLUE}═══ Git Insight Captured ═══${NC}"
echo ""

# Show the note
echo -e "${GREEN}Note:${NC} $NOTE"
echo ""

# Add context-aware insights
echo -e "${CYAN}Current Context:${NC}"
echo "  Branch: $BRANCH"
echo "  Last commit: $RECENT_COMMIT"
echo "  Modified files: $MODIFIED_COUNT"
echo ""

# Look for patterns in the note
echo -e "${CYAN}Pattern Detection:${NC}"

# Check if note mentions specific files
FILES_MENTIONED=$(echo "$NOTE" | grep -oE '([a-zA-Z0-9_/-]+\.(rs|go|py|js|ts|md))' || true)
if [ -n "$FILES_MENTIONED" ]; then
    echo "  Files mentioned:"
    for file in $FILES_MENTIONED; do
        if [ -f "$file" ]; then
            AGE=$(git log --follow --format="%ar" -- "$file" 2>/dev/null | tail -1 || echo "new")
            COMMITS=$(git log --oneline -- "$file" 2>/dev/null | wc -l || echo "0")
            echo "    • $file: $AGE old, $COMMITS commits"
        fi
    done
fi

# Check for keywords that indicate patterns
if echo "$NOTE" | grep -qiE "always|never|every time|pattern|tends to|usually"; then
    echo "  ✓ Pattern indicator detected - this might be a reusable insight"
fi

if echo "$NOTE" | grep -qiE "failed|didn't work|broke|error|issue|problem"; then
    echo "  ⚠ Failure indicator - valuable negative knowledge"
fi

if echo "$NOTE" | grep -qiE "fixed|solved|works|success|better"; then
    echo "  ✓ Success indicator - solution worth remembering"
fi
echo ""

# Add to insights file
TIMESTAMP=$(date +"%Y-%m-%d %H:%M")
cat >> "$GIT_INSIGHTS_FILE" << EOF

### $TIMESTAMP
**Branch**: $BRANCH  
**Context**: $RECENT_COMMIT  
**Insight**: $NOTE
EOF

# If there's an active work session, add the note there too
if [ -f "$GIT_WORK_FILE" ]; then
    cat >> "$GIT_WORK_FILE" << EOF

### $(date +"%H:%M") - Insight
$NOTE
EOF
    echo -e "${GREEN}✓ Added to current work session${NC}"
fi

# Track insights about specific files
if [ -n "$FILES_MENTIONED" ]; then
    for file in $FILES_MENTIONED; do
        if [ -f "$file" ]; then
            # Check if this file has a pattern of changes
            CO_MODIFIED=$(git log --format="" --name-only -- "$file" | sort | uniq -c | sort -rn | head -3 | xargs)
            if [ -n "$CO_MODIFIED" ]; then
                echo ""
                echo -e "${YELLOW}Co-modification hint for $file:${NC}"
                echo "  Files that often change with $file:"
                git log --format="" --name-only -- "$file" | grep -v "^$" | grep -v "$file" | sort | uniq -c | sort -rn | head -3 | while read count cofile; do
                    echo "    • $cofile ($count times)"
                done
            fi
        fi
    done
fi

# Provide memory tips based on insight type
echo ""
echo -e "${CYAN}Memory Tip:${NC}"
if echo "$NOTE" | grep -qiE "failed|didn't work"; then
    echo "  Failed experiments are gold - they prevent repeated mistakes"
    echo "  Consider: git checkout -b exp/$(echo "$NOTE" | head -c 20 | tr ' ' '-')"
elif echo "$NOTE" | grep -qiE "always|pattern"; then
    echo "  This pattern might belong in layer/topics/ or layer/core/"
    echo "  Document it before it's forgotten"
elif echo "$NOTE" | grep -qiE "fixed|solved"; then
    echo "  Solutions that work should be committed with good messages"
    echo "  Future you will thank current you"
fi

echo ""
echo -e "${GREEN}✓ Insight captured in: $GIT_INSIGHTS_FILE${NC}"

# Show recent related insights
echo ""
echo -e "${YELLOW}Recent related insights:${NC}"
grep -B1 -A1 "$(echo "$NOTE" | awk '{print $1}')" "$GIT_INSIGHTS_FILE" 2>/dev/null | tail -6 | grep -v "^--$" || echo "  No related insights found yet"