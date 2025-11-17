#!/usr/bin/env bash
# Download a specific embedding model from the registry
# Usage: ./scripts/download-model.sh <model-id>
# Example: ./scripts/download-model.sh e5-base-v2

set -euo pipefail

MODEL_ID="${1:-}"
REGISTRY="resources/models/registry.toml"

if [ -z "$MODEL_ID" ]; then
    echo "Usage: $0 <model-id>"
    echo "Example: $0 e5-base-v2"
    exit 1
fi

if [ ! -f "$REGISTRY" ]; then
    echo "Error: Registry not found at $REGISTRY"
    exit 1
fi

# Extract model section from registry
MODEL_SECTION=$(awk "/^\[models\.$MODEL_ID\]/,/^$/" "$REGISTRY")

if [ -z "$MODEL_SECTION" ]; then
    echo "Error: Model '$MODEL_ID' not found in registry"
    echo "Available models:"
    grep '^\[models\.' "$REGISTRY" | sed 's/\[models\.\(.*\)\]/  - \1/'
    exit 1
fi

# Extract fields from model section
get_field() {
    echo "$MODEL_SECTION" | grep "^$1 = " | sed -E 's/.*"(.*)".*/\1/' | head -1
}

MODEL_PATH=$(get_field "path")
DOWNLOAD_QUANTIZED=$(get_field "download_quantized")
DOWNLOAD_TOKENIZER=$(get_field "download_tokenizer")

if [ -z "$MODEL_PATH" ]; then
    echo "Error: Model path not found in registry for '$MODEL_ID'"
    exit 1
fi

# Create model directory
mkdir -p "$MODEL_PATH"

# Download quantized model
if [ -n "$DOWNLOAD_QUANTIZED" ]; then
    MODEL_FILE="$MODEL_PATH/model_quantized.onnx"
    if [ ! -f "$MODEL_FILE" ]; then
        echo "📥 Downloading quantized model for $MODEL_ID..."
        curl -L --progress-bar -o "$MODEL_FILE" "$DOWNLOAD_QUANTIZED"
        echo "   ✓ Model downloaded to $MODEL_FILE"
    else
        echo "   ✓ Model already exists: $MODEL_FILE"
    fi
else
    echo "Warning: No download_quantized URL found for $MODEL_ID"
fi

# Download tokenizer
if [ -n "$DOWNLOAD_TOKENIZER" ]; then
    TOKENIZER_FILE="$MODEL_PATH/tokenizer.json"
    if [ ! -f "$TOKENIZER_FILE" ]; then
        echo "📥 Downloading tokenizer for $MODEL_ID..."
        curl -L --progress-bar -o "$TOKENIZER_FILE" "$DOWNLOAD_TOKENIZER"
        echo "   ✓ Tokenizer downloaded to $TOKENIZER_FILE"
    else
        echo "   ✓ Tokenizer already exists: $TOKENIZER_FILE"
    fi
else
    echo "Warning: No download_tokenizer URL found for $MODEL_ID"
fi

echo ""
echo "✅ Model '$MODEL_ID' ready at $MODEL_PATH"
