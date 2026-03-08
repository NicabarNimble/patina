#!/bin/bash
# Pre-commit checks for Patina - block large files before they enter history
#
# GitHub rejects files > 100MB. Catching at commit time avoids
# costly filter-repo rewrites later.

set -e

MAX_SIZE_KB=10240  # 10MB — generous but well under GitHub's 100MB limit

echo "🔍 Running pre-commit checks..."
echo ""

echo "📦 [1/1] Checking for large staged files (>${MAX_SIZE_KB}KB)..."
large_files=()

# Check staged files (added or modified), skip deletions
for file in $(git diff --cached --name-only --diff-filter=d); do
    # Get staged blob size (not working-tree size)
    size=$(git cat-file -s "$(git ls-files -s "$file" | awk '{print $2}')" 2>/dev/null || echo 0)
    size_kb=$((size / 1024))
    if [ "$size_kb" -gt "$MAX_SIZE_KB" ]; then
        large_files+=("$file (${size_kb}KB)")
    fi
done

if [ ${#large_files[@]} -gt 0 ]; then
    echo "   ❌ Large files detected — these would bloat git history:"
    for f in "${large_files[@]}"; do
        echo "      $f"
    done
    echo ""
    echo "   Add to .gitignore or use Git LFS instead."
    echo "   To bypass (if intentional): git commit --no-verify"
    exit 1
fi

echo "   ✓ No large files"
echo ""
echo "✅ Pre-commit checks passed!"
