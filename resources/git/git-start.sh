#!/bin/bash
# Universal git workflow start script
# Outputs JSON for LLM-agnostic consumption

set -euo pipefail

# Helper function to output JSON
output_json() {
    local status="$1"
    local data="$2"
    local message="$3"
    
    cat <<EOF
{
  "status": "$status",
  "data": $data,
  "message": "$message",
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
}
EOF
}

# Check if we have a session context (optional)
SESSION_TITLE=""
if [ -f ".claude/context/active-session.md" ]; then
    SESSION_TITLE=$(grep "# Session:" ".claude/context/active-session.md" 2>/dev/null | cut -d: -f2- | xargs || echo "")
elif [ -f ".patina/context/active-session.json" ]; then
    SESSION_TITLE=$(jq -r '.title // ""' ".patina/context/active-session.json" 2>/dev/null || echo "")
fi

# If no session title provided as argument, use session context
WORKFLOW_TITLE="${1:-$SESSION_TITLE}"

if [ -z "$WORKFLOW_TITLE" ]; then
    output_json "error" "null" "No workflow title provided and no active session found"
    exit 1
fi

# Check current git state
CURRENT_BRANCH=$(git branch --show-current 2>/dev/null || echo "")
if [ -z "$CURRENT_BRANCH" ]; then
    output_json "error" "null" "Not in a git repository"
    exit 1
fi

# Check for uncommitted changes
UNCOMMITTED_COUNT=$(git status --porcelain 2>/dev/null | wc -l | xargs)
HAS_STAGED=$(git diff --cached --quiet 2>/dev/null && echo "false" || echo "true")

# Check for open PRs (if gh is available)
OPEN_PR_COUNT=0
if command -v gh >/dev/null 2>&1; then
    OPEN_PR_COUNT=$(gh pr list --author @me --state open --json number 2>/dev/null | jq 'length // 0' || echo 0)
fi

# Build data object
DATA=$(cat <<EOF
{
  "current_branch": "$CURRENT_BRANCH",
  "uncommitted_files": $UNCOMMITTED_COUNT,
  "has_staged_changes": $HAS_STAGED,
  "open_prs": $OPEN_PR_COUNT,
  "base_commit": "$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")"
}
EOF
)

# Validation checks
if [ "$CURRENT_BRANCH" != "main" ] && [ "$CURRENT_BRANCH" != "master" ]; then
    output_json "warning" "$DATA" "Not on main branch. Currently on: $CURRENT_BRANCH"
    exit 0
fi

if [ "$UNCOMMITTED_COUNT" -gt 0 ]; then
    output_json "warning" "$DATA" "Found $UNCOMMITTED_COUNT uncommitted changes. Consider committing or stashing first."
    exit 0
fi

if [ "$OPEN_PR_COUNT" -gt 0 ]; then
    output_json "info" "$DATA" "You have $OPEN_PR_COUNT open PR(s). Consider finishing those first."
    exit 0
fi

# Create branch name from title
# Convert to lowercase, replace spaces with hyphens, remove special chars
BRANCH_NAME=$(echo "$WORKFLOW_TITLE" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9-]/-/g' | sed 's/--*/-/g' | sed 's/^-//;s/-$//')

# Determine branch type (could be made smarter)
BRANCH_TYPE="feat"
if [[ "$BRANCH_NAME" == *"fix"* ]] || [[ "$BRANCH_NAME" == *"bug"* ]]; then
    BRANCH_TYPE="fix"
elif [[ "$BRANCH_NAME" == *"doc"* ]] || [[ "$BRANCH_NAME" == *"docs"* ]]; then
    BRANCH_TYPE="docs"
elif [[ "$BRANCH_NAME" == *"refactor"* ]]; then
    BRANCH_TYPE="refactor"
elif [[ "$BRANCH_NAME" == *"test"* ]]; then
    BRANCH_TYPE="test"
fi

FULL_BRANCH="${BRANCH_TYPE}/${BRANCH_NAME}"

# Ensure we're up to date with remote
if ! git fetch origin main --quiet 2>/dev/null; then
    output_json "warning" "$DATA" "Could not fetch from origin. Continuing anyway."
fi

# Create and checkout branch
if ! git checkout -b "$FULL_BRANCH" 2>/dev/null; then
    output_json "error" "$DATA" "Failed to create branch: $FULL_BRANCH"
    exit 1
fi

# Build success data
SUCCESS_DATA=$(cat <<EOF
{
  "branch": "$FULL_BRANCH",
  "branch_type": "$BRANCH_TYPE",
  "base_commit": "$(git rev-parse --short HEAD)",
  "workflow_title": "$WORKFLOW_TITLE"
}
EOF
)

output_json "success" "$SUCCESS_DATA" "Git workflow started on branch: $FULL_BRANCH"