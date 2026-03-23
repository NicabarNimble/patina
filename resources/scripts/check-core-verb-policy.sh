#!/usr/bin/env bash
set -euo pipefail

# Verify core-verb runtime policy for Mother-off and Mother-on modes.
#
# Determinism contract:
# - `--mode off --isolated` runs against a fresh temporary PATINA_HOME.
# - `--mode on` is an integration check against a live daemon/runtime.
#
# This script does not start/stop Mother automatically.
# Run with --mode off when Mother is stopped, and --mode on when Mother is running.

mode="both"
isolated=false
if [[ $# -gt 0 ]]; then
    while [[ $# -gt 0 ]]; do
        case "$1" in
        --mode)
            if [[ $# -lt 2 ]]; then
                echo "error: --mode requires one of: off, on, both" >&2
                exit 1
            fi
            mode="$2"
            shift 2
            ;;
        --isolated)
            isolated=true
            shift
            ;;
        --help|-h)
            echo "Usage: resources/scripts/check-core-verb-policy.sh [--mode off|on|both] [--isolated]"
            exit 0
            ;;
        *)
            echo "error: unknown argument '$1'" >&2
            echo "Usage: resources/scripts/check-core-verb-policy.sh [--mode off|on|both] [--isolated]" >&2
            exit 1
            ;;
        esac
    done
fi

case "$mode" in
off|on|both) ;;
*)
    echo "error: invalid mode '$mode' (expected off, on, or both)" >&2
    exit 1
    ;;
esac

if $isolated && [[ "$mode" != "off" ]]; then
    echo "error: --isolated is supported only with --mode off" >&2
    echo "hint: use --mode off --isolated for deterministic local checks" >&2
    exit 1
fi

if ! command -v patina >/dev/null 2>&1; then
    echo "error: patina CLI not found in PATH" >&2
    exit 1
fi

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
cd "$repo_root"

patina_home_prefix=()
if $isolated; then
    temp_home=$(mktemp -d)
    trap 'rm -rf "$temp_home"' EXIT
    patina_home_prefix=(env PATINA_HOME="$temp_home")
fi

mother_running() {
    local out
    out=$(patina mother status 2>&1 || true)
    [[ "$out" == *"Mother daemon: running"* ]]
}

require_mother_off() {
    if $isolated; then
        return 0
    fi
    if mother_running; then
        echo "error: Mother is running; stop it before --mode off" >&2
        echo "hint: run this script as --mode on, then stop Mother and re-run --mode off" >&2
        exit 1
    fi
}

require_mother_on() {
    if ! mother_running; then
        echo "error: Mother is not running; start it before --mode on" >&2
        echo "hint: patina mother start" >&2
        exit 1
    fi
}

run_one() {
    local phase="$1"
    local label="$2"
    local cmd="$3"
    local out
    out=$(mktemp)

    echo "[$phase] $label"
    if "${patina_home_prefix[@]}" bash -lc "$cmd" >"$out" 2>&1; then
        :
    else
        local rc=$?
        echo "error: command failed ($rc): $cmd" >&2
        cat "$out" >&2
        rm -f "$out"
        exit 1
    fi

    if grep -qi "not yet implemented" "$out"; then
        echo "error: placeholder fallback detected for '$label'" >&2
        cat "$out" >&2
        rm -f "$out"
        exit 1
    fi

    if [[ "$phase" == "mother-off" ]]; then
        if grep -Eqi "Mother unavailable|requires Mother|Mother is required" "$out"; then
            echo "error: mother-required behavior detected for core verb '$label' in Mother-off mode" >&2
            cat "$out" >&2
            rm -f "$out"
            exit 1
        fi
    fi

    rm -f "$out"
}

run_phase() {
    local phase="$1"

    # Deterministic fixture-style smoke commands rooted in this repository.
    run_one "$phase" "scrape" "patina scrape layer --no-tmux"
    if [[ "$phase" == "mother-off" ]]; then
        run_one "$phase" "assay-derive (setup)" "patina assay derive --no-tmux"
        run_one "$phase" "scry" "patina scry orient . --no-tmux"
    else
    run_one "$phase" "scry" "patina scry architecture --limit 1 --no-tmux"
    fi
    run_one "$phase" "assay" "patina assay inventory --limit 1 --no-tmux"
    run_one "$phase" "context" "patina context --topic architecture --no-tmux"
    run_one "$phase" "belief" "patina belief audit --no-tmux"
    run_one "$phase" "measure" "patina measure --no-tmux"

    # Oxidize currently has no dedicated dry-run; use command-surface smoke.
    run_one "$phase" "oxidize" "patina oxidize --help"
}

if [[ "$mode" == "off" || "$mode" == "both" ]]; then
    require_mother_off
    run_phase "mother-off"
fi

if [[ "$mode" == "on" || "$mode" == "both" ]]; then
    require_mother_on
    run_phase "mother-on"
fi

echo "ok: core-verb policy smoke checks passed for mode '$mode'"
