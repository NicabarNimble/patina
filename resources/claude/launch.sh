#!/bin/bash
# Launch implementation branch from current session
# Creates branch, extracts TODO, makes Draft PR

set -e

# Color codes for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check for required tools
if ! command -v git &> /dev/null; then
    echo -e "${RED}Error: git is required${NC}"
    exit 1
fi

# Parse arguments
LAUNCH_ARG="${1:-}"
if [ -z "$LAUNCH_ARG" ]; then
    echo -e "${RED}Error: Branch name required${NC}"
    echo "Usage: /launch [type/]name"
    echo "Examples:"
    echo "  /launch semantic-scraping"
    echo "  /launch experiment/semantic-scraping"
    echo "  /launch feature/semantic-scraping"
    exit 1
fi

# Check for active session
SESSION_FILE=".claude/context/active-session.md"
if [ ! -f "$SESSION_FILE" ]; then
    echo -e "${YELLOW}Warning: No active session found${NC}"
    echo "Consider starting a session with /session-git-start first"
    read -p "Continue anyway? [y/N]: " confirm
    if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
        exit 1
    fi
    SESSION_ID="manual-$(date +%Y%m%d-%H%M%S)"
else
    # Extract session info
    SESSION_ID=$(grep "\*\*ID\*\*:" "$SESSION_FILE" | cut -d' ' -f2 || echo "unknown")
fi

# Get current branch
CURRENT_BRANCH=$(git branch --show-current)

# Parse launch argument (type/name or just name)
if [[ "$LAUNCH_ARG" == *"/"* ]]; then
    BRANCH_TYPE=$(echo "$LAUNCH_ARG" | cut -d'/' -f1)
    BRANCH_NAME=$(echo "$LAUNCH_ARG" | cut -d'/' -f2-)
else
    # Auto-detect type from session content
    if [ -f "$SESSION_FILE" ] && grep -q -i "experiment" "$SESSION_FILE"; then
        BRANCH_TYPE="experiment"
    else
        BRANCH_TYPE="feature"
    fi
    BRANCH_NAME="$LAUNCH_ARG"
fi

echo -e "${BLUE}Launching $BRANCH_TYPE: $BRANCH_NAME${NC}"
echo "Parent session: $SESSION_ID"
echo "Current branch: $CURRENT_BRANCH"
echo ""

# Check branch context (similar to session-git-start logic)
IS_WORK_RELATED=false
NEW_BRANCH=""

if [[ "$CURRENT_BRANCH" == "work" ]]; then
    IS_WORK_RELATED=true
    NEW_BRANCH="$BRANCH_TYPE/$BRANCH_NAME"
    echo -e "${GREEN}✓ On work branch, will create: $NEW_BRANCH${NC}"
elif git merge-base --is-ancestor work HEAD 2>/dev/null; then
    IS_WORK_RELATED=true
    # We're on a work sub-branch, create under it
    NEW_BRANCH="$CURRENT_BRANCH/$BRANCH_NAME"
    echo -e "${GREEN}✓ On work sub-branch, will create: $NEW_BRANCH${NC}"
fi

# Handle non-work branches
if [[ "$IS_WORK_RELATED" == "false" ]]; then
    echo -e "${YELLOW}⚠️  Not on work branch or descendant${NC}"
    echo "Current branch: $CURRENT_BRANCH"
    echo ""
    echo "Options:"
    echo "1. Switch to work branch (recommended)"
    echo "2. Create work branch here"
    echo "3. Create $BRANCH_TYPE/$BRANCH_NAME anyway"
    echo "4. Cancel"
    echo ""
    read -p "Choice [1/2/3/4]: " choice
    
    case $choice in
        1) 
            git checkout work 2>/dev/null || {
                echo -e "${RED}work branch doesn't exist${NC}"
                exit 1
            }
            NEW_BRANCH="$BRANCH_TYPE/$BRANCH_NAME"
            ;;
        2) 
            git checkout -b work
            NEW_BRANCH="$BRANCH_TYPE/$BRANCH_NAME"
            ;;
        3) 
            NEW_BRANCH="$BRANCH_TYPE/$BRANCH_NAME"
            echo -e "${YELLOW}Warning: Creating branch outside work hierarchy${NC}"
            ;;
        *) 
            echo "Cancelled"
            exit 1
            ;;
    esac
fi

# Check for uncommitted changes
if [[ -n $(git status --porcelain 2>/dev/null) ]]; then
    echo -e "${YELLOW}⚠️  Warning: Uncommitted changes exist${NC}"
    echo "Consider: git stash or git commit -am 'WIP: saving work'"
    read -p "Continue anyway? [y/N]: " confirm
    if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Create the branch
echo ""
echo -e "${BLUE}Creating branch: $NEW_BRANCH${NC}"
git checkout -b "$NEW_BRANCH"

# Extract implementation details from session if it exists
TODOS=""
DESIGN=""
DECISIONS=""

