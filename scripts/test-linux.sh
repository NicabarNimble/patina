#!/bin/bash
# Test in exact CI environment (Linux x86, matches GitHub Actions)
# Catches platform-specific issues before pushing to CI

set -e

echo "🐧 Testing in Linux container (ubuntu-latest, Rust stable)"
echo "   This matches the GitHub Actions CI environment"
echo ""

# Use rust:latest (equivalent to stable, matching dtolnay/rust-toolchain@stable)
docker run --rm \
  -v "$(pwd)":/workspace \
  -w /workspace \
  -e CARGO_TERM_COLOR=always \
  -e CARGO_HOME=/workspace/.cargo \
  -e CARGO_INCREMENTAL=0 \
  rust:latest \
  bash -c '
    set -e

    echo "📦 Installing DuckDB..."
    apt-get update -qq && apt-get install -y -qq wget unzip > /dev/null 2>&1
    wget -q https://github.com/duckdb/duckdb/releases/download/v1.1.3/libduckdb-linux-amd64.zip
    unzip -q libduckdb-linux-amd64.zip -d libduckdb
    export DUCKDB_LIB_DIR=/workspace/libduckdb
    export DUCKDB_INCLUDE_DIR=/workspace/libduckdb
    export LD_LIBRARY_PATH=/workspace/libduckdb

    echo "📥 Downloading embedding models..."
    ./scripts/download-active-model.sh

    echo ""
    echo "🧪 Running tests..."
    cargo test --workspace

    echo ""
    echo "✅ All tests passed on Linux!"
  '
