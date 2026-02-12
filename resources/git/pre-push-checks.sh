#!/bin/bash
# Pre-push checks for Patina - fast local gate
#
# Runs fmt, clippy, and tests before push (~1-2 min).
# CI handles the full suite (including cargo install --locked).

set -e

echo "🔍 Running pre-push checks..."
echo ""

# Step 1: WIT consistency — guest crate wit/ must match canonical wit/
# Two groups: mother-child crates need full wit/ tree, command crates need wit/command/
echo "📦 [1/4] Checking WIT consistency..."
wit_ok=true
# Mother-child guest crates: full wit/ tree (mother-child + command + deps)
for crate_dir in patina-plugin-api patina-plugin-models patina-plugin-repos; do
    if [ -d "$crate_dir/wit" ]; then
        if ! diff -r wit/ "$crate_dir/wit/" > /dev/null 2>&1; then
            echo "   ERROR: $crate_dir/wit/ differs from canonical wit/"
            echo "   Fix: cp -r wit/ $crate_dir/wit/"
            wit_ok=false
        fi
    fi
done
# Command guest crates: only wit/command/ subtree
for crate_dir in patina-command-api; do
    if [ -d "$crate_dir/wit/command" ]; then
        if ! diff -r wit/command/ "$crate_dir/wit/command/" > /dev/null 2>&1; then
            echo "   ERROR: $crate_dir/wit/command/ differs from canonical wit/command/"
            echo "   Fix: cp -r wit/command/ $crate_dir/wit/command/"
            wit_ok=false
        fi
    fi
done
if [ "$wit_ok" = false ]; then
    echo ""
    echo "❌ WIT consistency check failed!"
    exit 1
fi
echo "   ✓ WIT files consistent across all crates"
echo ""

# Step 2: Check formatting (CI uses --check, not --fix)
echo "📦 [2/4] Checking Rust formatting..."
if ! cargo fmt --all -- --check; then
    echo ""
    echo "❌ Formatting check failed!"
    echo "   Run: cargo fmt --all"
    exit 1
fi
echo "   ✓ Formatting OK"
echo ""

# Step 3: Clippy with -D warnings (same as CI)
echo "📦 [3/4] Running clippy (warnings = errors)..."
if ! cargo clippy --workspace -- -D warnings; then
    echo ""
    echo "❌ Clippy failed! Fix warnings above."
    exit 1
fi
echo "   ✓ Clippy OK"
echo ""

# Step 4: Run tests
echo "📦 [4/4] Running tests..."
if ! cargo test --workspace; then
    echo ""
    echo "❌ Tests failed!"
    exit 1
fi
echo "   ✓ Tests OK"
echo ""

echo "✅ All checks passed! Ready to push."
