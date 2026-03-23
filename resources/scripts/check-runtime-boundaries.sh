#!/usr/bin/env bash
set -euo pipefail

has_rg=false
if command -v rg >/dev/null 2>&1; then
    has_rg=true
else
    echo "warning: rg not found, falling back to grep for boundary checks" >&2
fi

required_dirs=(
    "src/beliefs"
    "src/mother"
    "src/child"
    "src/child/toy_host"
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
    "src/child/toy_host/lake.rs"
    "src/child/toy_host/ingress.rs"
    "src/child/toy_host/connector.rs"
    "src/child/toy_host/query.rs"
    "src/child/toy_host/http.rs"
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
if $has_rg; then
    legacy_host_calls=$(rg -n "crate::mother::(lake_host|graph_host|belief_host)::" src 2>/dev/null || true)
else
    legacy_host_calls=$(grep -RInE "crate::mother::(lake_host|graph_host|belief_host)::" \
        --exclude-dir=.git \
        --exclude-dir=target \
        src 2>/dev/null || true)
fi
if [[ -n "$legacy_host_calls" ]]; then
    echo "$legacy_host_calls"
    echo "error: found legacy mother host direct calls"
    exit 1
fi

echo "Checking no ducklake-wasm path references remain in active code..."
if $has_rg; then
    ducklake_wasm_refs=$(rg -n "children/ducklake-wasm" \
        --glob '!resources/scripts/check-runtime-boundaries.sh' \
        --glob '!resources/scripts/check-single-sdk-surface.sh' \
        src children sdk tests Cargo.toml .github resources/scripts 2>/dev/null || true)
else
    ducklake_wasm_refs=$(grep -RInE "children/ducklake-wasm" \
        --exclude-dir=.git \
        --exclude-dir=target \
        src children sdk tests Cargo.toml .github resources/scripts 2>/dev/null \
        | grep -v "resources/scripts/check-runtime-boundaries.sh" \
        | grep -v "resources/scripts/check-single-sdk-surface.sh" || true)
fi
if [[ -n "$ducklake_wasm_refs" ]]; then
    echo "$ducklake_wasm_refs"
    echo "error: found deprecated ducklake-wasm path reference"
    exit 1
fi

if [[ -f "children/ducklake/src/main.rs" ]]; then
    echo "error: found legacy native ducklake entrypoint children/ducklake/src/main.rs"
    exit 1
fi

echo "ok: runtime boundary drift guards enforced"
