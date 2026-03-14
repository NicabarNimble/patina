#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
    echo "error: rg not found (required for boundary checks)" >&2
    exit 1
fi

required_dirs=(
    "src/beliefs"
    "src/mother"
    "src/child"
    "src/toys"
    "src/core_tools"
)

echo "Checking required runtime boundary roots..."
for dir in "${required_dirs[@]}"; do
    if [[ ! -d "$dir" ]]; then
        echo "error: missing required boundary root '$dir'"
        exit 1
    fi
done

required_toy_modules=(
    "src/toys/catalog.rs"
    "src/toys/lake.rs"
    "src/toys/ingress.rs"
    "src/toys/connector.rs"
    "src/toys/query.rs"
    "src/toys/http.rs"
)

echo "Checking canonical toy module surfaces..."
for file in "${required_toy_modules[@]}"; do
    if [[ ! -f "$file" ]]; then
        echo "error: missing canonical toy module '$file'"
        exit 1
    fi
done

echo "Checking legacy runtime boundary paths are absent..."
if [[ -f "src/mother/lake_host.rs" ]]; then
    echo "error: legacy lake host path src/mother/lake_host.rs reintroduced"
    exit 1
fi

echo "Checking no direct mother::*host calls remain in active runtime..."
if rg -n "crate::mother::(lake_host|graph_host|belief_host)::" src 2>/dev/null; then
    echo "error: found legacy mother host direct calls"
    exit 1
fi

echo "Checking no ducklake-wasm path references remain in active code..."
if rg -n "children/ducklake-wasm" \
    --glob '!resources/scripts/check-runtime-boundaries.sh' \
    --glob '!resources/scripts/check-single-sdk-surface.sh' \
    src children sdk tests Cargo.toml .github resources/scripts 2>/dev/null; then
    echo "error: found deprecated ducklake-wasm path reference"
    exit 1
fi

if [[ -f "children/ducklake/src/main.rs" ]]; then
    echo "error: found legacy native ducklake entrypoint children/ducklake/src/main.rs"
    exit 1
fi

echo "ok: runtime boundary drift guards enforced"
