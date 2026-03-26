#!/bin/bash
# Run Linux-specific tests locally via Docker.
#
# Uses the same Rust toolchain as rust-toolchain.toml.
# Mounts the source read-only, uses a separate target dir
# inside the container to avoid polluting macOS target/.
#
# Usage:
#   ./resources/scripts/test-linux.sh                    # run all tests
#   ./resources/scripts/test-linux.sh -p patina-pipe     # run tests for one crate
#   ./resources/scripts/test-linux.sh -- --test sandbox  # pass args to cargo test

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Read Rust version from rust-toolchain.toml
RUST_VERSION=$(grep 'channel' "$PROJECT_ROOT/rust-toolchain.toml" | sed 's/.*= *"//' | sed 's/".*//')
if [ -z "$RUST_VERSION" ]; then
    RUST_VERSION="latest"
fi

IMAGE="rust:${RUST_VERSION}"
CONTAINER_NAME="patina-linux-test"
CACHE_VOLUME="patina-cargo-cache"
TARGET_VOLUME="patina-target-cache"

echo "🐧 Running Linux tests locally via Docker"
echo "   Rust: ${RUST_VERSION}"
echo "   Image: ${IMAGE}"
echo ""

# Check Docker is available
if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found. Install Docker Desktop or Colima."
    exit 1
fi

if ! docker info &> /dev/null 2>&1; then
    echo "❌ Docker daemon not running. Start Docker Desktop or Colima."
    exit 1
fi

# Check kernel version inside Docker VM
KERNEL_VERSION=$(docker run --rm "${IMAGE}" uname -r 2>/dev/null || echo "unknown")
echo "   Docker VM kernel: ${KERNEL_VERSION}"
echo ""

# Build cargo test args
# Default: patina-pipe only. Full --workspace needs DuckDB, ONNX, wasmtime
# which aren't installed in the container. Pass explicit args to override.
CARGO_ARGS="test -p patina-pipe"
if [ $# -gt 0 ]; then
    CARGO_ARGS="test $*"
fi

# Run tests in Docker
# - Mount source at /src (read-write for Cargo.lock updates)
# - Use a named volume for cargo cache (survives across runs)
# - Use /tmp/target inside container (separate from macOS target/)
# - Exclude layer/dust (large, not needed for tests)
docker run --rm \
    --name "${CONTAINER_NAME}" \
    -v "${PROJECT_ROOT}:/src" \
    -v "${CACHE_VOLUME}:/usr/local/cargo/registry" \
    -v "${TARGET_VOLUME}:/tmp/target" \
    -w /src \
    -e CARGO_TARGET_DIR=/tmp/target \
    "${IMAGE}" \
    sh -c "
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
