#!/bin/bash
# Pre-push checks for Patina - fast local gate
#
# Runs fmt, clippy, and tests before push (~1-2 min).
# CI handles the full suite (including cargo install --locked).

set -e

echo "🔍 Running pre-push checks..."
echo ""

# Step 1: Check formatting (CI uses --check, not --fix)
echo "📦 [1/3] Checking Rust formatting..."
if ! cargo fmt --all -- --check; then
    echo ""
    echo "❌ Formatting check failed!"
    echo "   Run: cargo fmt --all"
    exit 1
fi
echo "   ✓ Formatting OK"
echo ""

# Step 2: Clippy with -D warnings (same as CI)
echo "📦 [2/3] Running clippy (warnings = errors)..."
if ! cargo clippy --workspace -- -D warnings; then
    echo ""
    echo "❌ Clippy failed! Fix warnings above."
    exit 1
fi
echo "   ✓ Clippy OK"
echo ""

# Step 3: Run tests
echo "📦 [3/3] Running tests..."
if ! cargo test --workspace; then
    echo ""
    echo "❌ Tests failed!"
    exit 1
fi
echo "   ✓ Tests OK"
echo ""

echo "✅ All checks passed! Ready to push."
