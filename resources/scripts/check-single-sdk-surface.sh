#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo not found" >&2
    exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "error: jq not found (required for SDK surface check)" >&2
    exit 1
fi

if ! command -v rg >/dev/null 2>&1; then
    echo "error: rg not found (required for SDK surface check)" >&2
    exit 1
fi

echo "Checking for removed SDK package names..."
old_packages=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name' | rg '^(patina-child-sdk|patina-toy-sdk)$' || true)
if [[ -n "$old_packages" ]]; then
    echo "error: old SDK package(s) still present in workspace:"
    echo "$old_packages"
    exit 1
fi

echo "Checking for removed SDK identifiers in active code/docs/workflows..."
if rg -n "patina_child_sdk|patina_toy_sdk|patina-child-sdk|patina-toy-sdk" \
    --glob '!resources/scripts/check-single-sdk-surface.sh' \
    --glob '!sdk/patina-sdk/README.md' \
    src sdk plugins children crates scripts resources .github CONTRIBUTING.md README.md AGENTS.md CLAUDE.md GEMINI.md 2>/dev/null; then
    echo "error: found references to removed SDK surfaces"
    exit 1
fi

echo "Checking for legacy doctrine paths..."
if rg -n "plugins/sdk|plugins/ducklake|plugins/belief-verifier" \
    --glob '!layer/**' \
    --glob '!resources/scripts/check-single-sdk-surface.sh' \
    src sdk plugins children crates scripts resources .github Cargo.toml tests 2>/dev/null; then
    echo "error: found legacy doctrine paths under plugins/"
    exit 1
fi

echo "Checking deprecated DuckLake paths are not reintroduced..."
if rg -n "children/ducklake-wasm" \
    --glob '!layer/**' \
    --glob '!resources/scripts/check-single-sdk-surface.sh' \
    --glob '!resources/scripts/check-runtime-boundaries.sh' \
    src children sdk tests Cargo.toml .github resources/scripts 2>/dev/null; then
    echo "error: found deprecated ducklake-wasm path references"
    exit 1
fi

if [[ -f "children/ducklake/src/main.rs" ]]; then
    echo "error: found legacy native ducklake entrypoint children/ducklake/src/main.rs"
    exit 1
fi

echo "ok: single SDK surface enforced"
