#!/usr/bin/env bash
# Download the currently active embedding model (from .patina/config.toml)
# Also downloads baseline model (all-minilm-l6-v2) for unit tests
# This is used by CI to ensure tests run against production model

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASELINE_MODEL="all-minilm-l6-v2"

# Get active model from config
ACTIVE_MODEL=$("$SCRIPT_DIR/get-active-model.sh")

echo "📦 Active model: $ACTIVE_MODEL"
echo ""

# Download the active model
"$SCRIPT_DIR/download-model.sh" "$ACTIVE_MODEL"

# Download baseline model for unit tests (if not the active model)
if [ "$ACTIVE_MODEL" != "$BASELINE_MODEL" ]; then
    echo ""
    echo "📦 Baseline model for unit tests: $BASELINE_MODEL"
    echo ""
    "$SCRIPT_DIR/download-model.sh" "$BASELINE_MODEL"
fi
