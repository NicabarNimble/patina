#!/bin/bash
# Pre-push checks for Patina - ensures CI will pass
# Run this before pushing to avoid CI failures

set -e

echo "🔍 Running pre-push checks..."

# Rust checks
echo "📦 Rust formatting..."
cargo fmt --all

echo "📦 Rust clippy..."
cargo clippy --workspace --fix --allow-dirty --allow-staged

echo "📦 Rust tests..."
cargo test --workspace

echo "✅ All checks passed! Ready to push."