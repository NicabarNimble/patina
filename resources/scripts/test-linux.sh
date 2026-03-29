#!/bin/bash
# Run Linux tests locally via Docker.
#
# Uses patina-ci-mirror image when available (full workspace support),
# falls back to rust: image (patina-pipe only).
#
# Usage:
#   ./resources/scripts/test-linux.sh                    # run all tests
#   ./resources/scripts/test-linux.sh -p patina-pipe     # run tests for one crate
#   ./resources/scripts/test-linux.sh -- --test sandbox  # pass args to cargo test

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Read Rust version from rust-toolchain.toml
RUST_VERSION=$(grep 'channel' "$PROJECT_ROOT/rust-toolchain.toml" | sed 's/.*= *"//' | sed 's/".*//')
if [ -z "$RUST_VERSION" ]; then
    RUST_VERSION="latest"
fi

CI_MIRROR_IMAGE="patina-ci-mirror"
BARE_IMAGE="rust:${RUST_VERSION}"
CONTAINER_NAME="patina-linux-test"
CACHE_VOLUME="patina-cargo-cache"
TARGET_VOLUME="patina-target-cache"

# Check Docker is available
if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found. Install Docker Desktop or Colima."
    exit 1
fi

if ! docker info &> /dev/null 2>&1; then
    echo "❌ Docker daemon not running. Start Docker Desktop or Colima."
    exit 1
fi

# Select image: CI mirror if built, bare rust otherwise
if docker image inspect "$CI_MIRROR_IMAGE" &> /dev/null 2>&1; then
    IMAGE="$CI_MIRROR_IMAGE"
    DEFAULT_CARGO_ARGS="test --workspace"
    echo "🐧 Running Linux tests via CI-mirror image (full workspace support)"
else
    IMAGE="$BARE_IMAGE"
    DEFAULT_CARGO_ARGS="test -p patina-pipe"
    echo "🐧 Running Linux tests via bare Rust image (patina-pipe only)"
    echo "   Build CI mirror for full workspace: docker build -f resources/docker/Dockerfile.ci-mirror -t patina-ci-mirror ."
fi

echo "   Rust: ${RUST_VERSION}"
echo "   Image: ${IMAGE}"
echo ""

# Build cargo test args
CARGO_ARGS="$DEFAULT_CARGO_ARGS"
if [ $# -gt 0 ]; then
    CARGO_ARGS="test $*"
fi

# Build WASM children inside container if using CI mirror and running workspace tests
WASM_BUILD_CMD=""
if [[ "$IMAGE" == "$CI_MIRROR_IMAGE" && "$CARGO_ARGS" == *"--workspace"* ]]; then
    WASM_BUILD_CMD="echo '📦 Building WASM children for integration tests...' && \
        cargo build -p patina-ai-child-file-system-monitor --target wasm32-wasip2 && \
        cargo build -p patina-ai-child-folder-text-to-parquet --target wasm32-wasip2 && \
        cargo build -p patina-ai-child-content-extractor --target wasm32-wasip2 && \
        cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2 && \
        cargo build -p patina-ai-child-dedup-filter --target wasm32-wasip2 && \
        cargo build -p patina-ai-child-record-writer --target wasm32-wasip2 && \
        cargo build -p patina-ai-child-lakehouse-catalog --target wasm32-wasip2 && \
        echo '' && "
fi

# Run tests in Docker
docker run --rm \
    --name "${CONTAINER_NAME}" \
    -v "${PROJECT_ROOT}:/src" \
    -v "${CACHE_VOLUME}:/usr/local/cargo/registry" \
    -v "${TARGET_VOLUME}:/tmp/target" \
    -w /src \
    -e CARGO_TARGET_DIR=/tmp/target \
    "${IMAGE}" \
    sh -c "
        ${WASM_BUILD_CMD}
        echo '📦 Running: cargo ${CARGO_ARGS}'
        echo ''
        echo '🐧 Linux kernel:' \$(uname -r)
        echo ''
        cargo ${CARGO_ARGS}
    "

STATUS=$?

if [ $STATUS -eq 0 ]; then
    echo ""
    echo "✅ Linux tests passed!"
else
    echo ""
    echo "❌ Linux tests failed (exit code: ${STATUS})"
fi

exit $STATUS
