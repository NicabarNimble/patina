#!/bin/bash
# Universal git finish script  
# Finalizes branch for GitHub PR
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

# Get current branch
CURRENT_BRANCH=$(git branch --show-current 2>/dev/null || echo "")
if [ -z "$CURRENT_BRANCH" ]; then
    output_json "error" "null" "Not in a git repository"
    exit 1
fi

# Check if on main/master
if [ "$CURRENT_BRANCH" = "main" ] || [ "$CURRENT_BRANCH" = "master" ]; then
    output_json "error" "{\"branch\": \"$CURRENT_BRANCH\"}" "Already on main branch. No git workflow to finish."
    exit 0
fi

# Check for uncommitted changes
UNCOMMITTED_COUNT=$(git status --porcelain 2>/dev/null | wc -l | xargs)
if [ "$UNCOMMITTED_COUNT" -gt 0 ]; then
    DATA=$(cat <<EOF
{
  "branch": "$CURRENT_BRANCH",
  "uncommitted_files": $UNCOMMITTED_COUNT,
  "changed_files": $(git diff --name-only | wc -l | xargs),
  "staged_files": $(git diff --cached --name-only | wc -l | xargs)
}
EOF
)
    output_json "error" "$DATA" "Uncommitted changes found. Run /git-checkpoint first or commit manually."
    exit 1
fi

# Get commit count on this branch
BASE_BRANCH="main"
if ! git rev-parse --verify "main" >/dev/null 2>&1; then
    BASE_BRANCH="master"
fi

COMMIT_COUNT=$(git rev-list --count "$BASE_BRANCH".."HEAD" 2>/dev/null || echo 0)
if [ "$COMMIT_COUNT" -eq 0 ]; then
    output_json "warning" "{\"branch\": \"$CURRENT_BRANCH\"}" "No commits on this branch. Nothing to push."
    exit 0
fi

# Get commit summary
COMMITS=$(git log "$BASE_BRANCH".."HEAD" --oneline --no-decorate | head -10)

# Check if branch has upstream
HAS_UPSTREAM=$(git rev-parse --abbrev-ref --symbolic-full-name @{u} 2>/dev/null || echo "")
NEEDS_PUSH=true
LOCAL_SHA=$(git rev-parse HEAD)

if [ -n "$HAS_UPSTREAM" ]; then
    REMOTE_SHA=$(git rev-parse @{u} 2>/dev/null || echo "")
    if [ "$LOCAL_SHA" = "$REMOTE_SHA" ]; then
        NEEDS_PUSH=false
    fi
fi

# Run CI checks if available
CI_STATUS="skipped"
CI_DETAILS="No CI checks configured"

# Check for pre-push script
if [ -f ".claude/bin/pre-push-checks.sh" ]; then
    CI_STATUS="running"
    if .claude/bin/pre-push-checks.sh >/dev/null 2>&1; then
        CI_STATUS="passed"
        CI_DETAILS="All pre-push checks passed"
    else
        CI_STATUS="failed"
        CI_DETAILS="Pre-push checks failed. Fix issues before pushing."
    fi
elif [ -f ".github/workflows/test.yml" ] || [ -f ".github/workflows/ci.yml" ]; then
    # Basic checks if CI config exists
    CI_STATUS="running"
    
    # Run basic Rust checks if it's a Rust project
    if [ -f "Cargo.toml" ]; then
        if command -v cargo >/dev/null 2>&1; then
            if cargo fmt --all -- --check >/dev/null 2>&1 && \
               cargo clippy --workspace -- -D warnings >/dev/null 2>&1; then
                CI_STATUS="passed"
                CI_DETAILS="Basic Rust checks passed"
            else
                CI_STATUS="failed"
                CI_DETAILS="Rust formatting or clippy checks failed"
            fi
        else
            CI_STATUS="skipped"
            CI_DETAILS="Cargo not available for checks"
        fi
    fi
fi

# Build data object
DATA=$(cat <<EOF
{
  "branch": "$CURRENT_BRANCH",
  "base_branch": "$BASE_BRANCH",
  "commit_count": $COMMIT_COUNT,
  "needs_push": $NEEDS_PUSH,
  "has_upstream": $([ -n "$HAS_UPSTREAM" ] && echo "true" || echo "false"),
  "ci_status": "$CI_STATUS",
  "ci_details": "$CI_DETAILS",
  "commits": $(echo "$COMMITS" | jq -Rs .),
  "session_id": "$(grep "\*\*ID\*\*:" ".claude/context/active-session.md" 2>/dev/null | cut -d' ' -f2 || echo "")"
}
EOF
)

# Stop if CI failed
if [ "$CI_STATUS" = "failed" ]; then
    output_json "error" "$DATA" "$CI_DETAILS"
    exit 1
fi

# Push if needed
PUSH_RESULT="already_pushed"
if [ "$NEEDS_PUSH" = true ]; then
    if git push -u origin "$CURRENT_BRANCH" 2>/dev/null; then
        PUSH_RESULT="success"
    else
        output_json "error" "$DATA" "Failed to push branch to origin"
        exit 1
    fi
fi

# Check if PR already exists
PR_EXISTS=false
PR_NUMBER=""
PR_URL=""

if command -v gh >/dev/null 2>&1; then
    PR_INFO=$(gh pr list --head "$CURRENT_BRANCH" --json number,url --jq '.[0]' 2>/dev/null || echo "{}")
    if [ "$PR_INFO" != "{}" ] && [ -n "$PR_INFO" ]; then
        PR_EXISTS=true
        PR_NUMBER=$(echo "$PR_INFO" | jq -r '.number // ""')
        PR_URL=$(echo "$PR_INFO" | jq -r '.url // ""')
    fi
fi

# Extract workflow title from branch name
WORKFLOW_TITLE=$(echo "$CURRENT_BRANCH" | sed 's/^[^\/]*\///' | sed 's/-/ /g')

# Build success data
SUCCESS_DATA=$(cat <<EOF
{
  "branch": "$CURRENT_BRANCH",
  "base_branch": "$BASE_BRANCH",
  "commit_count": $COMMIT_COUNT,
  "push_result": "$PUSH_RESULT",
  "pr_exists": $PR_EXISTS,
  "pr_number": "$PR_NUMBER",
  "pr_url": "$PR_URL",
  "ci_status": "$CI_STATUS",
  "workflow_title": "$WORKFLOW_TITLE",
  "ready_for_pr": $([ "$PR_EXISTS" = false ] && echo "true" || echo "false")
}
EOF
)

if [ "$PR_EXISTS" = true ]; then
    output_json "success" "$SUCCESS_DATA" "Branch pushed. PR already exists: $PR_URL"
else
    output_json "success" "$SUCCESS_DATA" "Branch pushed and ready for PR. Suggested title: $WORKFLOW_TITLE"
fi