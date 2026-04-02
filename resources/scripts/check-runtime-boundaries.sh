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
)

echo "Checking required runtime boundary roots..."
for dir in "${required_dirs[@]}"; do
    if [[ ! -d "$dir" ]]; then
        echo "error: missing required boundary root '$dir'"
        exit 1
    fi
done

echo "Checking canonical toy module surfaces..."
toy_mod_file="src/child/toy_host/mod.rs"
if [[ ! -f "$toy_mod_file" ]]; then
    echo "error: missing toy host module root '$toy_mod_file'"
    exit 1
fi
toy_modules=$(sed -nE 's/^pub mod ([a-zA-Z0-9_]+);$/\1/p' "$toy_mod_file")
if [[ -z "$toy_modules" ]]; then
    echo "error: no public toy modules declared in $toy_mod_file"
    exit 1
fi
while IFS= read -r module; do
    [[ -n "$module" ]] || continue
    module_file="src/child/toy_host/${module}.rs"
    if [[ ! -f "$module_file" ]]; then
        echo "error: missing canonical toy module '$module_file'"
        exit 1
    fi
done <<< "$toy_modules"

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

echo "Checking no hardcoded .patina/ paths outside allowed modules..."
# Allowed: paths.rs (canonical), migration.rs (old paths by design), init/ (creates .patina/),
# test code (temp dirs), project/internal.rs (detection/config), secrets/ (vault paths).
# This guard catches NEW production code hardcoding .patina/ instead of using crate::paths.
if $has_rg; then
    patina_path_violations=$(rg -n '\.join\(".patina"\)' src/ --glob '*.rs' \
        --glob '!src/paths.rs' \
        --glob '!src/migration.rs' \
        --glob '!src/commands/init/*' \
        --glob '!src/project/*' \
        --glob '!src/secrets/*' \
        --glob '!src/connect/*' \
        --glob '!src/session/*' \
        --glob '!src/commands/repo/*' \
        --glob '!src/commands/schema/*' \
        --glob '!src/test_support.rs' 2>/dev/null \
        | grep -v 'mod tests' \
        | grep -v 'fn test_' \
        | grep -v '#\[test\]' \
        | grep -v '// assert!' \
        | grep -v 'assert!' \
        | grep -v 'temp.*\.join' \
        | grep -v 'unwrap()' || true)
else
    patina_path_violations=$(grep -RInE '\.join\(".patina"\)' src/ --include='*.rs' \
        --exclude-dir=.git --exclude-dir=target 2>/dev/null \
        | grep -v 'src/paths.rs' \
        | grep -v 'src/migration.rs' \
        | grep -v 'src/commands/init/' \
        | grep -v 'src/project/' \
        | grep -v 'src/secrets/' \
        | grep -v 'src/connect/' \
        | grep -v 'src/session/' \
        | grep -v 'src/commands/repo/' \
        | grep -v 'src/commands/schema/' \
        | grep -v 'src/test_support.rs' \
        | grep -v 'mod tests' \
        | grep -v 'fn test_' \
        | grep -v '#\[test\]' \
        | grep -v '// assert!' \
        | grep -v 'assert!' \
        | grep -v 'temp.*\.join' \
        | grep -v 'unwrap()' || true)
fi
if [[ -n "$patina_path_violations" ]]; then
    echo "$patina_path_violations"
    echo "error: hardcoded .patina/ path found — use crate::paths instead"
    exit 1
fi

echo "Checking no blanket #![allow(dead_code)] annotations..."
if $has_rg; then
    blanket_dead_code=$(rg -n '#!\[allow\(dead_code\)\]' src/ --glob '*.rs' 2>/dev/null || true)
else
    blanket_dead_code=$(grep -RInE '#!\[allow\(dead_code\)\]' src/ --include='*.rs' \
        --exclude-dir=.git --exclude-dir=target 2>/dev/null || true)
fi
if [[ -n "$blanket_dead_code" ]]; then
    echo "$blanket_dead_code"
    echo "error: blanket #![allow(dead_code)] found — use per-item #[allow(dead_code)] instead"
    exit 1
fi

echo "ok: runtime boundary drift guards enforced"
