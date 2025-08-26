#!/bin/bash
# Script to verify true functional parity between original and refactored code

set -e

echo "=== Testing Functional Parity ==="

# Save current branch
CURRENT_BRANCH=$(git branch --show-current)

# Create test databases
ORIGINAL_DB="/tmp/patina_original.db"
REFACTORED_DB="/tmp/patina_refactored.db"

# Test with original code
echo "1. Testing original implementation..."
git checkout HEAD~10 2>/dev/null || git checkout 2b09d99  # Before refactor commits
cargo build --release --quiet
rm -f .patina/knowledge.db
./target/release/patina scrape --init
./target/release/patina scrape

# Extract metrics from original
echo "2. Extracting original metrics..."
./target/release/patina scrape --query "
SELECT 'functions' as metric, COUNT(*) as count FROM function_facts
UNION ALL
SELECT 'fingerprints', COUNT(*) FROM code_fingerprints  
UNION ALL
SELECT 'call_graph', COUNT(*) FROM call_graph
UNION ALL
SELECT 'documentation', COUNT(*) FROM documentation
UNION ALL
SELECT 'types', COUNT(*) FROM type_vocabulary
UNION ALL
SELECT 'imports', COUNT(*) FROM import_facts
" > /tmp/original_metrics.txt

cp .patina/knowledge.db "$ORIGINAL_DB"

# Test with refactored code
echo "3. Testing refactored implementation..."
git checkout "$CURRENT_BRANCH"
cargo build --release --quiet
rm -f .patina/knowledge.db
./target/release/patina scrape --init
./target/release/patina scrape

# Extract metrics from refactored
echo "4. Extracting refactored metrics..."
./target/release/patina scrape --query "
SELECT 'functions' as metric, COUNT(*) as count FROM function_facts
UNION ALL
SELECT 'fingerprints', COUNT(*) FROM code_fingerprints
UNION ALL
SELECT 'call_graph', COUNT(*) FROM call_graph
UNION ALL
SELECT 'documentation', COUNT(*) FROM documentation
UNION ALL
SELECT 'types', COUNT(*) FROM type_vocabulary
UNION ALL
SELECT 'imports', COUNT(*) FROM import_facts
" > /tmp/refactored_metrics.txt

cp .patina/knowledge.db "$REFACTORED_DB"

# Compare results
echo -e "\n=== COMPARISON RESULTS ==="
echo -e "\nMetric Counts:"
diff -y /tmp/original_metrics.txt /tmp/refactored_metrics.txt || true

# Detailed comparison of specific data
echo -e "\n5. Comparing fingerprint details..."
echo "Original fingerprints sample:"
duckdb "$ORIGINAL_DB" -c "SELECT name, kind, pattern, complexity FROM code_fingerprints WHERE kind='function' ORDER BY name LIMIT 5"

echo -e "\nRefactored fingerprints sample:"
duckdb "$REFACTORED_DB" -c "SELECT name, kind, pattern, complexity FROM code_fingerprints WHERE kind='function' ORDER BY name LIMIT 5"

# Check for missing functions
echo -e "\n6. Checking for missing functions..."
duckdb "$ORIGINAL_DB" -c "SELECT name FROM function_facts ORDER BY name" > /tmp/original_functions.txt
duckdb "$REFACTORED_DB" -c "SELECT name FROM function_facts ORDER BY name" > /tmp/refactored_functions.txt

if diff /tmp/original_functions.txt /tmp/refactored_functions.txt > /dev/null; then
    echo "✅ All functions match!"
else
    echo "❌ Function differences found:"
    diff /tmp/original_functions.txt /tmp/refactored_functions.txt | head -20
fi

echo -e "\n=== SUMMARY ==="
ORIG_COUNT=$(wc -l < /tmp/original_functions.txt)
REFACT_COUNT=$(wc -l < /tmp/refactored_functions.txt)
echo "Original functions: $ORIG_COUNT"
echo "Refactored functions: $REFACT_COUNT"

if [ "$ORIG_COUNT" -eq "$REFACT_COUNT" ]; then
    echo "✅ Function count matches"
else
    echo "❌ Function count mismatch!"
fi

# Cleanup
rm -f /tmp/original_*.txt /tmp/refactored_*.txt