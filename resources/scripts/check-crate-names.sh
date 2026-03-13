#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo not found" >&2
    exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "error: jq not found (required for crate naming check)" >&2
    exit 1
fi

metadata_json=$(cargo metadata --no-deps --format-version 1)
workspace_root=$(jq -r '.workspace_root' <<<"$metadata_json")

failures=0

is_legacy_exact() {
    case "$1" in
        patina-pipe|patina-pipe-types|patina-doctor|patina-plugin-models|patina-plugin-repos|patina-plugin-ducklake|patina-plugin-belief-verifier|ducklake|github-connector)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

is_allowed_name() {
    local name="$1"
    local rel_manifest="$2"

    case "$rel_manifest" in
        Cargo.toml)
            [[ "$name" == "patina-ai" ]]
            return
            ;;
        sdk/patina-sdk/Cargo.toml)
            [[ "$name" == "patina-sdk" ]]
            return
            ;;
        crates/*/Cargo.toml|plugins/*/Cargo.toml|children/*/Cargo.toml)
            if [[ "$name" == patina-ai-* ]]; then
                return 0
            fi
            if [[ "$name" == patina-sdk-* ]]; then
                return 0
            fi
            if is_legacy_exact "$name"; then
                return 0
            fi
            return 1
            ;;
        *)
            return 0
            ;;
    esac
}

echo "Checking workspace crate names..."

while IFS=$'\t' read -r name manifest_path; do
    rel_manifest="${manifest_path#${workspace_root}/}"

    if is_allowed_name "$name" "$rel_manifest"; then
        continue
    fi

    failures=$((failures + 1))
    echo ""
    echo "crate name policy violation:"
    echo "  crate:    $name"
    echo "  manifest: $rel_manifest"
    echo "  expected: patina-ai-* (runtime/app/internal) or patina-sdk-* (public SDK),"
    echo "            unless explicitly grandfathered"
done < <(jq -r '.packages[] | [.name, .manifest_path] | @tsv' <<<"$metadata_json")

if [[ "$failures" -gt 0 ]]; then
    echo ""
    echo "failed: $failures crate naming violation(s)"
    exit 1
fi

echo "ok: workspace crate names follow policy"
