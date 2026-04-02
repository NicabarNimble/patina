#!/usr/bin/env bash
# Tier 0 pre-commit checks (<5s): format + staged large file guard.

set -euo pipefail

MAX_SIZE_KB=10240 # 10MB, far below GitHub's hard reject threshold.

echo "🔍 Running pre-commit checks (Tier 0)..."
echo ""

echo "📦 [1/2] Checking Rust formatting..."
if ! cargo fmt --all -- --check; then
    echo ""
    echo "❌ Formatting check failed!"
    echo "   Run: cargo fmt --all"
    exit 1
fi
echo "   ✓ Formatting OK"
echo ""

echo "📦 [2/2] Checking for large staged files (>${MAX_SIZE_KB}KB)..."
large_files=()

while IFS= read -r file; do
    [ -n "$file" ] || continue
    staged_blob=$(git ls-files -s -- "$file" | awk '{print $2}')
    if [ -z "$staged_blob" ]; then
        continue
    fi
    size=$(git cat-file -s "$staged_blob" 2>/dev/null || printf '0')
    size_kb=$((size / 1024))
    if [ "$size_kb" -gt "$MAX_SIZE_KB" ]; then
        large_files+=("$file (${size_kb}KB)")
    fi
done < <(git diff --cached --name-only --diff-filter=d)

if [ "${#large_files[@]}" -gt 0 ]; then
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
echo "✅ Tier 0 checks passed!"
