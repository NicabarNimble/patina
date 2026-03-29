#!/usr/bin/env bash
# Tier 3 manual lane: full local suite (CI-equivalent semantic coverage).

set -euo pipefail

echo "🔍 Running full local preflight suite (Tier 3)..."
echo ""

echo "📦 [1/10] Tier 1 structural checks..."
bash resources/git/pre-push-checks.sh --structural-only
echo ""

echo "📦 [2/10] Rust formatting..."
cargo fmt --all -- --check
echo ""

echo "📦 [3/10] Workspace clippy..."
cargo clippy --workspace -- -D warnings
echo ""

echo "📦 [4/10] Workspace tests..."
cargo test --workspace
echo ""

echo "📦 [5/10] DuckLake parity..."
bash resources/scripts/check-ducklake-parity.sh
echo ""

echo "📦 [6/10] Schema consistency..."
cargo run --release --quiet -- schema check
echo ""

echo "📦 [7/10] Broker integration invariants..."
bash resources/scripts/check-broker-integration.sh
echo ""

echo "📦 [8/10] Build release..."
cargo build --release
echo ""

echo "📦 [9/10] Test cargo install..."
cargo install --path . --locked --root ./test-install
echo ""

echo "📦 [10/10] Pre-commit staged large-file guard..."
bash resources/git/pre-commit-checks.sh
echo ""

echo "✅ Full local preflight suite passed."
