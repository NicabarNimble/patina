#!/bin/bash
# Pre-push checks for Patina - mirrors CI exactly
#
# Run this before pushing to avoid CI failures.
# These checks match .github/workflows/test.yml exactly.

set -e

echo "🔍 Running pre-push checks (mirrors CI)..."
echo ""

# Step 1: Check formatting (CI uses --check, not --fix)
echo "📦 [1/4] Checking Rust formatting..."
if ! cargo fmt --all -- --check; then
    echo ""
    echo "❌ Formatting check failed!"
    echo "   Run: cargo fmt --all"
    exit 1
fi
echo "   ✓ Formatting OK"
echo ""

# Step 2: Clippy with -D warnings (same as CI)
echo "📦 [2/4] Running clippy (warnings = errors)..."
if ! cargo clippy --workspace -- -D warnings; then
    echo ""
    echo "❌ Clippy failed! Fix warnings above."
    exit 1
fi
echo "   ✓ Clippy OK"
echo ""

# Step 3: Run tests
echo "📦 [3/4] Running tests..."
if ! cargo test --workspace; then
    echo ""
    echo "❌ Tests failed!"
    exit 1
fi
echo "   ✓ Tests OK"
echo ""

# Step 4: Verify cargo install works (catches dependency resolution issues)
echo "📦 [4/4] Testing cargo install --locked..."
if ! cargo install --path . --locked --root /tmp/patina-test-install 2>/dev/null; then
    echo ""
    echo "❌ cargo install --locked failed!"
    echo "   This usually means Cargo.lock is out of sync."
    echo "   Run: cargo update && cargo build"
    exit 1
fi
rm -rf /tmp/patina-test-install
echo "   ✓ Install OK"
echo ""

echo "✅ All checks passed! Ready to push."
