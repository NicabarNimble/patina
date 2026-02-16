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
echo "📦 [1/5] Checking WIT consistency..."
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

# Step 2: WIT host.wit hard link enforcement
# Per [[wit-deps-must-be-hard-links-verified]]: all world-level
# deps/patina-host/host.wit MUST be hard links to canonical.
# Stale copies cause silent build failures when new imports are added.
echo "📦 [2/5] Checking WIT host.wit hard links..."
CANONICAL="wit/deps/patina-host/host.wit"
wit_link_ok=true
if [ ! -f "$CANONICAL" ]; then
    echo "   ERROR: canonical $CANONICAL not found"
    wit_link_ok=false
else
    # Get canonical inode (macOS: stat -f %i, Linux: stat -c %i)
    if stat -f "%i" "$CANONICAL" > /dev/null 2>&1; then
        CANONICAL_INODE=$(stat -f "%i" "$CANONICAL")
        STAT_FMT="-f %i"
    else
        CANONICAL_INODE=$(stat -c "%i" "$CANONICAL")
        STAT_FMT="-c %i"
    fi

    # Check world-level deps AND guest crate deps — same canonical file
    COPIES=(
        "wit/mother-child/deps/patina-host/host.wit"
        "wit/command/deps/patina-host/host.wit"
        "wit/task/deps/patina-host/host.wit"
        "wit/pipeline/deps/patina-host/host.wit"
        "patina-command-api/wit/command/deps/patina-host/host.wit"
        "patina-task-api/wit/task/deps/patina-host/host.wit"
        "patina-pipeline-api/wit/pipeline/deps/patina-host/host.wit"
    )
    for COPY in "${COPIES[@]}"; do
        if [ ! -f "$COPY" ]; then
            echo "   ERROR: $COPY not found"
            wit_link_ok=false
            continue
        fi

        COPY_INODE=$(stat $STAT_FMT "$COPY")
        if [ "$CANONICAL_INODE" != "$COPY_INODE" ]; then
            # Strategy 2 fallback: check content match
            if diff -q "$CANONICAL" "$COPY" > /dev/null 2>&1; then
                echo "   WARN: $COPY matches content but is NOT a hard link"
                echo "         (inode $COPY_INODE != canonical $CANONICAL_INODE)"
                echo "         Fix: rm $COPY && ln $CANONICAL $COPY"
                wit_link_ok=false
            else
                echo "   ERROR: $COPY content DIFFERS from canonical!"
                echo "         This is a split-brain — plugins compile against different host imports."
                echo "         Fix: rm $COPY && ln $CANONICAL $COPY"
                wit_link_ok=false
            fi
        fi
    done
fi
if [ "$wit_link_ok" = false ]; then
    echo ""
    echo "❌ WIT host.wit hard link check failed!"
    echo "   All world deps must be hard links to $CANONICAL"
    exit 1
fi
echo "   ✓ All world host.wit files are hard links to canonical"
echo ""

# Step 3: Check formatting (CI uses --check, not --fix)
echo "📦 [3/5] Checking Rust formatting..."
if ! cargo fmt --all -- --check; then
    echo ""
    echo "❌ Formatting check failed!"
    echo "   Run: cargo fmt --all"
    exit 1
fi
echo "   ✓ Formatting OK"
echo ""

# Step 4: Clippy with -D warnings (same as CI)
echo "📦 [4/5] Running clippy (warnings = errors)..."
if ! cargo clippy --workspace -- -D warnings; then
    echo ""
    echo "❌ Clippy failed! Fix warnings above."
    exit 1
fi
echo "   ✓ Clippy OK"
echo ""

# Step 5: Run tests
echo "📦 [5/5] Running tests..."
if ! cargo test --workspace; then
    echo ""
    echo "❌ Tests failed!"
    exit 1
fi
echo "   ✓ Tests OK"
echo ""

echo "✅ All checks passed! Ready to push."
