#!/usr/bin/env bash
# Tier 3 manual lane: full local suite (CI-equivalent semantic coverage).
# Mirrors .github/workflows/test.yml step ordering.

set -euo pipefail

echo "Running full local preflight suite (Tier 3)..."
echo ""

echo "[1/12] WASM target..."
rustup target add wasm32-wasip2 2>/dev/null || true
echo ""

echo "[2/12] Build WASM children for integration tests..."
cargo build -p patina-ai-child-file-system-monitor --target wasm32-wasip2
cargo build -p patina-ai-child-content-extractor --target wasm32-wasip2
cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2
cargo build -p patina-ai-child-dedup-filter --target wasm32-wasip2
cargo build -p patina-ai-child-record-writer --target wasm32-wasip2
cargo build -p patina-ai-child-lakehouse-catalog --target wasm32-wasip2
echo ""

echo "[3/12] Tier 1 structural checks..."
bash resources/git/pre-push-checks.sh --structural-only
echo ""

echo "[4/12] DuckLake parity..."
bash resources/scripts/check-ducklake-parity.sh
echo ""

echo "[5/12] Broker integration invariants..."
bash resources/scripts/check-broker-integration.sh
echo ""

echo "[6/12] Rust formatting..."
cargo fmt --all -- --check
echo ""

echo "[7/12] Workspace clippy..."
cargo clippy --workspace -- -D warnings
echo ""

echo "[8/12] Build release..."
cargo build --release
echo ""

echo "[9/12] Schema consistency..."
./target/release/patina schema check
echo ""

echo "[10/12] Workspace tests (unit + integration via nextest)..."
cargo nextest run --workspace
echo ""

echo "[11/12] Test cargo install..."
cargo install --path . --locked --root ./test-install
echo ""

echo "[12/12] Pre-commit staged large-file guard..."
bash resources/git/pre-commit-checks.sh
echo ""

echo "Full local preflight suite passed."
