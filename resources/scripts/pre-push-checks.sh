#!/bin/bash
# Pre-push checks for Patina - ensures CI will pass

set -e

echo "🔍 Running pre-push checks..."

# Rust checks
echo "📦 Rust formatting..."
cargo fmt --all

echo "📦 Rust clippy..."
cargo clippy --workspace --fix --allow-dirty --allow-staged

echo "📦 Rust tests..."
cargo test --workspace

# Go checks (if workspace exists)
if [ -d "workspace" ]; then
    echo "📦 Go formatting..."
    cd workspace
    go fmt ./...
    
    echo "📦 Go tests..."
    go test -v ./...
    cd ..
fi

echo "✅ All checks passed! Ready to push."