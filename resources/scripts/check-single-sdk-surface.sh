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

has_rg=false
if command -v rg >/dev/null 2>&1; then
    has_rg=true
else
    echo "warning: rg not found, falling back to grep for SDK surface checks" >&2
fi

echo "Checking for removed SDK package names..."
old_packages=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name' | grep -E '^(patina-child-sdk|patina-toy-sdk)$' || true)
if [[ -n "$old_packages" ]]; then
    echo "error: old SDK package(s) still present in workspace:"
    echo "$old_packages"
    exit 1
fi

echo "Checking for removed SDK identifiers in active code/docs/workflows..."
if $has_rg; then
    removed_sdk_refs=$(rg -n "patina_child_sdk|patina_toy_sdk|patina-child-sdk|patina-toy-sdk" \
        --glob '!resources/scripts/check-single-sdk-surface.sh' \
        --glob '!sdk/patina-sdk/README.md' \
        src sdk plugins children crates scripts resources .github CONTRIBUTING.md README.md AGENTS.md CLAUDE.md GEMINI.md 2>/dev/null || true)
else
    removed_sdk_refs=$(grep -RInE "patina_child_sdk|patina_toy_sdk|patina-child-sdk|patina-toy-sdk" \
        --exclude-dir=.git \
        --exclude-dir=target \
        src sdk plugins children crates scripts resources .github CONTRIBUTING.md README.md AGENTS.md CLAUDE.md GEMINI.md 2>/dev/null \
        | grep -v "resources/scripts/check-single-sdk-surface.sh" \
        | grep -v "sdk/patina-sdk/README.md" || true)
fi
if [[ -n "$removed_sdk_refs" ]]; then
    echo "$removed_sdk_refs"
    echo "error: found references to removed SDK surfaces"
    exit 1
fi

echo "Checking for legacy doctrine paths..."
if $has_rg; then
    legacy_path_refs=$(rg -n "plugins/sdk|plugins/ducklake|plugins/belief-verifier" \
        --glob '!layer/**' \
        --glob '!resources/scripts/check-single-sdk-surface.sh' \
        src sdk plugins children crates resources .github Cargo.toml tests 2>/dev/null || true)
else
    legacy_path_refs=$(grep -RInE "plugins/sdk|plugins/ducklake|plugins/belief-verifier" \
        --exclude-dir=.git \
        --exclude-dir=target \
        src sdk plugins children crates resources .github Cargo.toml tests 2>/dev/null \
        | grep -v "resources/scripts/check-single-sdk-surface.sh" || true)
fi
if [[ -n "$legacy_path_refs" ]]; then
    echo "$legacy_path_refs"
    echo "error: found legacy doctrine paths under plugins/"
    exit 1
fi

echo "Checking deprecated DuckLake paths are not reintroduced..."
if $has_rg; then
    deprecated_ducklake_refs=$(rg -n "children/ducklake-wasm" \
        --glob '!layer/**' \
        --glob '!resources/scripts/check-single-sdk-surface.sh' \
        --glob '!resources/scripts/check-runtime-boundaries.sh' \
        src children sdk tests Cargo.toml .github resources/scripts 2>/dev/null || true)
else
    deprecated_ducklake_refs=$(grep -RInE "children/ducklake-wasm" \
        --exclude-dir=.git \
        --exclude-dir=target \
        src children sdk tests Cargo.toml .github resources/scripts 2>/dev/null \
        | grep -v "resources/scripts/check-single-sdk-surface.sh" \
        | grep -v "resources/scripts/check-runtime-boundaries.sh" || true)
fi
if [[ -n "$deprecated_ducklake_refs" ]]; then
    echo "$deprecated_ducklake_refs"
    echo "error: found deprecated ducklake-wasm path references"
    exit 1
fi

if [[ -f "children/ducklake/src/main.rs" ]]; then
    echo "error: found legacy native ducklake entrypoint children/ducklake/src/main.rs"
    exit 1
fi

echo "Checking child manifests are present for children/..."
child_toml_refs=$(find children/ -name 'child.toml' 2>/dev/null || true)
if [[ -z "$child_toml_refs" ]]; then
    echo "error: no child.toml files found under children/"
    exit 1
fi

# Check children/ and sdk/ for native Child trait impls that bypass the SDK.
# src/ is excluded — host-side code (WASM runtime, test stubs) legitimately
# implements Child to present WASM children to the host.
if $has_rg; then
    native_child_refs=$(rg -n "impl Child for" \
        --glob '!resources/scripts/check-single-sdk-surface.sh' \
        children sdk 2>/dev/null || true)
else
    native_child_refs=$(grep -RInE "impl Child for" \
        --exclude-dir=.git --exclude-dir=target \
        children sdk 2>/dev/null \
        | grep -v "resources/scripts/check-single-sdk-surface.sh" || true)
fi
if [[ -n "$native_child_refs" ]]; then
    echo "$native_child_refs"
    echo "error: native Child trait usage found in children/sdk — use patina-sdk worlds"
    exit 1
fi

if $has_rg; then
    child_type_refs=$(rg -n "ChildType" \
        --glob '!resources/scripts/check-single-sdk-surface.sh' \
        src children sdk 2>/dev/null || true)
else
    child_type_refs=$(grep -RInE "ChildType" \
        --exclude-dir=.git --exclude-dir=target \
        src children sdk 2>/dev/null \
        | grep -v "resources/scripts/check-single-sdk-surface.sh" || true)
fi
if [[ -n "$child_type_refs" ]]; then
    echo "$child_type_refs"
    echo "error: ChildType references found in code — native child model is retired"
    exit 1
fi

if $has_rg; then
    resolve_binary_refs=$(rg -n "resolve_child_binary" \
        --glob '!resources/scripts/check-single-sdk-surface.sh' \
        src 2>/dev/null || true)
else
    resolve_binary_refs=$(grep -RInE "resolve_child_binary" \
        --exclude-dir=.git --exclude-dir=target \
        src 2>/dev/null \
        | grep -v "resources/scripts/check-single-sdk-surface.sh" || true)
fi
if [[ -n "$resolve_binary_refs" ]]; then
    echo "$resolve_binary_refs"
    echo "error: resolve_child_binary still referenced — binary-existence validation is retired"
    exit 1
fi

echo "ok: single SDK surface enforced"
