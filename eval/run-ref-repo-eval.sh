#!/bin/bash
# Run eval queries for ref repo semantic search
# Computes hit rate and MRR for each repo

set -e

EVAL_FILE="eval/ref-repo-queryset.json"
RESULTS_FILE="eval/ref-repo-results.md"

echo "# Ref Repo Semantic Eval Results" > "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"
echo "Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")" >> "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

# Process each repo
for repo in gemini-cli opencode dojo codex; do
    echo "## $repo" >> "$RESULTS_FILE"
    echo "" >> "$RESULTS_FILE"

    # Get queries for this repo using jq
    queries=$(jq -r ".repos[\"$repo\"].queries | length" "$EVAL_FILE")

    total_hits=0
    total_queries=0
    mrr_sum=0

    echo "| Query | Expected | Found in Top 5 | Rank | Hit? |" >> "$RESULTS_FILE"
    echo "|-------|----------|----------------|------|------|" >> "$RESULTS_FILE"

    for i in $(seq 0 $((queries - 1))); do
        query=$(jq -r ".repos[\"$repo\"].queries[$i].query" "$EVAL_FILE")
        expected_files=$(jq -r ".repos[\"$repo\"].queries[$i].expected_files[]" "$EVAL_FILE")

        # Run scry and capture results
        results=$(patina scry "$query" --repo "$repo" 2>&1 | grep -E "^\[" | head -5 || echo "")

        # Check if any expected file is in results
        hit=0
        first_rank=0
        found_file=""

        for expected in $expected_files; do
            # Normalize path - remove leading ./
            normalized=$(echo "$expected" | sed 's|^\./||')

            # Check each result line
            rank=1
            while IFS= read -r line; do
                if echo "$line" | grep -q "$normalized"; then
                    if [ $hit -eq 0 ]; then
                        hit=1
                        first_rank=$rank
                        found_file="$normalized"
                    fi
                    break
                fi
                rank=$((rank + 1))
            done <<< "$results"

            if [ $hit -eq 1 ]; then
                break
            fi
        done

        total_queries=$((total_queries + 1))

        if [ $hit -eq 1 ]; then
            total_hits=$((total_hits + 1))
            mrr_sum=$(echo "$mrr_sum + 1.0/$first_rank" | bc -l)
            echo "| $query | $(echo "$expected_files" | head -1) | $found_file | $first_rank | YES |" >> "$RESULTS_FILE"
        else
            echo "| $query | $(echo "$expected_files" | head -1) | - | - | NO |" >> "$RESULTS_FILE"
        fi
    done

    # Calculate metrics
    if [ $total_queries -gt 0 ]; then
        hit_rate=$(echo "scale=1; $total_hits * 100 / $total_queries" | bc)
        mrr=$(echo "scale=3; $mrr_sum / $total_queries" | bc)
    else
        hit_rate=0
        mrr=0
    fi

    echo "" >> "$RESULTS_FILE"
    echo "**Summary:** Hit Rate: $hit_rate% ($total_hits/$total_queries), MRR: $mrr" >> "$RESULTS_FILE"
    echo "" >> "$RESULTS_FILE"

    echo "  $repo: Hit Rate $hit_rate%, MRR $mrr"
done

echo "" >> "$RESULTS_FILE"
echo "---" >> "$RESULTS_FILE"
echo "Hit Rate = % of queries where at least one expected file appears in top 5" >> "$RESULTS_FILE"
echo "MRR = Mean Reciprocal Rank (1/rank of first hit, averaged)" >> "$RESULTS_FILE"

echo ""
echo "Results written to $RESULTS_FILE"
