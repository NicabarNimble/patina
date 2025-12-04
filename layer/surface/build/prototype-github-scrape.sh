#!/bin/bash
# Prototype: GitHub Issue Scraping
# Demonstrates fetching and parsing GitHub issues for Patina integration
#
# Usage: ./prototype-github-scrape.sh <owner/repo>
# Example: ./prototype-github-scrape.sh dojoengine/dojo

set -euo pipefail

REPO="${1:-dojoengine/dojo}"
LIMIT="${2:-20}"

echo "🔍 Fetching issues from $REPO (limit: $LIMIT)"
echo ""

# Fetch issues as JSON
ISSUES=$(gh issue list --repo "$REPO" \
  --limit "$LIMIT" \
  --state all \
  --json number,title,body,state,labels,author,createdAt,updatedAt,url)

# Count
TOTAL=$(echo "$ISSUES" | jq 'length')
echo "📊 Found $TOTAL issues"
echo ""

# Detect bounties
echo "💰 Bounties detected:"
echo "$ISSUES" | jq -r '.[] | select(.labels | map(.name | ascii_downcase) | any(. == "bounty" or . == "onlydust" or . == "reward")) | "  #\(.number): \(.title) [\(.state)]"'
echo ""

# Good first issues
echo "🎯 Good first issues:"
echo "$ISSUES" | jq -r '.[] | select(.labels | map(.name | ascii_downcase) | any(. == "good first issue" or . == "beginner")) | "  #\(.number): \(.title) [\(.state)]"'
echo ""

# Show sample issue structure
echo "📋 Sample issue JSON structure:"
echo "$ISSUES" | jq '.[0] | {
  number,
  title,
  state,
  labels: .labels | map(.name),
  author: .author.login,
  created: .createdAt,
  updated: .updatedAt,
  url,
  body_preview: .body[:200]
}'
echo ""

# Demonstrate SQL insert format
echo "💾 Sample SQL inserts:"
echo "$ISSUES" | jq -r '.[] | @json' | head -3 | while read -r issue; do
  NUMBER=$(echo "$issue" | jq -r '.number')
  TITLE=$(echo "$issue" | jq -r '.title' | sed "s/'/''/g")
  BODY=$(echo "$issue" | jq -r '.body // ""' | sed "s/'/''/g")
  STATE=$(echo "$issue" | jq -r '.state')
  LABELS=$(echo "$issue" | jq -r '.labels | map(.name) | @json' | sed "s/'/''/g")
  AUTHOR=$(echo "$issue" | jq -r '.author.login')
  CREATED=$(echo "$issue" | jq -r '.createdAt')
  UPDATED=$(echo "$issue" | jq -r '.updatedAt')
  URL=$(echo "$issue" | jq -r '.url')

  # Detect bounty
  IS_BOUNTY=$(echo "$issue" | jq -r '
    .labels | map(.name | ascii_downcase) |
    any(. == "bounty" or . == "onlydust" or . == "reward")
  ')

  echo "INSERT INTO github_issues (number, title, body, state, labels, author, created_at, updated_at, url, is_bounty)"
  echo "VALUES ($NUMBER, '$TITLE', '${BODY:0:100}...', '$STATE', '$LABELS', '$AUTHOR', '$CREATED', '$UPDATED', '$URL', $IS_BOUNTY);"
  echo ""
done

echo "✅ Prototype complete"
echo ""
echo "Next steps:"
echo "  1. Implement in Rust: src/commands/scrape/github.rs"
echo "  2. Add GitHub tables to schema.sql"
echo "  3. Extend scry command with --include-issues flag"
echo "  4. Add FTS5 indexing for issue title + body"