if [ -f "$SESSION_FILE" ]; then
    echo "📝 Extracting implementation plan from session..."
    
    # Extract different sections (being generous with line counts)
    TODOS=$(grep -A30 -i "implementation tasks\|todo\|next:" "$SESSION_FILE" 2>/dev/null | head -40 || echo "- [ ] Define implementation tasks")
    DESIGN=$(grep -A20 -i "solution:\|design:\|approach:" "$SESSION_FILE" 2>/dev/null | head -25 || echo "Design to be defined")
    DECISIONS=$(grep -A15 -i "key decisions:\|decisions:\|decided:" "$SESSION_FILE" 2>/dev/null | head -20 || echo "Decisions to be documented")
    
    # Clean up extracted text (remove grep artifacts)
    TODOS=$(echo "$TODOS" | sed 's/--$//')
    DESIGN=$(echo "$DESIGN" | sed 's/--$//')
    DECISIONS=$(echo "$DECISIONS" | sed 's/--$//')
fi

# If no session or no content found, use placeholders
if [ -z "$TODOS" ]; then
    TODOS="- [ ] Define implementation tasks
- [ ] Create initial structure
- [ ] Write tests
- [ ] Implement core functionality"
fi

if [ -z "$DESIGN" ]; then
    DESIGN="[Design extracted from session or to be defined]"
fi

if [ -z "$DECISIONS" ]; then
    DECISIONS="[Key decisions to be documented]"
fi

# Create implementation plan document
echo "📄 Creating IMPLEMENTATION_PLAN.md..."
cat > "IMPLEMENTATION_PLAN.md" << EOF
# Implementation: $BRANCH_NAME

**Parent Session**: $SESSION_ID
**Branch**: $NEW_BRANCH
**Type**: $BRANCH_TYPE
**Created**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")

## Design
$DESIGN

## Key Decisions
$DECISIONS

## Implementation Tasks
$TODOS

## Success Criteria
- [ ] All tests pass
- [ ] Code follows project patterns
- [ ] Documentation updated

## Test Plan
- [ ] Unit tests for core functionality
- [ ] Integration tests for command
- [ ] Manual testing checklist

---
*Auto-generated from session $SESSION_ID by /launch command*
EOF

# Commit the plan
git add IMPLEMENTATION_PLAN.md
git commit -m "$BRANCH_TYPE: initialize $BRANCH_NAME from session $SESSION_ID

Created implementation plan and branch structure.
Parent session: $SESSION_ID"

# Try to create Draft PR if gh is available
if command -v gh &> /dev/null; then
    echo ""
    echo "📋 Creating Draft PR..."
    
    # Determine base branch (work or current parent)
    BASE_BRANCH="work"
    if ! git show-ref --verify --quiet refs/heads/work; then
        BASE_BRANCH="main"
    fi
    
    PR_BODY="## Parent Session
Session ID: $SESSION_ID
See: .claude/context/active-session.md

## Implementation Plan
See: [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md)

## TODO
$TODOS

## Success Criteria
- [ ] All tests pass
- [ ] Code follows patterns
- [ ] Documentation updated

## Test Plan
- [ ] Unit tests
- [ ] Integration tests
- [ ] Manual testing

---
*Auto-generated from /launch command*"

    # Create the PR (may fail if not pushed yet)
    gh pr create --draft \
        --title "[$BRANCH_TYPE] $BRANCH_NAME" \
        --body "$PR_BODY" \
        --base "$BASE_BRANCH" 2>/dev/null && \
        echo -e "${GREEN}✓ Draft PR created${NC}" || \
        echo -e "${YELLOW}Note: Push branch and run 'gh pr create --draft' to create PR${NC}"
else
    echo -e "${YELLOW}Note: GitHub CLI (gh) not found. Install it to auto-create PRs${NC}"
fi

# Update session to note the launch
if [ -f "$SESSION_FILE" ]; then
    echo "" >> "$SESSION_FILE"
    echo "### $(date +%H:%M) - Launched Implementation" >> "$SESSION_FILE"
    echo "Created branch: $NEW_BRANCH" >> "$SESSION_FILE"
    echo "Type: $BRANCH_TYPE" >> "$SESSION_FILE"
    echo "Implementation plan: IMPLEMENTATION_PLAN.md" >> "$SESSION_FILE"
fi

# Success message
echo ""
echo -e "${GREEN}✅ Launch complete!${NC}"
echo "   Branch: $NEW_BRANCH"
echo "   Plan: IMPLEMENTATION_PLAN.md"
echo ""
echo "Next steps:"
echo "1. Review and edit IMPLEMENTATION_PLAN.md"
echo "2. Push branch: git push -u origin $NEW_BRANCH"
echo "3. Create PR: gh pr create --draft"
echo "4. Start implementing TODOs"
echo ""
echo "Useful commands:"
echo "  /session-git-update  - Track progress"
echo "  /session-git-note    - Capture decisions"
echo "  /session-git-end     - Complete session"