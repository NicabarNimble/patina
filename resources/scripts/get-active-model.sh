#!/usr/bin/env bash
# Extract the active model name from .patina/config.toml
# Returns model ID (e.g., "e5-base-v2")

set -euo pipefail

CONFIG_FILE="${1:-.patina/config.toml}"

if [ ! -f "$CONFIG_FILE" ]; then
    # Default to baseline if no config exists
    echo "all-minilm-l6-v2"
    exit 0
fi

# Extract model from [embeddings] section
# Handles: model = "e5-base-v2"
MODEL=$(grep -A1 '^\[embeddings\]' "$CONFIG_FILE" | grep 'model' | sed -E 's/.*"(.*)".*/\1/' | tr -d ' ')

if [ -z "$MODEL" ]; then
    echo "all-minilm-l6-v2"
else
    echo "$MODEL"
fi
