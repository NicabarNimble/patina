#!/usr/bin/env bash
# Benchmark multiple embedding models on test queries
# Helps identify which model provides best retrieval quality

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Test queries from Topic 0 smoke test
QUERIES=(
    "when should i extract code to a module?"
    "how do i handle errors in this project?"
    "when is optimization premature?"
    "concurrency problems with sqlite"
    "how should i prioritize what to build first?"
)

# Models to test (must be in resources/models/registry.toml)
MODELS=(
    "all-minilm-l6-v2"
    "bge-small-en-v1-5"
    "bge-base-en-v1-5"
    "e5-base-v2"
    "nomic-embed-text-v1-5"
)

RESULTS_DIR="$PROJECT_ROOT/tests/model-benchmarks"
mkdir -p "$RESULTS_DIR"

TIMESTAMP=$(date +%Y%m%d-%H%M%S)
REPORT="$RESULTS_DIR/benchmark-$TIMESTAMP.md"

echo "# Embedding Model Benchmark" > "$REPORT"
echo "**Date**: $(date)" >> "$REPORT"
echo "**Queries**: ${#QUERIES[@]}" >> "$REPORT"
echo "**Models**: ${#MODELS[@]}" >> "$REPORT"
echo "" >> "$REPORT"

# Function to test a single model
test_model() {
    local model=$1
    echo "Testing model: $model"

    # Update config to use this model
    cat > "$PROJECT_ROOT/.patina/config.toml" <<EOF
[embeddings]
model = "$model"
EOF

    # Regenerate embeddings
    echo "  Regenerating embeddings..."
    cd "$PROJECT_ROOT"
    cargo build --release --quiet
    cargo install --path . --quiet
    patina embeddings generate --force 2>&1 | grep -E "(Generated|Total)" || true

    # Create results directory for this model
    local model_dir="$RESULTS_DIR/$model"
    mkdir -p "$model_dir"

    # Test each query
    local total_similarity=0
    local query_count=0

    for query in "${QUERIES[@]}"; do
        query_count=$((query_count + 1))
        local output_file="$model_dir/query-$query_count.json"

        echo "  Query $query_count: $query"
        patina query semantic "$query" --limit 5 > "$output_file"

        # Extract top similarity score
        local top_sim=$(jq -r '.[0].similarity // 0' "$output_file")
        total_similarity=$(echo "$total_similarity + $top_sim" | bc)

        echo "    Top similarity: $top_sim"
    done

    # Calculate average similarity
    local avg_similarity=$(echo "scale=4; $total_similarity / $query_count" | bc)
    echo "  Average similarity: $avg_similarity"
    echo ""

    # Write to report
    echo "## Model: $model" >> "$REPORT"
    echo "**Average Similarity**: $avg_similarity" >> "$REPORT"
    echo "" >> "$REPORT"

    for i in "${!QUERIES[@]}"; do
        local query_num=$((i + 1))
        local output_file="$model_dir/query-$query_num.json"
        local query="${QUERIES[$i]}"
        local top_result=$(jq -r '.[0] | "\(.text) (sim: \(.similarity))"' "$output_file")

        echo "### Query $query_num: \"$query\"" >> "$REPORT"
        echo "**Top Result**: $top_result" >> "$REPORT"
        echo "" >> "$REPORT"
    done

    echo "---" >> "$REPORT"
    echo "" >> "$REPORT"
}

# Header
echo "============================================"
echo "Patina Embedding Model Benchmark"
echo "============================================"
echo ""

# Test each model
for model in "${MODELS[@]}"; do
    test_model "$model"
done

# Summary
echo "" >> "$REPORT"
echo "## Summary" >> "$REPORT"
echo "" >> "$REPORT"
echo "| Model | Avg Similarity | Notes |" >> "$REPORT"
echo "|-------|----------------|-------|" >> "$REPORT"

for model in "${MODELS[@]}"; do
    # Calculate average from results
    local model_dir="$RESULTS_DIR/$model"
    local total=0
    local count=0

    for query_file in "$model_dir"/query-*.json; do
        if [ -f "$query_file" ]; then
            local sim=$(jq -r '.[0].similarity // 0' "$query_file")
            total=$(echo "$total + $sim" | bc)
            count=$((count + 1))
        fi
    done

    if [ $count -gt 0 ]; then
        local avg=$(echo "scale=4; $total / $count" | bc)
        echo "| $model | $avg | - |" >> "$REPORT"
    fi
done

echo "" >> "$REPORT"
echo "**Recommendation**: Choose model with highest average similarity AND qualitative result review." >> "$REPORT"
echo "" >> "$REPORT"
echo "Full results saved to: \`$RESULTS_DIR/$model\`" >> "$REPORT"

# Print report
echo ""
echo "============================================"
echo "Benchmark Complete!"
echo "============================================"
echo ""
cat "$REPORT"
echo ""
echo "Report saved to: $REPORT"
