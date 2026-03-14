#!/usr/bin/env bash
set -euo pipefail

required_layer_dirs=(
    "layer/core"
    "layer/surface"
    "layer/dust"
)

echo "Checking layer output contract directories..."
for dir in "${required_layer_dirs[@]}"; do
    if [[ ! -d "$dir" ]]; then
        echo "error: missing required layer contract directory '$dir'"
        exit 1
    fi
done

echo "ok: layer output contract directories preserved"
