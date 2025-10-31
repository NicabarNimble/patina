#!/bin/bash
# Download quantized ONNX model for testing
# This downloads the INT8 quantized version (23MB) instead of FP32 (90MB)

set -e

MODEL_DIR="target/test-models"
MODEL_URL="https://huggingface.co/Xenova/all-MiniLM-L6-v2/resolve/main/onnx/model_quantized.onnx"
TOKENIZER_URL="https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json"

echo "📥 Downloading test models to $MODEL_DIR..."
mkdir -p "$MODEL_DIR"

# Download model if not present
if [ ! -f "$MODEL_DIR/all-MiniLM-L6-v2-int8.onnx" ]; then
    echo "   Downloading quantized model (23MB)..."
    curl -L --progress-bar -o "$MODEL_DIR/all-MiniLM-L6-v2-int8.onnx" "$MODEL_URL"
    echo "   ✓ Model downloaded"
else
    echo "   ✓ Model already exists"
fi

# Download tokenizer if not present
if [ ! -f "$MODEL_DIR/tokenizer.json" ]; then
    echo "   Downloading tokenizer (466KB)..."
    curl -L --progress-bar -o "$MODEL_DIR/tokenizer.json" "$TOKENIZER_URL"
    echo "   ✓ Tokenizer downloaded"
else
    echo "   ✓ Tokenizer already exists"
fi

echo ""
echo "✅ Test models ready in $MODEL_DIR"
echo ""
echo "You can now run: cargo test"
