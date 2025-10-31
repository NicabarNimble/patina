# Embedding Models

This directory contains ONNX models for generating semantic embeddings.

## Current Model: all-MiniLM-L6-v2

**Source**: [Xenova/all-MiniLM-L6-v2](https://huggingface.co/Xenova/all-MiniLM-L6-v2) (ONNX converted)

**Original**: [sentence-transformers/all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2)

**Specifications**:
- Dimensions: 384
- Max sequence length: 256 tokens
- Model size: 86.2 MB (FP32)
- Performance: ~30-50ms per embedding (Metal GPU on Apple Silicon)

**Files**:
- `all-MiniLM-L6-v2.onnx` - ONNX model (FP32)
- `tokenizer.json` - HuggingFace tokenizer

## Downloading Models

If models are missing, download them:

```bash
# Download ONNX model (FP32, 86.2 MB)
curl -L -o resources/models/all-MiniLM-L6-v2.onnx \
  https://huggingface.co/Xenova/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx

# Download tokenizer
curl -L -o resources/models/tokenizer.json \
  https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json
```

## Alternative Model Variants

Available at [Xenova/all-MiniLM-L6-v2](https://huggingface.co/Xenova/all-MiniLM-L6-v2/tree/main/onnx):

- `model.onnx` (90.4 MB) - FP32, best quality
- `model_fp16.onnx` (45.3 MB) - FP16, good quality, half size
- `model_int8.onnx` (23 MB) - INT8, faster, smaller
- `model_q4.onnx` (54.6 MB) - Q4, balanced

## Usage

Models are loaded automatically by the embeddings module (`src/embeddings/onnx.rs`).

## Why ONNX Runtime?

- **Cross-platform**: Works on Mac (Metal), Linux (CPU), Windows
- **Pure Rust**: No Python dependency at runtime
- **Production-ready**: Used by Twitter for 100M+ users
- **Same vector space**: Mac and Linux generate identical embeddings
