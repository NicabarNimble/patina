#!/bin/bash
# Senior Rust smell scan for Patina
#
# This is not a replacement for clippy/tests. It targets repo-specific
# architectural smells that often slip past weak coding agents:
# - runtime regex compilation in scraper hot paths
# - whole-file reads in ingestion/indexing paths
# - fragmented matrix storage in numeric code
# - byte-slicing truncation helpers that can panic on UTF-8
# - benchmark-path cloning/materialization
#
# Default mode prints findings and exits 0.
# Pass --strict to fail on any finding.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"

STRICT=0
if [ "${1:-}" = "--strict" ]; then
    STRICT=1
fi

if ! command -v rg >/dev/null 2>&1; then
    echo "error: ripgrep (rg) is required for senior-smell-checks.sh" >&2
    exit 2
fi

TOTAL_FINDINGS=0

run_check() {
    local title="$1"
    local why="$2"
    local pattern="$3"
    shift 3

    local output
    output="$(rg -n "$pattern" "$@" 2>/dev/null || true)"

    if [ -n "$output" ]; then
        local count
        count="$(printf '%s\n' "$output" | wc -l | tr -d ' ')"
        TOTAL_FINDINGS=$((TOTAL_FINDINGS + count))
        echo ""
        echo "[$title] $count finding(s)"
        echo "Why: $why"
        printf '%s\n' "$output"
    fi
}

echo "Senior Rust smell scan"
echo "Repo: $ROOT_DIR"

run_check \
    "regex-hot-path" \
    "Static regexes in scraper/ingestion paths should usually be compiled once, not per file/function call." \
    'Regex(::new|Builder::new)\(' \
    src/commands/scrape/layer \
    src/commands/scrape/beliefs

run_check \
    "whole-file-read" \
    "Hot ingestion/listing paths should stream with BufRead when possible instead of loading whole files into String." \
    'read_to_string\(' \
    src/commands/persona \
    src/commands/session \
    src/commands/context.rs \
    src/commands/scrape/layer \
    src/commands/scrape/beliefs

run_check \
    "fragmented-matrix" \
    "Vec<Vec<f32>> is usually the wrong storage for dense numeric kernels; prefer contiguous arrays." \
    'Vec<Vec<f32>>' \
    src/commands/oxidize \
    src/commands/eval

run_check \
    "utf8-byte-slice" \
    "Byte slicing string truncation can panic on UTF-8 boundaries and often indicates avoidable string churn." \
    '\&[A-Za-z_][A-Za-z0-9_]*\[\.\.|format!\("\{\}\.\.\.", \&[A-Za-z_][A-Za-z0-9_]*\[\.\.' \
    src/commands

run_check \
    "benchmark-clone" \
    "Benchmark/query-eval paths should avoid cloning/materializing IDs unless ownership is required." \
    'clone\(\)|collect::<Vec<_>>\(\)' \
    src/commands/bench \
    src/commands/eval

run_check \
    "repeat-lowercase" \
    "Repeated to_lowercase in search/routing code is a hint to normalize once and compare borrowed data." \
    'to_lowercase\(' \
    src/commands/assay \
    src/commands/scry \
    src/commands/mother

echo ""
if [ "$TOTAL_FINDINGS" -eq 0 ]; then
    echo "No senior-smell findings."
    exit 0
fi

echo "Total findings: $TOTAL_FINDINGS"
if [ "$STRICT" -eq 1 ]; then
    echo "Strict mode enabled: failing."
    exit 1
fi

echo "Report mode only: exit 0."
