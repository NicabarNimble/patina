#!/usr/bin/env bash
# Tier 2 pre-push lane: impact-driven cargo checks with fail-closed fallback.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)

is_test_mode() {
    [[ "${PATINA_PRE_PUSH_TEST_MODE:-0}" == "1" ]]
}

run_cargo() {
    if is_test_mode; then
        if [[ "${1:-}" == "metadata" ]]; then
            printf '%s\n' '{"packages":[]}'
            return 0
        fi
        echo "[test-mode] cargo $*"
        return 0
    fi
    cargo "$@"
}

run_script() {
    local script_path="$1"
    shift || true

    if is_test_mode; then
        echo "[test-mode] bash $script_path $*"
        return 0
    fi

    bash "$script_path" "$@"
}

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

read_changed_files() {
    if [[ -n "${PATINA_PRE_PUSH_CHANGED_FILES:-}" ]]; then
        printf '%s\n' "$PATINA_PRE_PUSH_CHANGED_FILES"
        return
    fi

    collect_changed_files
}

is_cargo_impact_file() {
    local file="$1"
    case "$file" in
        Cargo.toml|*/Cargo.toml|Cargo.lock|rust-toolchain.toml|rust-toolchain|build.rs|*/build.rs)
            return 0
            ;;
        *.rs|*.c|*.h|*.cc|*.cpp|*.cxx|*.hpp|*.hxx|*.wit)
            return 0
            ;;
        wit/*|sdk/*)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

changed_files=()
while IFS= read -r line; do
    [[ -n "$line" ]] || continue
    changed_files+=("$line")
done < <(read_changed_files | sed '/^$/d')

if [[ "${#changed_files[@]}" -eq 0 ]]; then
    echo "No changed files detected. Running full workspace checks (fail-closed)."
    run_cargo clippy --workspace -- -D warnings
    run_cargo test --workspace --lib
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
cargo_impact_files=()

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

    if is_cargo_impact_file "$file"; then
        cargo_impact_files+=("$file")
    fi
done

ran_cargo_lane=false

if [[ "$run_full_workspace" == true ]]; then
    echo "Broad-impact files changed. Escalating to full workspace clippy + unit tests."
    run_cargo clippy --workspace -- -D warnings
    run_cargo test --workspace --lib
    ran_cargo_lane=true
elif [[ "${#cargo_impact_files[@]}" -eq 0 ]]; then
    echo "No cargo-impacting files detected. Skipping cargo clippy/test lane."
else
    metadata_json=$(run_cargo metadata --no-deps --format-version 1)
    package_rows=()
    while IFS= read -r row; do
        [[ -n "$row" ]] || continue
        package_rows+=("$row")
    done < <(jq -r '.packages[] | [.name, .manifest_path] | @tsv' <<<"$metadata_json")

    impacted_packages_list=""

    add_impacted_package() {
        local package="$1"
        if [[ -z "$impacted_packages_list" ]]; then
            impacted_packages_list="$package"
            return
        fi
        if printf '%s\n' "$impacted_packages_list" | grep -Fxq "$package"; then
            return
        fi
        impacted_packages_list+=$'\n'"$package"
    }

    for file in "${cargo_impact_files[@]}"; do
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
            add_impacted_package "$best_package"
        fi
    done

    if [[ -z "$impacted_packages_list" ]]; then
        echo "Could not resolve impacted package set. Escalating to full workspace checks."
        run_cargo clippy --workspace -- -D warnings
        run_cargo test --workspace --lib
    else
        sorted_packages=()
        while IFS= read -r package; do
            [[ -n "$package" ]] || continue
            sorted_packages+=("$package")
        done < <(printf '%s\n' "$impacted_packages_list" | sort -u)
        echo "Cargo-impacting files:"
        for file in "${cargo_impact_files[@]}"; do
            echo "  - $file"
        done
        echo "Impacted packages: ${sorted_packages[*]}"
        echo ""
        for package in "${sorted_packages[@]}"; do
            echo "📦 Running clippy for $package..."
            run_cargo clippy -p "$package" -- -D warnings
        done
        echo ""
        for package in "${sorted_packages[@]}"; do
            echo "Running unit tests for $package..."
            run_cargo test -p "$package" --lib
        done
    fi

    ran_cargo_lane=true
fi

if [[ "$ducklake_trigger" == true ]]; then
    echo ""
    echo "📦 Path-triggered check: DuckLake parity"
    run_script resources/scripts/check-ducklake-parity.sh
fi

if [[ "$schema_trigger" == true ]]; then
    echo ""
    echo "📦 Path-triggered check: schema consistency"
    run_cargo run --release --quiet -- schema check
fi

echo ""
if [[ "$ran_cargo_lane" == true ]]; then
    echo "✅ Tier 2 targeted cargo lane passed."
else
    echo "✅ Tier 2 targeted checks passed (cargo lane skipped; no cargo-impacting paths)."
fi
