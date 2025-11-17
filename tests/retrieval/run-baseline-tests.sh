#!/bin/bash
# Topic 1: Retrieval Quality Baseline Tests
# Date: 2025-11-17
# Model: E5-base-v2 (768-dim)
# Dataset: 992 observations (52 passing quality filter)

set -e

echo "=========================================="
echo "Topic 1: Retrieval Quality Baseline Tests"
echo "Date: $(date +%Y-%m-%d-%H%M%S)"
echo "Model: E5-base-v2 (768 dimensions)"
echo "Dataset: 992 total observations"
echo "Filtered: 52 observations (source + reliability > 0.85)"
echo "=========================================="
echo

OUTPUT_DIR="tests/retrieval/results"
mkdir -p "$OUTPUT_DIR"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
RESULTS_FILE="$OUTPUT_DIR/baseline-$TIMESTAMP.md"

# Write header
cat > "$RESULTS_FILE" << 'EOF'
# Topic 1: Retrieval Quality Baseline Results

**Date**: $(date +"%Y-%m-%d %H:%M:%S")
**Model**: E5-base-v2 (768 dimensions)
**Dataset**: 992 total observations
**Filtered**: 52 observations (reliability > 0.85, source: session|documentation)

## Test Queries

EOF

# Test queries from test-queries.txt
queries=(
    "how should i commit changes in this project?"
    "when should i extract code to a new module?"
    "what testing commands should i run before pushing?"
    "how do i handle errors in rust?"
    "how do i track my development sessions?"
    "how do i build and test the release binary?"
    "what code quality checks are required?"
    "what are the core design principles of this project?"
    "how does patina work with different llms?"
    "what is the unix philosophy approach in patina?"
)

query_names=(
    "Git Workflow"
    "Code Organization"
    "Testing Strategy"
    "Error Handling"
    "Session Workflow"
    "Build Process"
    "Code Quality"
    "Architecture Principles"
    "LLM Integration"
    "Modular Design"
)

for i in "${!queries[@]}"; do
    query="${queries[$i]}"
    name="${query_names[$i]}"

    echo "[$((i+1))/10] Testing: $name"
    echo "Query: \"$query\""
    echo

    # Run query (filtered by default)
    result=$(patina query semantic "$query" --limit 3 --min-score 0.3)

    # Append to results file
    {
        echo "### Query $((i+1)): $name"
        echo
        echo "**Question**: \"$query\""
        echo
        echo "**Results**:"
        echo '```json'
        echo "$result"
        echo '```'
        echo
    } >> "$RESULTS_FILE"

    # Parse and display summary
    count=$(echo "$result" | jq '. | length')
    if [ "$count" -gt 0 ]; then
        top_sim=$(echo "$result" | jq -r '.[0].similarity')
        top_source=$(echo "$result" | jq -r '.[0].source_type')
        echo "  ✓ Found $count results (top similarity: $top_sim, source: $top_source)"
    else
        echo "  ⚠ No results found"
    fi
    echo
done

# Summary
{
    echo "## Summary"
    echo
    echo "All 10 queries executed against filtered dataset (52 high-quality observations)."
    echo
    echo "**Quality Filtering Active:**"
    echo "- Source types: session, documentation (excludes commit_message)"
    echo "- Reliability threshold: > 0.85"
    echo "- Deduplication: by content"
    echo
    echo "**Next Steps:**"
    echo "1. Review results for relevance and quality"
    echo "2. Compare with unfiltered search (992 observations)"
    echo "3. Document precision/recall metrics"
    echo "4. Adjust reliability threshold if needed (>= 0.85 vs > 0.85)"
} >> "$RESULTS_FILE"

echo "=========================================="
echo "✅ Baseline tests complete!"
echo "Results saved to: $RESULTS_FILE"
echo "=========================================="
