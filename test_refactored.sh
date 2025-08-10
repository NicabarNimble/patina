#!/bin/bash
# Test script to compare original vs refactored modules

set -e

echo "========================================="
echo "Testing Refactored vs Original Modules"
echo "========================================="
echo ""

# Create test directory
TEST_DIR="/tmp/patina_test_$$"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

# Function to run command with both versions
test_module() {
    local module_name="$1"
    local env_var="$2"
    local test_command="$3"
    
    echo "Testing: $module_name"
    echo "----------------------------------------"
    
    # Run with original
    echo "Original version:"
    unset "$env_var"
    eval "$test_command" > original_output.txt 2>&1 || true
    
    # Run with refactored
    echo "Refactored version:"
    export "$env_var"=1
    eval "$test_command" > refactored_output.txt 2>&1 || true
    
    # Compare outputs
    if diff -u original_output.txt refactored_output.txt > diff_output.txt 2>&1; then
        echo "✅ Outputs are identical"
    else
        echo "❌ Outputs differ:"
        head -20 diff_output.txt
    fi
    
    unset "$env_var"
    echo ""
}

# Change to patina directory
cd /Users/nicabar/Projects/Sandbox/AI/RUST/patina

echo "1. Testing INIT command"
echo "========================================="
rm -rf "$TEST_DIR/test_init_orig" "$TEST_DIR/test_init_refact"
unset PATINA_USE_REFACTORED_INIT
cargo run -- init "$TEST_DIR/test_init_orig" --llm claude 2>&1 | tee "$TEST_DIR/init_orig.log"
export PATINA_USE_REFACTORED_INIT=1
cargo run -- init "$TEST_DIR/test_init_refact" --llm claude 2>&1 | tee "$TEST_DIR/init_refact.log"
unset PATINA_USE_REFACTORED_INIT

# Compare structure
echo "Comparing directory structures..."
if diff -r "$TEST_DIR/test_init_orig" "$TEST_DIR/test_init_refact" > "$TEST_DIR/init_diff.txt" 2>&1; then
    echo "✅ INIT: Directory structures are identical"
else
    echo "❌ INIT: Directory structures differ"
    head -20 "$TEST_DIR/init_diff.txt"
fi
echo ""

echo "2. Testing NAVIGATE command"
echo "========================================="
# Test navigate with sample query
test_module "NAVIGATE" "PATINA_USE_REFACTORED_NAVIGATE" "cargo run -- navigate 'layer patterns' 2>&1"

echo "3. Testing AGENT command"
echo "========================================="
# Test agent status (should work without starting)
test_module "AGENT" "PATINA_USE_REFACTORED_AGENT" "cargo run -- agent status 2>&1"

echo "4. Testing DOCTOR command (uses indexer internally)"
echo "========================================="
test_module "DOCTOR (indexer)" "PATINA_USE_REFACTORED_INDEXER" "cargo run -- doctor 2>&1"

echo ""
echo "========================================="
echo "Summary of test results above"
echo "========================================="

# Cleanup
rm -rf "$TEST_DIR"