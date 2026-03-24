#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo not found" >&2
    exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "error: jq not found (required for dependency direction check)" >&2
    exit 1
fi

metadata_json=$(cargo metadata --no-deps --format-version 1)

workspace_names=$(jq -r '[.workspace_members[] as $id | .packages[] | select(.id == $id) | .name]' <<<"$metadata_json")

failures=0

check_core() {
    local dep_count
    dep_count=$(jq -r '.packages[] | select(.name == "patina-core") | (.dependencies | length)' <<<"$metadata_json")
    if [[ "$dep_count" -ne 0 ]]; then
        echo "dependency direction violation: patina-core must have zero dependencies" >&2
        failures=$((failures + 1))
    fi
}

check_protocol() {
    local invalid
    invalid=$(jq -r --argjson workspace_names "$workspace_names" '
      .packages[]
      | select(.name == "patina-protocol")
      | .dependencies
      | map(select((.name as $n | $workspace_names | index($n)) != null and .name != "patina-core"))
      | .[].name
    ' <<<"$metadata_json")

    if [[ -n "$invalid" ]]; then
        echo "dependency direction violation: patina-protocol may only depend on patina-core among workspace crates" >&2
        while IFS= read -r dep; do
            [[ -z "$dep" ]] && continue
            echo "  invalid workspace dependency: $dep" >&2
        done <<<"$invalid"
        failures=$((failures + 1))
    fi
}

echo "Checking core/protocol dependency direction..."
check_core
check_protocol

if [[ "$failures" -gt 0 ]]; then
    echo "failed: $failures dependency direction violation(s)" >&2
    exit 1
fi

echo "ok: core/protocol dependency direction follows M6a policy"
