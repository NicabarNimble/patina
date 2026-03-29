#!/usr/bin/env bash
# Tier 2 pre-push lane: impact-driven cargo checks with fail-closed fallback.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)

collect_changed_files() {
    local upstream
    if upstream=$(git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null); then
        git diff --name-only "$upstream...HEAD"
        return
    fi

    if git diff --cached --quiet; then
        if git rev-parse --verify HEAD~1 >/dev/null 2>&1; then
            git diff --name-only HEAD~1..HEAD
        else
            git ls-files
        fi
    else
        git diff --cached --name-only
    fi
}

mapfile -t changed_files < <(collect_changed_files | sed '/^$/d')

if [[ "${#changed_files[@]}" -eq 0 ]]; then
    echo "ℹ No changed files detected. Running full workspace checks (fail-closed)."
    cargo clippy --workspace -- -D warnings
    cargo test --workspace
    exit 0
fi

echo "Changed files for impact analysis:"
for file in "${changed_files[@]}"; do
    echo "  - $file"
done
echo ""

run_full_workspace=false
ducklake_trigger=false
schema_trigger=false

for file in "${changed_files[@]}"; do
    case "$file" in
        Cargo.toml|Cargo.lock|rust-toolchain.toml|src/child/internal/*|src/mother/*|mother/src/*)
            run_full_workspace=true
            ;;
    esac

    case "$file" in
        children/ducklake/*|wit/*|sdk/*)
            ducklake_trigger=true
            ;;
    esac

    case "$file" in
        src/spec.rs|src/commands/spec/*|resources/schemas/*)
            schema_trigger=true
            ;;
    esac
done

if [[ "$run_full_workspace" == true ]]; then
    echo "⚠ Broad-impact files changed. Escalating to full workspace clippy+test."
    cargo clippy --workspace -- -D warnings
    cargo test --workspace
else
    metadata_json=$(cargo metadata --no-deps --format-version 1)
    mapfile -t package_rows < <(jq -r '.packages[] | [.name, .manifest_path] | @tsv' <<<"$metadata_json")

    declare -A impacted_packages=()
    for file in "${changed_files[@]}"; do
        abs_file="$repo_root/$file"
        best_package=""
        best_len=0

        for row in "${package_rows[@]}"; do
            package_name=${row%%$'\t'*}
            manifest_path=${row#*$'\t'}
            package_dir=$(dirname "$manifest_path")

            if [[ "$abs_file" == "$package_dir" || "$abs_file" == "$package_dir/"* ]]; then
                dir_len=${#package_dir}
                if (( dir_len > best_len )); then
                    best_len=$dir_len
                    best_package="$package_name"
                fi
            fi
        done

        if [[ -n "$best_package" ]]; then
            impacted_packages["$best_package"]=1
        fi
    done

    if [[ "${#impacted_packages[@]}" -eq 0 ]]; then
        echo "⚠ Could not resolve impacted package set. Escalating to full workspace checks."
        cargo clippy --workspace -- -D warnings
        cargo test --workspace
    else
        mapfile -t sorted_packages < <(printf '%s\n' "${!impacted_packages[@]}" | sort)
        echo "Impacted packages: ${sorted_packages[*]}"
        echo ""
        for package in "${sorted_packages[@]}"; do
            echo "📦 Running clippy for $package..."
            cargo clippy -p "$package" -- -D warnings
        done
        echo ""
        for package in "${sorted_packages[@]}"; do
            echo "📦 Running tests for $package..."
            cargo test -p "$package"
        done
    fi
fi

if [[ "$ducklake_trigger" == true ]]; then
    echo ""
    echo "📦 Path-triggered check: DuckLake parity"
    bash resources/scripts/check-ducklake-parity.sh
fi

if [[ "$schema_trigger" == true ]]; then
    echo ""
    echo "📦 Path-triggered check: schema consistency"
    cargo run --release --quiet -- schema check
fi

echo ""
echo "✅ Tier 2 targeted cargo lane passed."
